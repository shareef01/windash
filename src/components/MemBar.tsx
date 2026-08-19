interface Props {
  usedMb: number;
  totalMb: number;
  pct: number;
}

function color(v: number): string {
  return v >= 85 ? "var(--red)" : v >= 60 ? "var(--amber)" : "var(--accent)";
}

export function MemBar({ usedMb, totalMb, pct }: Props) {
  const usedGb = (usedMb / 1024).toFixed(1);
  const totalGb = (totalMb / 1024).toFixed(1);
  return (
    <div className="stat">
      <div className="stat-top">
        <span className="stat-label">Memory</span>
        <span className="stat-val">
          {usedGb}
          <span className="stat-unit"> / {totalGb} GB</span>
        </span>
      </div>
      <div className="track">
        <div
          className="fill"
          style={{ width: `${Math.max(0, Math.min(100, pct))}%`, background: color(pct) }}
        />
      </div>
      <div className="stat-sub">{pct.toFixed(0)}% utilized</div>
    </div>
  );
}
