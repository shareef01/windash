import { usageColor, usageTone } from "../theme";
import { IconMemory } from "./icons";

interface Props {
  usedMb: number | null;
  totalMb: number | null;
  pct: number | null;
}

export function MemBar({ usedMb, totalMb, pct }: Props) {
  const loading = usedMb == null || totalMb == null || pct == null;
  const v = loading ? 0 : Math.max(0, Math.min(100, pct));
  const c = loading ? "var(--text-tertiary)" : usageColor(v);
  const tone = loading ? "unknown" : usageTone(v);
  const usedGb = loading ? "—" : (usedMb / 1024).toFixed(1);
  const totalGb = loading ? "—" : (totalMb / 1024).toFixed(1);
  return (
    <div className="stat">
      <div className="stat-top">
        <span className="stat-label">
          <IconMemory size={14} /> Memory
        </span>
        <span className="stat-val">
          {usedGb}
          <span className="stat-unit compact-hide"> / {totalGb} GB</span>
          <span className="stat-unit compact-only"> GB</span>
        </span>
      </div>
      <div
        className="track"
        role="progressbar"
        aria-label="Memory utilization"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={loading ? undefined : Math.round(v)}
        aria-valuetext={loading ? "Collecting" : `${usedGb} of ${totalGb} gigabytes, ${Math.round(v)} percent, ${tone}`}
      >
        <div className="fill" style={{ width: loading ? "0%" : `${v}%`, background: c }} />
      </div>
      <div className="stat-sub">{loading ? "Collecting…" : `${v.toFixed(0)}% used`}</div>
    </div>
  );
}
