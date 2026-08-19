import type { ProcInfo } from "../types";
import { cpuColor, fmtSize } from "../theme";

interface Props {
  procs: ProcInfo[];
}

export function ProcList({ procs }: Props) {
  if (!procs || procs.length === 0) return null;
  return (
    <section className="section">
      <div className="section-head">
        <span className="section-title">Top processes</span>
      </div>
      <div className="proc-list">
        {procs.map((p) => {
          const c = cpuColor(p.cpu);
          return (
            <div className="proc-row" key={p.pid}>
              <span className="proc-name" title={p.name}>
                {p.name}
              </span>
              <div className="proc-bar-wrap">
                <div
                  className="proc-bar"
                  style={{ width: `${Math.min(100, p.cpu)}%`, background: c }}
                />
              </div>
              <span className="proc-cpu">{p.cpu.toFixed(1)}%</span>
              <span className="proc-mem">{fmtSize(p.mem)}</span>
            </div>
          );
        })}
      </div>
    </section>
  );
}
