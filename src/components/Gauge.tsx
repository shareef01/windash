interface Props {
  label: string;
  value: number; // 0..100
  sub?: string;
}

function color(v: number): string {
  return v >= 85 ? "var(--red)" : v >= 60 ? "var(--amber)" : "var(--accent)";
}

// Compact stat tile: label + big value + progress bar. No oversized ring.
export function Gauge({ label, value, sub }: Props) {
  const v = Math.max(0, Math.min(100, value));
  return (
    <div className="stat">
      <div className="stat-top">
        <span className="stat-label">{label}</span>
        <span className="stat-val" style={{ color: color(v) }}>
          {v.toFixed(0)}
          <span className="stat-unit">%</span>
        </span>
      </div>
      <div className="track">
        <div
          className="fill"
          style={{ width: `${v}%`, background: color(v) }}
        />
      </div>
      {sub && <div className="stat-sub">{sub}</div>}
    </div>
  );
}
