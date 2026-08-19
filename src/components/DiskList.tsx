import type { DiskInfo } from "../types";

interface Props {
  disks: DiskInfo[];
}

function fmtGb(b: number): string {
  const gb = b / 1024 / 1024 / 1024;
  return gb >= 1024 ? `${(gb / 1024).toFixed(1)} TB` : `${gb.toFixed(0)} GB`;
}

export function DiskList({ disks }: Props) {
  if (!disks || disks.length === 0) return null;
  return (
    <div className="card disk-card">
      <h3>Disks</h3>
      <div className="disk-list">
        {disks.map((d) => {
          const used = d.total - d.available;
          const pct = d.total > 0 ? (used / d.total) * 100 : 0;
          const color =
            pct >= 85
              ? "var(--danger)"
              : pct >= 60
              ? "var(--warn)"
              : "var(--accent)";
          return (
            <div className="disk-row" key={d.mount_point + d.name}>
              <div className="disk-head">
                <span className="disk-mp">{d.mount_point}</span>
                <span className="disk-pct">{pct.toFixed(0)}%</span>
              </div>
              <div className="track">
                <div
                  className="fill"
                  style={{
                    width: `${pct}%`,
                    background: `linear-gradient(90deg, ${color}, ${color}cc)`,
                  }}
                />
              </div>
              <div className="disk-foot">
                {fmtGb(used)} / {fmtGb(d.total)}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
