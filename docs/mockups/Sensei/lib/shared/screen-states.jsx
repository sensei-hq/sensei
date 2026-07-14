// Shared empty / loading / error scaffolds for Observatory & project screens.
// One component, three states — a screen renders it instead of its happy path
// when there's no data yet, a fetch is in flight, or a fetch failed.
//
// Usage inside any screen:
//   if (state !== "ready") return <ScreenState state={state}
//       kanji="庫" emptyTitle="No libraries yet"
//       emptyHint="Run a session that uses a dependency and it'll appear here."
//       onRetry={() => {}} />;
//
// state: "ready" (render nothing here) | "loading" | "empty" | "error".
// Token-only → theme-free. Uses the global .kanji / .mono / .display classes.

function ScreenState({
  state,
  kanji = "空",
  emptyTitle = "Nothing here yet",
  emptyHint = "This fills in as sensei watches your work.",
  errorTitle = "Couldn't load this",
  errorHint = "The fetch failed. Check your connection and try again.",
  loadingLabel = "Still listening.",
  onRetry,
}) {
  const Wrap = ({ children }) => (
    <div style={{ width: "100%", height: "100%", minHeight: 280, display: "flex", flexDirection: "column",
      alignItems: "center", justifyContent: "center", gap: 14, background: "var(--paper)",
      padding: "48px 32px", textAlign: "center" }}>
      <style>{`@keyframes ssPulse{0%,100%{opacity:.35}50%{opacity:.9}}@keyframes ssSpin{to{transform:rotate(360deg)}}`}</style>
      {children}
    </div>
  );

  if (state === "loading") {
    return (
      <Wrap>
        <span style={{ width: 30, height: 30, borderRadius: "50%", border: "2.5px solid var(--paper-3)",
          borderTopColor: "var(--accent)", animation: "ssSpin 0.8s linear infinite" }} />
        <div style={{ display: "flex", flexDirection: "column", gap: 9, alignItems: "center", marginTop: 4 }}>
          {[220, 320, 260].map((w, i) => (
            <span key={i} style={{ width: w, maxWidth: "60vw", height: 9, borderRadius: 5, background: "var(--paper-3)",
              animation: "ssPulse 1.4s ease-in-out infinite", animationDelay: i * 0.18 + "s" }} />
          ))}
        </div>
        <div style={{ fontSize: 13, color: "var(--ink-3)", marginTop: 6, fontStyle: "italic" }}>{loadingLabel}</div>
      </Wrap>
    );
  }

  if (state === "error") {
    return (
      <Wrap>
        <span className="kanji" style={{ fontSize: 52, color: "var(--warning)", opacity: 0.9, lineHeight: 1 }}>誤</span>
        <div className="display" style={{ fontSize: 20, fontWeight: 400, color: "var(--ink)", letterSpacing: "-0.01em" }}>{errorTitle}</div>
        <div style={{ fontSize: 13.5, color: "var(--ink-2)", lineHeight: 1.55, maxWidth: 380 }}>{errorHint}</div>
        <button onClick={onRetry} style={{ marginTop: 8, display: "inline-flex", alignItems: "center", gap: 8,
          padding: "9px 16px", borderRadius: 8, border: "none", cursor: "pointer",
          background: "var(--ink)", color: "var(--paper)", fontSize: 13, fontWeight: 500, fontFamily: "inherit" }}>
          <span className="kanji" style={{ fontSize: 13, color: "var(--accent)" }}>再</span>{"Retry"}
        </button>
      </Wrap>
    );
  }

  // empty
  return (
    <Wrap>
      <span className="kanji" style={{ fontSize: 56, color: "var(--ink-4)", lineHeight: 1 }}>{kanji}</span>
      <div className="display" style={{ fontSize: 20, fontWeight: 400, color: "var(--ink)", letterSpacing: "-0.01em" }}>{emptyTitle}</div>
      <div style={{ fontSize: 13.5, color: "var(--ink-2)", lineHeight: 1.6, maxWidth: 420 }}>{emptyHint}</div>
    </Wrap>
  );
}

window.ScreenState = ScreenState;
