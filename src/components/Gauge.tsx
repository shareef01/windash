import { usageColor } from "../theme";

interface Props {
  label: string;
  value: number; // 0..100
  sub?: string;
}

// Compact stat tile: label + big value + progress bar. No oversized ring.
export function Gauge({ label, value, sub }: Props) {
  const v = Math.max(0, Math.min(100, value));
  const c = usageColor(v);
  return (
    <div className="stat">
      <div className="stat-top">
        <span className="stat-label">{label}</span>
        <span className="stat-val" style={{ color: c }}>
          {v.toFixed(0)}
          <span className="stat-unit">%</span>
        </span>
      </div>
      <div className="track">
        <div className="fill" style={{ width: `${v}%`, background: c }} />
      </div>
      {sub && <div className="stat-sub">{sub}</div>}
    </div>
  );
}
