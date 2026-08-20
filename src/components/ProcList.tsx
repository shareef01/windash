import type { ProcInfo } from "../types";
import { cpuColor, fmtSize } from "../theme";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { IconProcess, IconFolder, IconClose } from "./icons";

interface Props {
  procs: ProcInfo[];
  sortKey: "cpu" | "mem" | "name";
  onSort: (k: "cpu" | "mem" | "name") => void;
  selectedPid: number | null;
  onSelect: (pid: number | null) => void;
}

interface MenuState {
  pid: number;
  x: number;
  y: number;
}

export function ProcList({ procs, sortKey, onSort, selectedPid, onSelect }: Props) {
  const [menu, setMenu] = useState<MenuState | null>(null);

  if (!procs || procs.length === 0) {
    return <div className="empty">No processes.</div>;
  }

  const sorted = [...procs].sort((a, b) => {
    if (sortKey === "name") return a.name.localeCompare(b.name);
    if (sortKey === "mem") return b.mem - a.mem;
    return b.cpu - a.cpu;
  });

  const byPid = (pid: number) => sorted.find((p) => p.pid === pid);

  async function openLocation(p: ProcInfo | undefined) {
    if (!p) return;
    setMenu(null);
    try {
      await invoke("open_explorer", { pid: p.pid });
    } catch (e) {
      alert(String(e));
    }
  }

  async function endTask(p: ProcInfo | undefined) {
    setMenu(null);
    if (!p) return;
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
        <span className="section-title">
          <IconProcess size={13} /> Processes
        </span>
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
            onSelect(p.pid);
            setMenu({ pid: p.pid, x: e.clientX, y: e.clientY });
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

      {menu && (
        <>
          <div className="ctx-backdrop" onClick={() => setMenu(null)} onContextMenu={(e) => { e.preventDefault(); setMenu(null); }} />
          <div
            className="ctx-menu"
            style={{ position: "fixed", left: menu.x, top: menu.y }}
            role="menu"
          >
            <button
              className="ctx-item"
              role="menuitem"
              disabled={!byPid(menu.pid)?.exe}
              onClick={() => openLocation(byPid(menu.pid))}
            >
              <IconFolder size={14} /> Open file location
            </button>
            <button
              className="ctx-item ctx-danger"
              role="menuitem"
              onClick={() => endTask(byPid(menu.pid))}
            >
              <IconClose size={14} /> End task
            </button>
          </div>
        </>
      )}
    </section>
  );
}
