import type { ProcInfo } from "../types";
import { cpuColor, fmtSize } from "../theme";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  procs: ProcInfo[];
  sortKey: "cpu" | "mem" | "name";
  onSort: (k: "cpu" | "mem" | "name") => void;
  selectedPid: number | null;
  onSelect: (pid: number | null) => void;
}

export function ProcList({ procs, sortKey, onSort, selectedPid, onSelect }: Props) {
  if (!procs || procs.length === 0) {
    return <div className="empty">No processes.</div>;
  }

  const sorted = [...procs].sort((a, b) => {
    if (sortKey === "name") return a.name.localeCompare(b.name);
    if (sortKey === "mem") return b.mem - a.mem;
    return b.cpu - a.cpu;
  });

  async function openLocation(p: ProcInfo) {
    try {
      await invoke("open_explorer", { pid: p.pid });
    } catch (e) {
      alert(String(e));
    }
  }

  async function endTask(p: ProcInfo) {
    if (!confirm(`End "${p.name}" (PID ${p.pid})? This cannot be undone.`)) return;
    try {
      await invoke("end_process", { pid: p.pid });
    } catch (e) {
      alert(String(e));
    }
  }

  return (
    <section className="section">
      <div className="section-head">
        <span>PROCESSES</span>
        <div className="sort">
          {(["cpu", "mem", "name"] as const).map((k) => (
            <button
              key={k}
              className={"sortbtn" + (sortKey === k ? " active" : "")}
              onClick={() => onSort(k)}
              title={`Sort by ${k}`}
            >
              {k === "cpu" ? "CPU" : k === "mem" ? "MEM" : "NAME"}
            </button>
          ))}
        </div>
      </div>

      <div className="proc-head">
        <span>Process</span>
        <span className="num">CPU</span>
        <span className="num">Memory</span>
      </div>

      {sorted.map((p) => (
        <div
          key={p.pid}
          className={"proc-row" + (selectedPid === p.pid ? " selected" : "")}
          onClick={() => onSelect(selectedPid === p.pid ? null : p.pid)}
          onContextMenu={(e) => {
            e.preventDefault();
            const choice = window.prompt(
              `Process: ${p.name} (PID ${p.pid})\n\nChoose an action:\n1 = Open file location${p.exe ? "" : " (unavailable)"}\n2 = End task`,
              p.exe ? "1" : "2"
            );
            if (choice === "1" && p.exe) openLocation(p);
            else if (choice === "2") endTask(p);
          }}
        >
          <span className="proc-name" title={p.exe || p.name}>
            {p.name}
          </span>
          <span className="num">
            <span
              className="mini-bar"
              style={{ width: `${Math.min(100, p.cpu)}%`, background: cpuColor(p.cpu) }}
            />
            <span className="proc-cpu">{p.cpu.toFixed(1)}%</span>
          </span>
          <span className="num proc-mem">{fmtSize(p.mem)}</span>
        </div>
      ))}
    </section>
  );
}
