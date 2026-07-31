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
    <div className="w-full h-full flex flex-col items-center justify-center bg-paper text-center" style={{ minHeight: 280, gap: 14,
 padding: "48px 32px" }}>
      <style>{`@keyframes ssPulse{0%,100%{opacity:.35}50%{opacity:.9}}@keyframes ssSpin{to{transform:rotate(360deg)}}`}</style>
      {children}
    </div>
  );

  if (state === "loading") {
    return (
      <Wrap>
        <span className="rounded-full" style={{ width: 30, height: 30, border: "2.5px solid var(--paper-3)",
 borderTopColor: "var(--accent)", animation: "ssSpin 0.8s linear infinite" }} />
        <div className="flex flex-col items-center" style={{ gap: 9, marginTop: 4 }}>
          {[220, 320, 260].map((w, i) => (
            <span className="bg-paper-3" key={i} style={{ width: w, maxWidth: "60vw", height: 9, borderRadius: 5,
 animation: "ssPulse 1.4s ease-in-out infinite", animationDelay: i * 0.18 + "s" }} />
          ))}
        </div>
        <div className="text-ink-3 italic" style={{ fontSize: 13, marginTop: 6 }}>{loadingLabel}</div>
      </Wrap>
    );
  }

  if (state === "error") {
    return (
      <Wrap>
        <span className="kanji text-warning" style={{ fontSize: 52, opacity: 0.9, lineHeight: 1 }}>誤</span>
        <div className="display font-normal text-ink" style={{ fontSize: 20, letterSpacing: "-0.01em" }}>{errorTitle}</div>
        <div className="text-ink-2" style={{ fontSize: 13.5, lineHeight: 1.55, maxWidth: 380 }}>{errorHint}</div>
        <button className="inline-flex items-center border-0 cursor-pointer bg-ink text-paper font-medium" onClick={onRetry} style={{ marginTop: 8, gap: 8,
 padding: "12px 16px", borderRadius: 8, fontSize: 13, fontFamily: "inherit" }}>
          <span className="kanji text-accent" style={{ fontSize: 13 }}>再</span>{"Retry"}
        </button>
      </Wrap>
    );
  }

  // empty
  return (
    <Wrap>
      <span className="kanji text-ink-4" style={{ fontSize: 56, lineHeight: 1 }}>{kanji}</span>
      <div className="display font-normal text-ink" style={{ fontSize: 20, letterSpacing: "-0.01em" }}>{emptyTitle}</div>
      <div className="text-ink-2" style={{ fontSize: 13.5, lineHeight: 1.6, maxWidth: 420 }}>{emptyHint}</div>
    </Wrap>
  );
}

window.ScreenState = ScreenState;
