import type { DiskInfo } from "../types";
import { usageColor, fmtSize } from "../theme";
import { IconDisk } from "./icons";

interface Props {
  disks: DiskInfo[];
}

export function DiskList({ disks }: Props) {
  if (!disks || disks.length === 0) return null;
  return (
    <section className="section">
      <div className="section-head">
        <span className="section-title">
          <IconDisk size={13} /> Disks
        </span>
      </div>
      <div className="disk-list">
        {disks.map((d) => {
          const used = d.total - d.available;
          const pct = d.total > 0 ? (used / d.total) * 100 : 0;
          const c = usageColor(pct);
          return (
            <div className="disk-row" key={d.mount_point + d.name}>
              <div className="disk-head">
                <span className="disk-mp">{d.mount_point}</span>
                <span className="disk-pct">{pct.toFixed(0)}%</span>
              </div>
              <div className="track">
                <div
                  className="fill"
                  style={{ width: `${pct}%`, background: c }}
                />
              </div>
              <div className="disk-foot">
                {fmtSize(used)} / {fmtSize(d.total)}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
