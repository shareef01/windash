interface Props {
  label: string;
  value: number; // 0..100
  unit?: string;
}

export function Gauge({ label, value, unit = "%" }: Props) {
  const v = Math.max(0, Math.min(100, value));
  const color = v > 80 ? "#f87171" : v > 50 ? "#fbbf24" : "#2dd4bf";
  return (
    <section className="card">
      <div className="row-between">
        <h3>{label}</h3>
        <span className="big" style={{ color }}>
          {v.toFixed(1)}
          {unit}
        </span>
      </div>
      <div className="track">
        <div className="fill" style={{ width: `${v}%`, background: color }} />
      </div>
    </section>
  );
}
