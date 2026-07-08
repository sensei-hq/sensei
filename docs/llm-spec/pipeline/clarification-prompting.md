# 問 · Pipeline · Clarification prompting (stub)

**Status:** stub — deferred to v2.

## Purpose

When the assistant doesn't have enough information to answer
confidently, it should ask a clarifying question **with a reason**
— not "can you clarify?" but "I need to know whether we're
keeping backward compatibility here; that changes the answer
materially. Without it, half my choices are guesses." This
pipeline is the mechanism.

Kanji is 問 — *to ask*.

## Not yet designed

The full design lives with the mutual-pair vision in the
README (§ vision paragraph). Behaviourally, the assistant's
plugin surface enforces a "ask-with-reason" contract at
specific inflection points; the daemon exposes the resulting
questions and answers as a learning signal for
[[pipeline/memory]] and [[pipeline/insights]].

## Placeholder invariants

- `sensei.clarifications` — per session, list of
  `{ question, reason, answered?, answer, followed_up }`.
- Feeds an insight when a user consistently under-specifies a
  particular concern (feature request without acceptance
  criteria, schema change without compatibility policy, etc.).

## Related

- [[pipeline/memory]] — the assistant clarification-question is a
  future user-side memory candidate
- [[pipeline/insights]] — repeated clarifications on the same
  concern become a recommendation ("you tend to under-specify
  X — a template would help")
- README §vision — the pair-goes-both-ways framing
