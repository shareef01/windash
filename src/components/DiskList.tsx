import type { DiskInfo } from "../types";

interface Props {
  disks: DiskInfo[];
}

function fmtGb(b: number): string {
  const gb = b / 1024 / 1024 / 1024;
  return gb >= 1024 ? `${(gb / 1024).toFixed(1)} TB` : `${gb.toFixed(0)} GB`;
}

function color(pct: number): string {
  return pct >= 85 ? "var(--red)" : pct >= 60 ? "var(--amber)" : "var(--accent)";
}

export function DiskList({ disks }: Props) {
  if (!disks || disks.length === 0) return null;
  return (
    <section className="section">
      <div className="section-head">
        <span className="section-title">Disks</span>
      </div>
      <div className="disk-list">
        {disks.map((d) => {
          const used = d.total - d.available;
          const pct = d.total > 0 ? (used / d.total) * 100 : 0;
          return (
            <div className="disk-row" key={d.mount_point + d.name}>
              <div className="disk-head">
                <span className="disk-mp">{d.mount_point}</span>
                <span className="disk-pct">{pct.toFixed(0)}%</span>
              </div>
              <div className="track">
                <div
                  className="fill"
                  style={{ width: `${pct}%`, background: color(pct) }}
                />
              </div>
              <div className="disk-foot">
                {fmtGb(used)} / {fmtGb(d.total)}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
