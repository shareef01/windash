interface Props {
  rx: number;
  tx: number;
  stream: number[];
}

function fmt(bytes: number): string {
  if (bytes > 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB/s`;
  if (bytes > 1024) return `${(bytes / 1024).toFixed(1)} KB/s`;
  return `${bytes} B/s`;
}

export function NetSpark({ rx, tx, stream }: Props) {
  const max = Math.max(1, ...stream);
  const w = 300;
  const h = 48;
  const pts = stream
    .map((val, i) => {
      const x = (i / Math.max(1, stream.length - 1)) * w;
      const y = h - (val / max) * h;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  return (
    <section className="card">
      <div className="row-between">
        <h3>Network</h3>
        <span className="meta">
          ↓ {fmt(rx)} &nbsp; ↑ {fmt(tx)}
        </span>
      </div>
      <svg width="100%" height={h} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none">
        <polyline
          points={pts || `0,${h} ${w},${h}`}
          fill="none"
          stroke="#2dd4bf"
          strokeWidth={2}
        />
      </svg>
    </section>
  );
}
