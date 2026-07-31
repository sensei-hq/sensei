// ═══════════════════════════════════════════════════════════════════
// Dōjō · data access — the seam between screens and the backend
// ═══════════════════════════════════════════════════════════════════
// Screens must never touch window.DOJO2 directly. They call these
// accessors, which today read the fixture module and tomorrow read real
// endpoints. Swapping is a one-file change: replace each body with a
// fetch and delete the fixture import. Nothing in the UI moves.
//
// Every accessor is ALREADY async and already returns the exact shape a
// row or a detail pane needs, so making it real can't change a call
// site's signature. That's the point — an accessor that returns the raw
// fixture would just move the coupling.
//
// The endpoint each one maps to is named in its comment. Where the mock
// derives something the server should compute (inbox ordering, progress
// rollups), the comment says so, so the API contract is explicit rather
// than reverse-engineered from JSX later.

(function () {
  var D = function () { return window.DOJO2 || {}; };

  // The fixture is synchronous, but the accessors are not: resolving on a
  // microtask means a screen that forgets to handle "not loaded yet" is
  // caught here rather than in production.
  function ok(value) { return Promise.resolve(value); }
  function fail(message) { return Promise.reject(new Error(message)); }

  var api = {
    // ─── identity ────────────────────────────────────────────────────
    // GET /me
    getMe: function () { return ok(D().me); },

    // GET /me/dojos — memberships, with the role the server derived
    // from git. Roles are never client-computed.
    getMyDojos: function () { return ok(D().dojos || []); },

    getDojo: function (slug) {
      var d = (D().dojos || []).filter(function (x) { return x.slug === slug; })[0];
      return d ? ok(d) : fail('no dōjō ' + slug);
    },

    // ─── inbox · in-flight sessions ──────────────────────────────────
    // GET /you/inbox
    // SERVER CONTRACT: the ordering (needs-you → stalled/blocked/failed →
    // running → terminal) and `progress` belong to the server, not the
    // client — a paginated inbox can't be sorted in the browser. The mock
    // sorts in k2InboxRows() only because the fixture is one page.
    getInbox: function () { return ok(D().runs || []); },

    // GET /you/runs/:id
    getRun: function (runId) {
      var r = (D().runs || []).filter(function (x) { return x.id === runId; })[0];
      return r ? ok(r) : fail('no run ' + runId);
    },

    // GET /you/runs/:id/asks — what a session can't decide alone.
    // Called BOTH ways on purpose: with a run id for one session's asks,
    // and bare for every open ask (the inbox needs one per row, and the
    // nav badge needs a global count — neither can afford N calls).
    // Ordered by the server: blocking before advisory, oldest first.
    getAsks: function (runId) {
      var all = D().asks || [];
      return ok(runId ? all.filter(function (a) { return a.run === runId; }) : all);
    },

    // GET /you/asks?pending=1 — count only, for the nav badge.
    getPendingAskCount: function () {
      return ok((D().asks || []).length);
    },

    // POST /you/asks/:id/answer  { choice? , text? }
    // Returns the resumed run so the caller can re-render from the
    // response instead of re-fetching.
    answerAsk: function (askId, answer) {
      if (!askId) return fail('answerAsk needs an ask id');
      var ask = (D().asks || []).filter(function (a) { return a.id === askId; })[0];
      if (!ask) return fail('no ask ' + askId);
      return ok({
        askId: askId,
        runId: ask.run,
        resumedFrom: ask.task,
        recordedAs: answer && answer.choice ? 'Answered · ' + answer.choice
          : 'Answered · “' + ((answer && answer.text) || '') + '”',
      });
    },

    // POST /you/runs/:id/pause
    pauseRun: function (runId) { return ok({ runId: runId, state: 'paused' }); },

    // ─── projects ────────────────────────────────────────────────────
    // GET /you/projects · GET /:slug/projects
    getProjects: function (orgSlug) {
      if (!orgSlug) return ok(D().projects || []);
      var byOrg = D().orgProjects || {};
      return ok((byOrg[orgSlug] || byOrg.acme || []).map(function (p) {
        return Object.assign({}, p, {
          repo: orgSlug + '/' + p.name,
          note: p.team,
          lastRun: p.runsWeek + '/wk',
        });
      }));
    },

    // ─── governance ──────────────────────────────────────────────────
    // GET /you/constitution — the rungs, in precedence order.
    // Same source as getLadder; kept as the name the endpoint will have.
    getConstitution: function () { return ok(D().ladder || []); },
    // GET /you/packs
    getRulePacks: function () { return ok(D().rulePacks || []); },
    // GET /:slug/constitution
    getOrgLadder: function (slug) { return ok((D().ladders || {})[slug] || D().ladder || []); },

    // ─── teams · the collective view ─────────────────────────────────
    // GET /:slug/teams — metrics are server-aggregated; the client must
    // not compute a first-try rate from raw sessions.
    getTeams: function (orgSlug) {
      var t = D().teams || {};
      return ok(t[orgSlug] || t.acme || []);
    },

    getTeam: function (orgSlug, teamId) {
      return api.getTeams(orgSlug).then(function (teams) {
        var team = teams.filter(function (x) { return x.id === teamId; })[0];
        return team || Promise.reject(new Error('no team ' + teamId));
      });
    },

    // GET /:slug/teams/:id/members/:name
    getTeamMember: function (orgSlug, teamId, name) {
      return api.getTeam(orgSlug, teamId).then(function (team) {
        var p = (team.people || []).filter(function (x) { return x.name === name; })[0];
        return p ? Object.assign({}, p, { team: team.name }) : Promise.reject(new Error('no member ' + name));
      });
    },

    // GET /:slug/teams/inflow — what local sensei installs sent up
    getTeamInflow: function (teamName) {
      var f = D().teamInflow || [];
      return ok(teamName ? f.filter(function (x) { return x.team === teamName; }) : f);
    },

    // GET /me/sharing — the switches on THIS developer's machine. Server
    // reports them; it cannot change them. Sharing is local-first.
    getSharingSettings: function () { return ok(D().teamSharing || []); },

    // ─── org consoles ────────────────────────────────────────────────
    getMembers: function () { return ok(D().members || []); },
    getRoles: function () { return ok(D().roles || {}); },
    getConsole: function (name) { return ok((D().consoles || {})[name]); },
    // Billing lives at the top level of the fixture, not under consoles.
    getBilling: function () { return ok(D().billing || (D().consoles || {}).billing); },

    // ─── personal governance ─────────────────────────────────────────
    // GET /you/constitution/stance — the three autonomy dials
    getStance: function () { return ok(D().stance || []); },
    // GET /you/ladder — every rung, in precedence order
    getLadder: function () { return ok(D().ladder || []); },
    // GET /you/conflicts — rule clashes awaiting a settle
    getConflicts: function () { return ok(D().conflicts || []); },
    // GET /you/contributions
    getContributions: function () { return ok(D().contributions || {}); },

    // ─── org governance ──────────────────────────────────────────────
    // GET /:slug/constitution — the dōjō's OWN sections, not the ladder
    getOrgConstitution: function (slug) {
      var c = D().orgConstitution || {};
      return ok(c[slug] || c.acme || []);
    },
    // GET /:slug/scopes — who owns/triages each scope queue
    getScopeOwners: function (slug) {
      var s = D().scopeOwners || {};
      return ok(s[slug] || s.acme || []);
    },
    // GET /:slug/needs — the org-side needs band (distinct from your asks)
    getOrgNeeds: function () { return ok(D().needsYou || []); },

    // ─── conversation ────────────────────────────────────────────────
    // GET /you/runs/:id/thread
    getThread: function (runId) {
      var c = D().chat || {};
      return ok(c.session === runId ? c.thread : []);
    },
    // POST /you/runs/:id/thread
    postReply: function (runId, text) { return ok({ runId: runId, who: 'me', text: text }); },
  };

  // ─── how to make this real ─────────────────────────────────────────
  // Set ZS_API.transport to a fetch wrapper and each accessor above
  // becomes one line. Kept here rather than in a doc so the swap is
  // visible from the file you'd edit:
  //
  //   ZS_API.transport = (path, init) =>
  //     fetch('/api' + path, init).then(r => {
  //       if (!r.ok) throw new Error(path + ' → ' + r.status);
  //       return r.json();
  //     });
  //
  //   getInbox: () => ZS_API.transport('/you/inbox'),
  //   answerAsk: (id, answer) => ZS_API.transport('/you/asks/' + id + '/answer',
  //     { method: 'POST', headers: { 'content-type': 'application/json' },
  //       body: JSON.stringify(answer) }),
  api.transport = null;

  if (typeof window !== 'undefined') window.ZS_API = api;
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
})();
