import type { ProcInfo } from "../types";

interface Props {
  procs: ProcInfo[];
}

function color(cpu: number): string {
  return cpu >= 20 ? "var(--red)" : cpu >= 5 ? "var(--amber)" : "var(--accent)";
}

export function ProcList({ procs }: Props) {
  if (!procs || procs.length === 0) return null;
  return (
    <section className="section">
      <div className="section-head">
        <span className="section-title">Top processes</span>
      </div>
      <div className="proc-list">
        {procs.map((p) => (
          <div className="proc-row" key={p.pid}>
            <span className="proc-name" title={p.name}>
              {p.name}
            </span>
            <div className="proc-bar-wrap">
              <div
                className="proc-bar"
                style={{ width: `${Math.min(100, p.cpu)}%`, background: color(p.cpu) }}
              />
            </div>
            <span className="proc-cpu">{p.cpu.toFixed(1)}%</span>
            <span className="proc-mem">{(p.mem / 1024 / 1024).toFixed(0)} MB</span>
          </div>
        ))}
      </div>
    </section>
  );
}
