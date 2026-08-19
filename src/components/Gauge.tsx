interface Props {
  label: string;
  value: number; // 0..100
  sub?: string;
}

// Circular ring gauge rendered as an SVG stroke-dashoffset arc.
export function Gauge({ label, value, sub }: Props) {
  const v = Math.max(0, Math.min(100, value));
  const size = 132;
  const stroke = 10;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const offset = c * (1 - v / 100);
  const color =
    v >= 85 ? "var(--danger)" : v >= 60 ? "var(--warn)" : "var(--accent)";

  return (
    <div className="ring-card">
      <div className="ring-wrap">
        <svg width={size} height={size} className="ring-svg">
          <circle
            cx={size / 2}
            cy={size / 2}
            r={r}
            stroke="var(--panel)"
            strokeWidth={stroke}
            fill="none"
          />
          <circle
            cx={size / 2}
            cy={size / 2}
            r={r}
            stroke={color}
            strokeWidth={stroke}
            fill="none"
            strokeLinecap="round"
            strokeDasharray={c}
            strokeDashoffset={offset}
            transform={`rotate(-90 ${size / 2} ${size / 2})`}
            style={{ transition: "stroke-dashoffset 0.5s ease, stroke 0.3s ease" }}
          />
        </svg>
        <div className="ring-center">
          <span className="ring-val">{v.toFixed(0)}</span>
          <span className="ring-unit">%</span>
        </div>
      </div>
      <div className="ring-meta">
        <span className="ring-label">{label}</span>
        {sub && <span className="ring-sub">{sub}</span>}
      </div>
    </div>
  );
}
