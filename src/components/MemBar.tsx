import { usageColor } from "../theme";
import { IconMemory } from "./icons";

interface Props {
  usedMb: number;
  totalMb: number;
  pct: number;
}

export function MemBar({ usedMb, totalMb, pct }: Props) {
  const usedGb = (usedMb / 1024).toFixed(1);
  const totalGb = (totalMb / 1024).toFixed(1);
  const c = usageColor(pct);
  const v = Math.max(0, Math.min(100, pct));
  return (
    <div className="stat">
      <div className="stat-top">
        <span className="stat-label">
          <IconMemory size={13} /> Memory
        </span>
        <span className="stat-val">
          {usedGb}
          <span className="stat-unit"> / {totalGb} GB</span>
        </span>
      </div>
      <div className="track">
        <div className="fill" style={{ width: `${v}%`, background: c }} />
      </div>
      <div className="stat-sub">{pct.toFixed(0)}% utilized</div>
    </div>
  );
}
