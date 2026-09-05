import type { ProcInfo } from "../types";
import { cpuColor } from "../theme";
import { fmtMem, filterProcs, friendlyError } from "../format";
import { invokeCmd } from "../bridge";
import { useState, useRef, useEffect } from "react";
import { IconProcess, IconFolder, IconClose, IconSearch, IconCopy } from "./icons";

interface Props {
  procs: ProcInfo[];
  sortKey: "cpu" | "mem" | "name";
  onSort: (k: "cpu" | "mem" | "name") => void;
  selectedPid: number | null;
  onSelect: (pid: number | null) => void;
  onFocusFilter?: (fn: () => void) => void;
  onError?: (message: string, detail?: string) => void;
  onToast?: (message: string) => void;
  loading?: boolean;
}

interface MenuState {
  pid: number;
  x: number;
  y: number;
}

interface ConfirmState {
  pid: number;
  name: string;
}

const MENU_ITEMS = ["end", "location", "copy-details", "copy-pid"] as const;

export function ProcList({
  procs,
  sortKey,
  onSort,
  selectedPid,
  onSelect,
  onFocusFilter,
  onError,
  onToast,
  loading,
}: Props) {
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [query, setQuery] = useState("");
  const [confirm, setConfirm] = useState<ConfirmState | null>(null);
  const [menuIndex, setMenuIndex] = useState(0);
  const searchRef = useRef<HTMLInputElement | null>(null);
  const listRef = useRef<HTMLDivElement | null>(null);
  const confirmCancelRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    onFocusFilter?.(() => searchRef.current?.focus());
    return () => onFocusFilter?.(() => {});
  }, [onFocusFilter]);

  useEffect(() => {
    if (!menu) return;
    function onEscape(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        setMenu(null);
        listRef.current?.focus();
      }
    }
    window.addEventListener("keydown", onEscape);
    const t = window.setTimeout(() => {
      menuRef.current?.querySelector<HTMLButtonElement>("[role='menuitem']:not(:disabled)")?.focus();
    }, 0);
    return () => {
      window.removeEventListener("keydown", onEscape);
      window.clearTimeout(t);
    };
  }, [menu]);

  useEffect(() => {
    if (confirm) confirmCancelRef.current?.focus();
  }, [confirm]);

  const sorted = filterProcs(procs, query);
  const byPid = (pid: number) => procs.find((p) => p.pid === pid);

  function openMenu(pid: number, clientX: number, clientY: number) {
    const menuWidth = 188;
    const menuHeight = 168;
    const clampedX = Math.max(8, Math.min(clientX, window.innerWidth - menuWidth - 8));
    const clampedY = Math.max(8, Math.min(clientY, window.innerHeight - menuHeight - 8));
    setMenu({ pid, x: clampedX, y: clampedY });
    setMenuIndex(0);
  }

  function onListKeyDown(e: React.KeyboardEvent) {
    if (sorted.length === 0) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const idx = sorted.findIndex((p) => p.pid === selectedPid);
      let next = idx < 0 ? 0 : idx + (e.key === "ArrowDown" ? 1 : -1);
      next = Math.max(0, Math.min(sorted.length - 1, next));
      onSelect(sorted[next].pid);
      const row = listRef.current?.querySelector<HTMLElement>(`[data-pid="${sorted[next].pid}"]`);
      row?.scrollIntoView({ block: "nearest" });
    } else if ((e.key === "Enter" || e.key === "ContextMenu" || (e.key === "F10" && e.shiftKey)) && selectedPid != null) {
      e.preventDefault();
      const row = listRef.current?.querySelector<HTMLElement>(`[data-pid="${selectedPid}"]`);
      if (row) {
        const r = row.getBoundingClientRect();
        openMenu(selectedPid, r.left, r.bottom);
      }
    } else if (e.key === "Delete" && selectedPid != null) {
      const p = byPid(selectedPid);
      if (p) {
        e.preventDefault();
        setConfirm({ pid: p.pid, name: p.name });
      }
    }
  }

  function onMenuKeyDown(e: React.KeyboardEvent) {
    const enabled = MENU_ITEMS.filter((id) => {
      if (id === "location") return Boolean(byPid(menu?.pid ?? -1)?.exe);
      return true;
    });
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const dir = e.key === "ArrowDown" ? 1 : -1;
      const next = (menuIndex + dir + enabled.length) % enabled.length;
      setMenuIndex(next);
      const buttons = menuRef.current?.querySelectorAll<HTMLButtonElement>("[role='menuitem']:not(:disabled)");
      buttons?.[next]?.focus();
    } else if (e.key === "Tab") {
      e.preventDefault();
      setMenu(null);
      listRef.current?.focus();
    }
  }

  async function copy(text: string, label: string) {
    try {
      await navigator.clipboard.writeText(text);
      onToast?.(`Copied ${label}`);
    } catch {
      onError?.("Couldn't copy to the clipboard.");
    }
  }

  async function openLocation(p: ProcInfo | undefined) {
    if (!p) return;
    setMenu(null);
    try {
      await invokeCmd("open_explorer", { pid: p.pid });
    } catch (e) {
      const f = friendlyError(e, "Couldn't find that process's file location.");
      onError?.(f.message, f.detail);
    }
  }

  function requestEnd(p: ProcInfo | undefined) {
    setMenu(null);
    if (!p) return;
    setConfirm({ pid: p.pid, name: p.name });
  }

  async function confirmEnd() {
    if (!confirm) return;
    const target = { ...confirm };
    const still = byPid(target.pid);
    setConfirm(null);
    if (!still || still.name !== target.name) {
      onError?.("That process is no longer in the list. Refresh and try again.");
      return;
    }
    try {
      await invokeCmd("end_process", { pid: target.pid });
      onToast?.(`Ended ${target.name}`);
      if (selectedPid === target.pid) onSelect(null);
    } catch (e) {
      const f = friendlyError(e, "Couldn't end that process.");
      onError?.(f.message, f.detail);
    }
  }

  const emptyMessage = loading && procs.length === 0
    ? "Collecting processes…"
    : query.trim()
      ? "No matching processes"
      : "No processes";

  return (
    <section className="section proc-section">
      <div className="section-head">
        <span className="section-title">
          <IconProcess size={14} /> Processes
          {query.trim() && (
            <span className="proc-count">
              {sorted.length}/{procs.length}
            </span>
          )}
        </span>
        <div className="sort" role="group" aria-label="Sort processes">
          {(["cpu", "mem", "name"] as const).map((k) => (
            <button
              key={k}
              type="button"
              className={"sortbtn" + (sortKey === k ? " active" : "")}
              onClick={() => onSort(k)}
              aria-pressed={sortKey === k}
              title={`Sort by ${k === "cpu" ? "CPU" : k === "mem" ? "memory" : "name"}`}
            >
              {k === "cpu" ? "CPU" : k === "mem" ? "Mem" : "Name"}
            </button>
          ))}
        </div>
      </div>

      <div className="proc-search">
        <IconSearch size={14} />
        <input
          ref={searchRef}
          type="text"
          value={query}
          placeholder="Filter processes"
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Filter processes"
          onKeyDown={(e) => {
            if (e.key === "Escape" && query) {
              e.preventDefault();
              setQuery("");
            }
          }}
        />
        {query && (
          <button
            type="button"
            className="proc-search-clear"
            onClick={() => setQuery("")}
            aria-label="Clear filter"
            title="Clear"
          >
            ×
          </button>
        )}
      </div>

      <div className="proc-head" aria-hidden="true">
        <span>Process</span>
        <span className="num">CPU</span>
        <span className="num">Memory</span>
      </div>

      <div
        ref={listRef}
        className="proc-list"
        role="listbox"
        aria-label="Processes"
        tabIndex={0}
        onKeyDown={onListKeyDown}
      >
        {sorted.length === 0 ? (
          <div className="empty">{emptyMessage}</div>
        ) : (
          sorted.map((p) => (
            <div
              key={p.pid}
              data-pid={p.pid}
              role="option"
              aria-selected={selectedPid === p.pid}
              title={p.exe || p.name}
              className={"proc-row" + (selectedPid === p.pid ? " selected" : "")}
              onClick={() => onSelect(selectedPid === p.pid ? null : p.pid)}
              onContextMenu={(e) => {
                e.preventDefault();
                onSelect(p.pid);
                openMenu(p.pid, e.clientX, e.clientY);
              }}
            >
              <span className="proc-name">{p.name}</span>
              <span className="num">
                <span
                  className="mini-bar"
                  style={{ width: `${Math.min(100, p.cpu)}%`, background: cpuColor(p.cpu) }}
                />
                <span className="proc-cpu">{p.cpu >= 100 ? p.cpu.toFixed(0) : p.cpu.toFixed(1)}%</span>
              </span>
              <span className="num proc-mem">{fmtMem(p.mem)}</span>
            </div>
          ))
        )}
      </div>

      {confirm && (
        <div className="confirm-bar" role="alertdialog" aria-labelledby="end-title" aria-describedby="end-desc">
          <div>
            <div id="end-title" className="confirm-title">
              End process?
            </div>
            <div id="end-desc" className="confirm-desc">
              {confirm.name} · PID {confirm.pid}
            </div>
          </div>
          <div className="confirm-actions">
            <button
              ref={confirmCancelRef}
              type="button"
              className="btn btn-quiet"
              onClick={() => {
                setConfirm(null);
                listRef.current?.focus();
              }}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  e.preventDefault();
                  setConfirm(null);
                  listRef.current?.focus();
                }
              }}
            >
              Cancel
            </button>
            <button type="button" className="btn btn-danger" onClick={confirmEnd}>
              End task
            </button>
          </div>
        </div>
      )}

      {menu && (
        <>
          <div
            className="ctx-backdrop"
            onClick={() => setMenu(null)}
            onContextMenu={(e) => {
              e.preventDefault();
              setMenu(null);
            }}
          />
          <div
            ref={menuRef}
            className="ctx-menu"
            style={{ position: "fixed", left: menu.x, top: menu.y }}
            role="menu"
            aria-label="Process actions"
            onKeyDown={onMenuKeyDown}
          >
            <button
              type="button"
              className="ctx-item ctx-danger"
              role="menuitem"
              onClick={() => requestEnd(byPid(menu.pid))}
            >
              <IconClose size={14} /> End task
            </button>
            <button
              type="button"
              className="ctx-item"
              role="menuitem"
              disabled={!byPid(menu.pid)?.exe}
              onClick={() => openLocation(byPid(menu.pid))}
            >
              <IconFolder size={14} /> Open file location
            </button>
            <div className="ctx-sep" />
            <button
              type="button"
              className="ctx-item"
              role="menuitem"
              onClick={() => {
                const p = byPid(menu.pid);
                if (p) copy(`${p.name} (PID ${p.pid}) — CPU ${p.cpu.toFixed(1)}% · ${fmtMem(p.mem)}`, "details");
                setMenu(null);
              }}
            >
              <IconCopy size={14} /> Copy details
            </button>
            <button
              type="button"
              className="ctx-item"
              role="menuitem"
              onClick={() => {
                const p = byPid(menu.pid);
                if (p) copy(String(p.pid), "PID");
                setMenu(null);
              }}
            >
              <IconCopy size={14} /> Copy PID
            </button>
          </div>
        </>
      )}
    </section>
  );
}
