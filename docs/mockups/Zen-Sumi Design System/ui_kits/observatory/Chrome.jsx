// Observatory · 38px window chrome
// Reused from the source prototype, simplified to pure design-system classes.

function Chrome({ title = "Sensei  先生  ·  observatory" }) {
  return (
    <div className="zs-chrome">
      <div className="zs-traffic"><span/><span/><span/></div>
      <div className="zs-chrome-title">{title}</div>
      <div style={{ width: 54 }}/>
    </div>
  );
}

window.Chrome = Chrome;
