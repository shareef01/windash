import { fmtRate } from "../theme";

interface Props {
  rx: number; // bytes/sec
  tx: number; // bytes/sec
  stream: number[]; // download-rate history (bytes/sec)
}

export function NetSpark({ rx, tx, stream }: Props) {
  const W = 320;
  const H = 48;
  const n = Math.max(stream.length, 2);
  const max = Math.max(1, ...stream);
  const pts = stream.map((v, i) => {
    const x = (i / (n - 1)) * W;
    const y = H - (v / max) * (H - 6) - 3;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  const line = pts.length > 1 ? `M ${pts.join(" L ")}` : "";
  const area = pts.length > 1 ? `${line} L ${W},${H} L 0,${H} Z` : "";

  return (
    <section className="section net">
      <div className="section-head">
        <span className="section-title">Network</span>
        <span className="net-speeds">
          <span className="net-dl">▼ {fmtRate(rx)}</span>
          <span className="net-ul">▲ {fmtRate(tx)}</span>
        </span>
      </div>
      <svg width="100%" height={H} viewBox={`0 0 ${W} ${H}`} className="net-svg" preserveAspectRatio="none">
        <path d={area} fill="url(#netGrad)" opacity="0.28" />
        <path d={line} fill="none" stroke="var(--accent)" strokeWidth="1.5" strokeLinejoin="round" />
        <defs>
          <linearGradient id="netGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--accent)" />
            <stop offset="100%" stopColor="transparent" />
          </linearGradient>
        </defs>
      </svg>
    </section>
  );
}
