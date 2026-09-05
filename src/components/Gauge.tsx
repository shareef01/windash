import { usageColor, usageTone } from "../theme";
import { IconCpu } from "./icons";

interface Props {
  value: number | null;
  cores?: number;
}

export function Gauge({ value, cores }: Props) {
  const loading = value == null;
  const v = loading ? 0 : Math.max(0, Math.min(100, value));
  const c = loading ? "var(--text-tertiary)" : usageColor(v);
  const tone = loading ? "unknown" : usageTone(v);
  return (
    <div className="stat">
      <div className="stat-top">
        <span className="stat-label">
          <IconCpu size={14} /> CPU
        </span>
        <span className="stat-val" style={{ color: c }}>
          {loading ? "—" : v.toFixed(0)}
          <span className="stat-unit">%</span>
        </span>
      </div>
      <div
        className="track"
        role="progressbar"
        aria-label="CPU utilization"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={loading ? undefined : Math.round(v)}
        aria-valuetext={loading ? "Collecting" : `${Math.round(v)} percent, ${tone}`}
      >
        <div className="fill" style={{ width: loading ? "0%" : `${v}%`, background: c }} />
      </div>
      <div className="stat-sub">
        {loading ? "Collecting…" : cores != null ? `${cores} cores` : "\u00a0"}
      </div>
    </div>
  );
}
