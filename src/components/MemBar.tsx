interface Props {
  usedMb: number;
  totalMb: number;
  pct: number;
}

export function MemBar({ usedMb, totalMb, pct }: Props) {
  const usedGb = (usedMb / 1024).toFixed(1);
  const totalGb = (totalMb / 1024).toFixed(1);
  const color =
    pct >= 85 ? "var(--danger)" : pct >= 60 ? "var(--warn)" : "var(--accent)";

  return (
    <div className="bar-card">
      <div className="bar-top">
        <span className="bar-label">Memory</span>
        <span className="bar-readout">
          {usedGb}
          <span className="bar-dim"> / {totalGb} GB</span>
        </span>
      </div>
      <div className="track">
        <div
          className="fill"
          style={{
            width: `${Math.max(0, Math.min(100, pct))}%`,
            background: `linear-gradient(90deg, ${color}, ${color}cc)`,
          }}
        />
      </div>
      <div className="bar-foot">{pct.toFixed(0)}% utilized</div>
    </div>
  );
}
