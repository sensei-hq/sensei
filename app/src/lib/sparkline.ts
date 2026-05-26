export function sparklinePath(
  points: Array<{ ftr_rate: number }>,
  w: number,
  h: number,
): string {
  if (points.length < 2) return '';
  const vals = points.map((p) => p.ftr_rate);
  const min = Math.min(...vals);
  const max = Math.max(...vals);
  const range = max - min || 0.01;
  const step = w / (vals.length - 1);
  return vals
    .map((v, i) => {
      const x = i * step;
      const y = h - (h - 2) * ((v - min) / range) - 1;
      return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)} ${y.toFixed(1)}`;
    })
    .join(' ');
}
