import type { ProcInfo } from "../types";

interface Props {
  procs: ProcInfo[];
}

export function ProcList({ procs }: Props) {
  if (!procs || procs.length === 0) return null;
  return (
    <div className="card proc-card">
      <h3>Top processes</h3>
      <div className="proc-list">
        {procs.map((p) => {
          const cpuColor =
            p.cpu >= 20
              ? "var(--danger)"
              : p.cpu >= 5
              ? "var(--warn)"
              : "var(--accent)";
          return (
            <div className="proc-row" key={p.pid}>
              <span className="proc-name">{p.name}</span>
              <div className="proc-bar-wrap">
                <div
                  className="proc-bar"
                  style={{
                    width: `${Math.min(100, p.cpu)}%`,
                    background: cpuColor,
                  }}
                />
              </div>
              <span className="proc-cpu">{p.cpu.toFixed(1)}%</span>
              <span className="proc-mem">
                {(p.mem / 1024 / 1024).toFixed(0)} MB
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
