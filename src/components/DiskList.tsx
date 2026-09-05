import type { DiskInfo } from "../types";
import { usageColor, usageTone, fmtSize } from "../theme";
import { diskLabel } from "../format";
import { IconDisk } from "./icons";

interface Props {
  disks: DiskInfo[];
  loading?: boolean;
}

export function DiskList({ disks, loading }: Props) {
  const ranked = [...disks].sort((a, b) => b.total - a.total);
  return (
    <section className="section">
      <div className="section-head">
        <span className="section-title">
          <IconDisk size={14} /> Storage
        </span>
      </div>
      {loading && ranked.length === 0 ? (
        <div className="empty">Collecting drives…</div>
      ) : ranked.length === 0 ? (
        <div className="empty">No disks reported</div>
      ) : (
        <div className="disk-list">
          {ranked.map((d) => {
            const used = Math.max(0, d.total - d.available);
            const pct = d.total > 0 ? (used / d.total) * 100 : 0;
            const c = usageColor(pct);
            const tone = usageTone(pct);
            const label = diskLabel(d.mount_point, d.name);
            const title = [d.name, d.mount_point, d.file_system].filter(Boolean).join(" · ");
            return (
              <div className="disk-row" key={d.mount_point + d.name}>
                <div className="disk-head">
                  <span className="disk-mp" title={title}>
                    {label}
                    {d.name && d.name !== label && <span className="disk-vol">{d.name}</span>}
                  </span>
                  <span className="disk-pct">{pct.toFixed(0)}%</span>
                </div>
                <div
                  className="track"
                  role="progressbar"
                  aria-label={`${label} disk usage`}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={Math.round(pct)}
                  aria-valuetext={`${fmtSize(used)} used of ${fmtSize(d.total)}, ${Math.round(pct)} percent, ${tone}`}
                >
                  <div className="fill" style={{ width: `${Math.min(100, pct)}%`, background: c }} />
                </div>
                <div className="disk-foot">
                  {fmtSize(used)} used · {fmtSize(d.available)} free
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
