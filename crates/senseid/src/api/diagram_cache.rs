//! The diagram payload cache (#233).
//!
//! ## What is expensive, and what is not
//!
//! Every diagram in the set — Structure, Layers, Cycles, Zones — is one read of
//! `sensei.structure_edges` for one project, plus an aggregation that is free by
//! comparison. Measured 2026-10-06 against the installed release daemon:
//! project `sensei` at module grain takes 37 s, and the layering itself is
//! single-digit milliseconds of that. `sensei.edges` is 3.3M rows and 4.2 GB,
//! and the read touches ~300,000 buffers. It is I/O, not arithmetic.
//!
//! ## Why a cache rather than a materialized view, FOR NOW
//!
//! The screens made the access pattern visible, which is what #233 said to wait
//! for. It is INTERACTIVE: grain and edge kinds are controls, so a user changes
//! the question several times a minute, and Layers and Cycles read the SAME
//! payload — so a visit to both paid 37 s twice. Precomputation fixes the FIRST
//! paint; a cache fixes every request after it, which is where the interaction
//! lives.
//!
//! It is also forward-only. Any precomputed table needs exactly the version
//! marker this introduces in order to know when it is stale, so nothing here is
//! thrown away by building one later.
//!
//! ## The version is part of the key, so nothing here can go stale
//!
//! An entry is returned only when the stored version EQUALS the caller's. There
//! is no TTL and no invalidation pass, because both are ways of being wrong for
//! a while: a TTL serves data known to be old, and an invalidation pass can miss
//! a writer. A version that does not match is a miss, full stop.
//!
//! The daemon supplies the version — the latest `files.indexed_at` across the
//! project's folders, which moves whenever anything that feeds these payloads is
//! re-indexed and costs 93 ms against a 37 s computation.
//!
//! ## `get`/`put` and not `get_or_compute`
//!
//! The obvious API takes a closure, and it cannot be written here: the
//! computation is `async` and fallible, and a `std::sync::Mutex` guard may not
//! be held across an `await`. Splitting it keeps every lock acquisition
//! non-async and short. Two concurrent misses on one key therefore both compute
//! — which is wasted work and not a wrong answer, and the alternative is holding
//! a lock for 37 s.

use std::collections::HashMap;
use std::sync::Mutex;

/// How many diagram payloads the daemon holds.
///
/// Sized for the way the screens are used rather than for the key space, which
/// is unbounded: three grains times thirty-one edge-kind combinations times
/// every project. What a person actually does is open two or three projects and
/// flip a handful of questions on each, and 64 covers that with room over. A
/// payload for a large project is hundreds of kilobytes of JSON, so this is tens
/// of megabytes at worst and reproducible in full from the database.
pub const DIAGRAM_CACHE_ENTRIES: usize = 64;

/// The question a cached payload answers.
///
/// `params` is the request's own parameters already rendered to a string —
/// `"module|calls"` — because the cache must not know what any diagram's
/// parameters MEAN. A key built from parsed values would need a variant per
/// diagram and would quietly collide the day two diagrams took different ones.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DiagramKey {
    /// Which endpoint. A `&'static str` because the set is closed and named at
    /// the call site; a `String` would invite building one from user input.
    pub diagram: &'static str,
    pub project: uuid::Uuid,
    pub params: String,
}

impl DiagramKey {
    pub fn new(diagram: &'static str, project: uuid::Uuid, params: impl Into<String>) -> Self {
        Self { diagram, project, params: params.into() }
    }
}

/// When the project's graph last changed.
///
/// `None` is a real value and not an absence: a project whose folders hold no
/// indexed file has no version, and two such states are the same state. It
/// compares equal to itself, so an empty project caches like any other.
pub type GraphVersion = Option<chrono::DateTime<chrono::Utc>>;

struct Entry {
    version: GraphVersion,
    payload: serde_json::Value,
    /// Monotonic tick of the last read or write, for eviction. A counter rather
    /// than a clock: `Instant::now()` inside the lock would make eviction order
    /// depend on timer resolution under a burst, and the only question here is
    /// relative order.
    used: u64,
}

