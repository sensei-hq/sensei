// Mounts the three website variants into a DesignCanvas for side-by-side
// comparison. Kept as a separate file (rather than inline in the HTML) so
// it gets the same Babel transform as every other variant module.

const { DesignCanvas, DCSection, DCArtboard,
        VariantA, VariantB, VariantC,
        TweaksPanel, useTweaks, TweakSection, TweakToggle } = window;

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "dark": false
}/*EDITMODE-END*/;

const VariantShell = ({ children }) => (
  <div className="variant-shell">{children}</div>
);

function SiteCanvas() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);

  // The variant artboards are nested inside DesignCanvas's transform-scaled
  // container, so we set data-theme on a wrapper rather than <html> — it has
  // to live where the variant-shell can see it via CSS variable inheritance.
  React.useEffect(() => {
    document.documentElement.setAttribute('data-theme', t.dark ? 'dark' : 'light');
    document.body.style.background = t.dark
      ? 'oklch(0.13 0.010 50)'
      : 'oklch(0.92 0.012 85)';
  }, [t.dark]);

  return (
    <>
      <DesignCanvas
        title="Sensei · Website directions"
        subtitle="Three takes on the marketing site, ranging from monastic to marketing-forward"
        defaultZoom={0.42}
      >
        <DCSection id="overview"
                    title="Same world / Confident continuity / Marketing-forward">
          <DCArtboard id="variant-a"
                       label="A · Same world as the app"
                       width={1280} height={3600}>
            <VariantShell><VariantA/></VariantShell>
          </DCArtboard>
          <DCArtboard id="variant-b"
                       label="B · Confident continuity"
                       width={1280} height={4200}>
            <VariantShell><VariantB/></VariantShell>
          </DCArtboard>
          <DCArtboard id="variant-c"
                       label="C · Marketing-forward"
                       width={1280} height={4400}>
            <VariantShell><VariantC/></VariantShell>
          </DCArtboard>
        </DCSection>
      </DesignCanvas>

      <TweaksPanel title="Tweaks">
        <TweakSection label="Theme"/>
        <TweakToggle label="Dark mode"
                     value={t.dark}
                     onChange={(v) => setTweak('dark', v)}/>
      </TweaksPanel>
    </>
  );
}

ReactDOM.createRoot(document.getElementById('root'))
        .render(<SiteCanvas/>);
