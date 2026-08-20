import type { ProcInfo } from "../types";
import { cpuColor, fmtSize } from "../theme";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { IconProcess, IconFolder, IconClose, IconSearch, IconCopy } from "./icons";

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
  const [query, setQuery] = useState("");
  const [copied, setCopied] = useState<string | null>(null);

  if (!procs || procs.length === 0) {
    return <div className="empty">No processes.</div>;
  }

  // Backend already returns the list pre-sorted by the active key; keep order
  // stable so rows don't jump between refreshes. Filtering is client-side by
  // name (the top-30 pool is the candidate set).
  const q = query.trim().toLowerCase();
  const sorted = q
    ? procs.filter((p) => p.name.toLowerCase().includes(q) || (p.exe ?? "").toLowerCase().includes(q))
    : procs;

  const byPid = (pid: number) => procs.find((p) => p.pid === pid);

  // Keyboard navigation: Up/Down move selection, Enter opens the context menu.
  function onKeyDown(e: React.KeyboardEvent) {
    if (sorted.length === 0) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const idx = sorted.findIndex((p) => p.pid === selectedPid);
      let next = idx < 0 ? 0 : idx + (e.key === "ArrowDown" ? 1 : -1);
      next = Math.max(0, Math.min(sorted.length - 1, next));
      onSelect(sorted[next].pid);
    } else if (e.key === "Enter" && selectedPid != null) {
      e.preventDefault();
      const row = document.querySelector<HTMLElement>(`[data-pid="${selectedPid}"]`);
      if (row) {
        const r = row.getBoundingClientRect();
        setMenu({ pid: selectedPid, x: r.left, y: r.bottom });
      }
    }
  }

  async function copy(text: string, label: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(label);
      window.setTimeout(() => setCopied(null), 1200);
    } catch {
      /* clipboard may be unavailable; ignore */
    }
  }

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
    <section
      className="section"
      tabIndex={0}
      role="grid"
      aria-label="Processes"
      onKeyDown={onKeyDown}
    >
      <div className="section-head">
        <span className="section-title">
          <IconProcess size={13} /> Processes
          {q && <span className="proc-count">{sorted.length}/{procs.length}</span>}
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

      <div className="proc-search">
        <IconSearch size={13} />
        <input
          type="text"
          value={query}
          placeholder="Filter processes…"
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Filter processes"
          onKeyDown={(e) => e.stopPropagation()}
        />
        {query && (
          <button className="proc-search-clear" onClick={() => setQuery("")} aria-label="Clear filter" title="Clear">
            &#10005;
          </button>
        )}
      </div>

      <div className="proc-head">
        <span>Process</span>
        <span className="num">CPU</span>
        <span className="num">Memory</span>
      </div>

      {sorted.length === 0 && <div className="empty">No matches.</div>}

      {sorted.map((p) => (
        <div
          key={p.pid}
          data-pid={p.pid}
          role="row"
          aria-selected={selectedPid === p.pid}
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
              className="ctx-item ctx-danger"
              role="menuitem"
              onClick={() => endTask(byPid(menu.pid))}
            >
              <IconClose size={14} /> End task
            </button>
            <button
              className="ctx-item"
              role="menuitem"
              disabled={!byPid(menu.pid)?.exe}
              onClick={() => openLocation(byPid(menu.pid))}
            >
              <IconFolder size={14} /> Open file location
            </button>
            <div className="ctx-sep" />
            <button
              className="ctx-item"
              role="menuitem"
              onClick={() => {
                const p = byPid(menu.pid);
                if (p) copy(`${p.name} (PID ${p.pid}) — CPU ${p.cpu.toFixed(1)}% · ${fmtSize(p.mem)}`, "info");
                setMenu(null);
              }}
            >
              <IconCopy size={14} /> Copy details
            </button>
            <button
              className="ctx-item"
              role="menuitem"
              onClick={() => {
                const p = byPid(menu.pid);
                if (p) copy(String(p.pid), "pid");
                setMenu(null);
              }}
            >
              <IconCopy size={14} /> Copy PID
            </button>
            {copied && <div className="ctx-copied">Copied {copied}</div>}
          </div>
        </>
      )}
    </section>
  );
}