/// Payloads already computed, keyed by question and by graph version.
pub struct DiagramCache {
    entries: Mutex<HashMap<DiagramKey, Entry>>,
    /// How many payloads to hold. BOUNDED because the key space is not: three
    /// grains times 31 edge-kind combinations times every project is thousands
    /// of entries, and a layering payload for a large project is hundreds of
    /// kilobytes of JSON.
    capacity: usize,
    tick: Mutex<u64>,
}

impl DiagramCache {
    pub fn new(capacity: usize) -> Self {
        Self { entries: Mutex::new(HashMap::new()), capacity: capacity.max(1), tick: Mutex::new(0) }
    }

    fn next_tick(&self) -> u64 {
        let mut tick = self.tick.lock().expect("diagram cache tick");
        *tick += 1;
        *tick
    }

    /// The payload for this question, if one was stored against THIS version.
    ///
    /// A version mismatch returns `None` and leaves the stale entry in place:
    /// the caller is about to `put` over it, and evicting here would drop a row
    /// that a concurrent request on the old version could still have used.
    pub fn get(&self, key: &DiagramKey, version: GraphVersion) -> Option<serde_json::Value> {
        let used = self.next_tick();
        let mut entries = self.entries.lock().expect("diagram cache");
        let entry = entries.get_mut(key)?;
        if entry.version != version {
            return None;
        }
        entry.used = used;
        Some(entry.payload.clone())
    }

    pub fn put(&self, key: DiagramKey, version: GraphVersion, payload: serde_json::Value) {
        let used = self.next_tick();
        let mut entries = self.entries.lock().expect("diagram cache");
        entries.insert(key, Entry { version, payload, used });
        // Evict AFTER inserting, so a full cache never refuses the newest
        // answer in order to keep an older one.
        while entries.len() > self.capacity {
            let Some(oldest) = entries.iter().min_by_key(|(_, e)| e.used).map(|(k, _)| k.clone())
            else {
                break;
            };
            entries.remove(&oldest);
        }
    }

