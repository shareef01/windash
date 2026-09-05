import { fmtRate } from "../theme";
import { IconNetwork } from "./icons";
import type { NetSample } from "../types";

interface Props {
  rx: number | null;
  tx: number | null;
  stream: NetSample[];
  loading?: boolean;
}

function polyline(values: number[], width: number, height: number, max: number): string {
  const n = Math.max(values.length, 2);
  return values
    .map((v, i) => {
      const x = (i / (n - 1)) * width;
      const y = height - (v / max) * (height - 6) - 3;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

export function NetSpark({ rx, tx, stream, loading }: Props) {
  const W = 320;
  const H = 44;
  const rxHist = stream.map((s) => s.rx);
  const txHist = stream.map((s) => s.tx);
  const max = Math.max(1, ...rxHist, ...txHist);
  const rxPts = rxHist.length ? polyline(rxHist, W, H, max) : "";
  const txPts = txHist.length ? polyline(txHist, W, H, max) : "";
  const quiet = !loading && stream.length > 0 && max <= 1 && (rx ?? 0) === 0 && (tx ?? 0) === 0;

  return (
    <section className="section net">
      <div className="section-head">
        <span className="section-title">
          <IconNetwork size={14} /> Network
        </span>
        <span className="net-speeds">
          <span className="net-dl">↓ {loading || rx == null ? "—" : fmtRate(rx)}</span>
          <span className="net-ul">↑ {loading || tx == null ? "—" : fmtRate(tx)}</span>
        </span>
      </div>
      <svg
        width="100%"
        height={H}
        viewBox={`0 0 ${W} ${H}`}
        className="net-svg"
        preserveAspectRatio="none"
        role="img"
        aria-label={
          loading
            ? "Network activity chart, collecting"
            : `Download ${fmtRate(rx ?? 0)}, upload ${fmtRate(tx ?? 0)}`
        }
      >
        {rxPts && (
          <polyline
            points={rxPts}
            fill="none"
            stroke="var(--accent)"
            strokeWidth="1.5"
            strokeLinejoin="round"
            strokeLinecap="round"
          />
        )}
        {txPts && (
          <polyline
            points={txPts}
            fill="none"
            stroke="var(--text-secondary)"
            strokeWidth="1.25"
            strokeLinejoin="round"
            strokeLinecap="round"
            strokeDasharray="0"
          />
        )}
      </svg>
      {quiet && <div className="empty">No recent activity</div>}
    </section>
  );
}
