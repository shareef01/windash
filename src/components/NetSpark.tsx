interface Props {
  rx: number;
  tx: number;
  stream: number[];
}

function fmt(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB/s`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB/s`;
  return `${bytes} B/s`;
}

export function NetSpark({ rx, tx, stream }: Props) {
  const W = 300;
  const H = 70;
  const n = Math.max(stream.length, 2);
  const max = Math.max(1, ...stream);
  const pts = stream.map((v, i) => {
    const x = (i / (n - 1)) * W;
    const y = H - (v / max) * (H - 8) - 4;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  const line = pts.length > 1 ? `M ${pts.join(" L ")}` : "";
  const area = pts.length > 1 ? `${line} L ${W},${H} L 0,${H} Z` : "";

  return (
    <div className="net-card">
      <div className="net-top">
        <span className="net-label">Network</span>
        <span className="net-speeds">
          <span className="net-dl">▼ {fmt(rx)}</span>
          <span className="net-ul">▲ {fmt(tx)}</span>
        </span>
      </div>
      <svg width="100%" height={H} viewBox={`0 0 ${W} ${H}`} className="net-svg">
        <path d={area} fill="url(#netGrad)" opacity="0.35" />
        <path d={line} fill="none" stroke="var(--accent)" strokeWidth="2" />
        <defs>
          <linearGradient id="netGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--accent)" />
            <stop offset="100%" stopColor="transparent" />
          </linearGradient>
        </defs>
      </svg>
      <div className="net-foot">total throughput · last {n}s</div>
    </div>
  );
}
