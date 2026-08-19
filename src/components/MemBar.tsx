interface Props {
  usedMb: number;
  totalMb: number;
  pct: number;
}

export function MemBar({ usedMb, totalMb, pct }: Props) {
  const v = Math.max(0, Math.min(100, pct));
  return (
    <section className="card">
      <div className="row-between">
        <h3>Memory</h3>
        <span className="meta">
          {(usedMb / 1024).toFixed(1)} / {(totalMb / 1024).toFixed(1)} GB
        </span>
      </div>
      <div className="track">
        <div className="fill" style={{ width: `${v}%`, background: "#60a5fa" }} />
      </div>
    </section>
  );
}