    /// How many payloads are held.
    ///
    /// TEST-ONLY, and that is the honest scope: nothing in the daemon reads the
    /// size. The bound is the reason it exists — an unbounded cache of
    /// hundred-kilobyte payloads over a key space of grains times kind
    /// combinations times projects is a daemon that grows until it is
    /// restarted — and the only way to assert a bound is to be able to see it.
    ///
    /// If an operator ever needs this on the health surface, make it `pub` then
    /// and give it a reader. Exporting it now on the grounds that somebody might
    /// is how a method comes to look load-bearing without being called.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.lock().expect("diagram cache").len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_project() -> uuid::Uuid {
        uuid::Uuid::nil()
    }

    fn at(secs: i64) -> GraphVersion {
        chrono::DateTime::from_timestamp(secs, 0)
    }

    fn payload(tag: &str) -> serde_json::Value {
        serde_json::json!({ "tag": tag })
    }

    /// A HIT NEEDS BOTH THE KEY AND THE VERSION.
    ///
    /// The property the whole design rests on: there is no TTL and no
    /// invalidation pass, so the only thing standing between a reader and a
    /// stale 37-second payload is this comparison. A cache that matched on the
    /// key alone would serve the graph as it was before the last index, which is
    /// worse than being slow — the Layers screen's arrows are the ones somebody
    /// is about to go and cut.
    ///
    /// Mutation that must break this test: drop the `entry.version != version`
    /// guard.
    #[test]
    fn a_changed_version_is_a_miss_and_never_a_stale_hit() {
        let cache = DiagramCache::new(8);
        let key = DiagramKey::new("layering", a_project(), "module|calls");

        cache.put(key.clone(), at(100), payload("old"));
        assert_eq!(cache.get(&key, at(100)), Some(payload("old")), "same version, same answer");
        assert_eq!(cache.get(&key, at(200)), None, "the graph moved, so the answer is unknown");

        cache.put(key.clone(), at(200), payload("new"));
        assert_eq!(cache.get(&key, at(200)), Some(payload("new")));
        assert_eq!(cache.len(), 1, "a re-put on one key replaces rather than accumulates");
    }

    /// The parameters are part of the question.
    ///
    /// Mutation that must break this test: drop `params` from `DiagramKey`.
    #[test]
    fn a_different_question_is_a_different_entry() {
        let cache = DiagramCache::new(8);
        let module = DiagramKey::new("layering", a_project(), "module|calls");
        let file = DiagramKey::new("layering", a_project(), "file|calls");
        let kinds = DiagramKey::new("layering", a_project(), "module|calls,imports");
        let other = DiagramKey::new("structure", a_project(), "module|calls");

        cache.put(module.clone(), at(1), payload("module"));
        cache.put(file.clone(), at(1), payload("file"));
        cache.put(kinds.clone(), at(1), payload("kinds"));
        cache.put(other.clone(), at(1), payload("structure"));

        assert_eq!(cache.get(&module, at(1)), Some(payload("module")));
        assert_eq!(cache.get(&file, at(1)), Some(payload("file")));
        assert_eq!(cache.get(&kinds, at(1)), Some(payload("kinds")));
        assert_eq!(
            cache.get(&other, at(1)),
            Some(payload("structure")),
            "the diagram is keyed too"
        );
        assert_eq!(cache.len(), 4);
    }

    /// A project with nothing indexed has a version, and it is `None`.
    ///
    /// Not a special case to skip: an empty project is a legitimate, cacheable
    /// answer, and treating `None` as "never cache" would make the cheapest
    /// payload the only uncached one.
    #[test]
    fn an_unindexed_project_caches_like_any_other() {
        let cache = DiagramCache::new(8);
        let key = DiagramKey::new("layering", a_project(), "module|calls");
        cache.put(key.clone(), None, payload("empty"));
        assert_eq!(cache.get(&key, None), Some(payload("empty")));
        assert_eq!(cache.get(&key, at(1)), None, "and it still misses once something is indexed");
    }

    /// THE BOUND HOLDS, and the thing dropped is the least recently used.
    ///
    /// The key space is three grains times 31 kind combinations times every
    /// project, and one payload is hundreds of kilobytes. Unbounded, this is a
    /// daemon that grows until it is restarted.
    ///
    /// Mutation that must break this test: remove the eviction loop, or evict
    /// the most recently used instead of the least.
    #[test]
    fn the_cache_is_bounded_and_drops_the_least_recently_used() {
        let cache = DiagramCache::new(2);
        let a = DiagramKey::new("layering", a_project(), "a");
        let b = DiagramKey::new("layering", a_project(), "b");
        let c = DiagramKey::new("layering", a_project(), "c");

        cache.put(a.clone(), at(1), payload("a"));
        cache.put(b.clone(), at(1), payload("b"));
        // READING `a` makes it the newer of the two, so `b` is what goes.
        assert_eq!(cache.get(&a, at(1)), Some(payload("a")));

        cache.put(c.clone(), at(1), payload("c"));
        assert_eq!(cache.len(), 2, "the bound holds");
        assert_eq!(cache.get(&b, at(1)), None, "the least recently used went");
        assert_eq!(cache.get(&a, at(1)), Some(payload("a")), "the one that was read stayed");
        assert_eq!(cache.get(&c, at(1)), Some(payload("c")), "and so did the newest");
    }

    /// A capacity of zero would make every `put` evict what it just stored, so
    /// the cache would be a slow no-op that still clones every payload.
    #[test]
    fn a_zero_capacity_still_holds_one_answer() {
        let cache = DiagramCache::new(0);
        let key = DiagramKey::new("layering", a_project(), "module|calls");
        cache.put(key.clone(), at(1), payload("one"));
        assert_eq!(cache.get(&key, at(1)), Some(payload("one")));
    }
}
