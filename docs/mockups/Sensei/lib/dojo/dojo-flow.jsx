// Dōjō · the two WIRED flows the Console presents.
//   · DojoDesktopFlow — one navigable desktop experience: sign in → land on your
//     Relay (You zone) → click a Dōjō in the switcher to step into its management
//     shell (role decided by your membership: admin · maintainer · lead) → back to
//     your work; open Project rules for the effective constitution of a project.
//   · (DojoMobileFlow lives in dojo-relay.jsx, beside its mobile primitives.)
// This replaces the old pile of standalone screen artboards — those screens are
// still defined in lib, but reached by navigating these flows.

const { useState: dfS } = React;

function DojoDesktopFlow({ start = "signin", startDojo = null }) {
  const D = window.DOJO;
  const initMem = startDojo ? D.memberships.find(m => m.id === startDojo) : null;
  const [view, setView] = dfS(initMem ? "dojo" : start);   // signin | you | dojo | project
  const [entered, setEntered] = dfS(initMem || null);   // the membership stepped into
  const you = () => { setView("you"); setEntered(null); };
  const enterDojo = (m) => {
    if (!m || !["admin", "maintainer", "lead"].includes(m.role)) return; // read-only / owner: no console
    setEntered(m); setView("dojo");
  };

  if (view === "signin") {
    return <window.DojoSignIn onContinue={() => setView("you")} />;
  }
  if (view === "dojo" && entered) {
    const common = { onExit: you, enteredOrg: entered };
    if (entered.role === "admin")      return <window.DojoAdminConsole initial="overview" {...common} />;
    if (entered.role === "maintainer") return <window.DojoMaintainerConsole initial="triage" {...common} />;
    if (entered.role === "lead")       return <window.DojoLeadConsole initial="clients" {...common} />;
  }
  if (view === "project") {
    return <window.DojoRulePreview initial="globex" onExit={you} />;
  }
  return (
    <window.DojoDeveloperConsole
      initial="teams" relayStart="projects"
      onEnterDojo={enterDojo}
      onOpenProject={() => setView("project")} />
  );
}

Object.assign(window, { DojoDesktopFlow });
