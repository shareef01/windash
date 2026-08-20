import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { MetricsSnapshot, Note, DockConfig, Settings, SortKey } from "./types";
import { fmtSize } from "./theme";
import {
  Gauge,
  MemBar,
  NetSpark,
  NotesStrip,
  QuickActions,
  DockBar,
  DiskList,
  ProcList,
  SettingsPanel,
} from "./components";
import { IconCopy } from "./components/icons";

function fmtUptime(s: number): string {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return `${h}h ${m}m`;
}

export default function App() {
  const [metrics, setMetrics] = useState<MetricsSnapshot | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [stream, setStream] = useState<number[]>([]);
  const [noteText, setNoteText] = useState("");
  const [dock, setDockState] = useState<DockConfig | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [sortKey, setSortKey] = useState<SortKey>("cpu");
  const [selectedPid, setSelectedPid] = useState<number | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [width, setWidth] = useState<number>(390);
  const [systemTheme, setSystemTheme] = useState<"dark" | "light">("dark");
  const [paused, setPaused] = useState(false);
  const timer = useRef<number | null>(null);
  const focusFilterRef = useRef<(() => void) | null>(null);
  const [statusFlash, setStatusFlash] = useState<string | null>(null);

  // Derive a responsive layout mode from the current window width.
  const layout: "compact" | "normal" | "expanded" =
    width < 340 ? "compact" : width > 520 ? "expanded" : "normal";

  // Effective theme: "system" follows the OS; otherwise use the explicit choice.
  const effectiveTheme: "dark" | "light" =
    settings?.theme === "system" || !settings
      ? systemTheme
      : (settings.theme as "dark" | "light");

  async function refresh() {
    try {
      const m = await invoke<MetricsSnapshot>("get_metrics", { sort_by: sortKey });
      setMetrics(m);
      setStream((s) => [...s.slice(-59), m.network_rx_bytes]);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function loadNotes() {
    try {
      setNotes(await invoke<Note[]>("get_notes"));
    } catch (e) {
      setError(String(e));
    }
  }

  async function loadSettings() {
    try {
      const s = await invoke<Settings>("get_settings");
      setSettings(s);
      // Restore the persisted process sort key.
      if (s.sort_key === "mem" || s.sort_key === "name") setSortKey(s.sort_key);
    } catch (e) {
      setError(String(e));
    }
  }

  async function loadSystemTheme() {
    try {
      setSystemTheme((await invoke<string>("get_system_theme")) as "dark" | "light");
    } catch {
      /* keep dark default */
    }
  }

  async function loadDock() {
    try {
      const cfg = await invoke<DockConfig>("get_dock");
      setDockState(cfg);
      await invoke("apply_immersive");
      await invoke<DockConfig>("set_dock", { edge: cfg.edge });
    } catch (e) {
      setError(String(e));
    }
  }

  async function setDock(edge: "none" | "left" | "right") {
    try {
      setDockState(await invoke<DockConfig>("set_dock", { edge }));
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    refresh();
    loadNotes();
    loadSettings();
    loadSystemTheme();
    loadDock();
  }, []);

  // Single source of truth for the polling timer. Re-arms whenever the refresh
  // interval or the active sort changes, and stops entirely while paused.
  useEffect(() => {
    if (timer.current) window.clearInterval(timer.current);
    if (paused) return;
    // Immediate refresh so a new sort key / interval takes effect at once.
    refresh();
    timer.current = window.setInterval(refresh, settings?.refresh_ms ?? 2000);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
  }, [settings?.refresh_ms, sortKey, paused]);

  // React to native window events emitted from Rust (resize / auto-dock / focus).
  useEffect(() => {
    let unlisten: Array<() => void> = [];
    listen<{ width: number; height: number }>("windash://resized", (e) => {
      setWidth(e.payload.width);
    }).then((u) => unlisten.push(u));
    listen<string>("windash://dock", (e) => {
      setDockState((prev) =>
        prev ? { ...prev, edge: e.payload as DockConfig["edge"] } : prev
      );
    }).then((u) => unlisten.push(u));
    // Re-read the OS theme when the window is focused (keeps "Follow Windows"
    // in sync without a polling timer that would flash consoles).
    listen("windash://focused", () => {
      if (settings?.theme === "system") loadSystemTheme();
    }).then((u) => unlisten.push(u));
    return () => unlisten.forEach((u) => u());
  }, []);

  // When the theme follows Windows, re-read the OS theme when the window is
  // (re)focused so switching the OS light/dark mode updates Windash — but we
  // do NOT poll on an interval (that would spawn a console `reg` query every
  // few seconds and flash empty terminals). Instead we listen for focus.
  useEffect(() => {
    if (settings?.theme !== "system") return;
    loadSystemTheme();
  }, [settings?.theme]);

  // Persist the active process sort key so it survives restarts.
  useEffect(() => {
    if (!settings) return;
    if (settings.sort_key === sortKey) return;
    invoke("update_settings", { patch: { sort_key: sortKey } }).catch(() => {});
  }, [sortKey, settings]);

  // Keyboard: "/" or Ctrl+F focuses the process filter from anywhere (unless
  // the user is already typing in a field).
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const el = document.activeElement;
      const typing =
        el instanceof HTMLInputElement ||
        el instanceof HTMLTextAreaElement ||
        (el as HTMLElement | null)?.isContentEditable;
      if ((e.key === "/" && !typing) || (e.key === "f" && e.ctrlKey)) {
        e.preventDefault();
        focusFilterRef.current?.();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  async function addNote() {
    const t = noteText.trim();
    if (!t) return;
    try {
      await invoke("add_note", { text: t });
      setNoteText("");
      await loadNotes();
    } catch (e) {
      setError(String(e));
    }
  }

  async function delNote(id: number) {
    try {
      await invoke("delete_note", { id });
      await loadNotes();
    } catch (e) {
      setError(String(e));
    }
  }

  async function copySummary() {
    if (!metrics) return;
    const text = [
      `Windash — ${metrics.os_name || "Windows"}`,
      `CPU: ${metrics.cpu_percent.toFixed(1)}% (${metrics.cpu_cores} cores)`,
      `Memory: ${fmtSize(metrics.memory_used_mb * 1024 * 1024)} / ${fmtSize(metrics.memory_total_mb * 1024 * 1024)} (${metrics.memory_percent}%)`,
      `Processes: ${metrics.process_count}`,
      `Uptime: ${fmtUptime(metrics.uptime_seconds)}`,
    ].join("\n");
    try {
      await navigator.clipboard.writeText(text);
      setStatusFlash("Copied system summary");
      window.setTimeout(() => setStatusFlash(null), 1400);
    } catch {
      setError("Clipboard unavailable");
    }
  }

  return (
    <div className={`app layout-${layout}`} data-theme={effectiveTheme}>
      <DockBar
        dock={dock}
        paused={paused}
        onTogglePause={() => setPaused((p) => !p)}
        onDock={setDock}
        onSettings={() => setShowSettings((v) => !v)}
      />

      {showSettings && settings && (
        <SettingsPanel
          settings={settings}
          onClose={() => setShowSettings(false)}
          onApply={(s) => setSettings(s)}
          onError={(e) => setError(e)}
        />
      )}

      {error && <div className="error">⚠ {error}</div>}

      <div className="stats-grid">
        <Gauge
          label="CPU"
          value={metrics?.cpu_percent ?? 0}
          sub={metrics ? `${metrics.cpu_cores} cores` : undefined}
        />
        <MemBar
          usedMb={metrics?.memory_used_mb ?? 0}
          totalMb={metrics?.memory_total_mb ?? 0}
          pct={metrics?.memory_percent ?? 0}
        />
      </div>

      <NetSpark
        rx={metrics?.network_rx_bytes ?? 0}
        tx={metrics?.network_tx_bytes ?? 0}
        stream={stream}
      />

      {settings?.show_disks !== false && (
        <DiskList disks={metrics?.disk_infos ?? []} />
      )}

      {settings?.show_processes !== false && (
        <ProcList
          procs={metrics?.top_processes ?? []}
          sortKey={sortKey}
          onSort={setSortKey}
          selectedPid={selectedPid}
          onSelect={setSelectedPid}
          onFocusFilter={(fn) => (focusFilterRef.current = fn)}
        />
      )}

      {settings?.show_actions !== false && <QuickActions onOpen={openUrl} onError={(e) => setError(e)} />}

      {settings?.show_notes !== false && (
        <NotesStrip
          notes={notes}
          text={noteText}
          onText={setNoteText}
          onAdd={addNote}
          onDelete={delNote}
        />
      )}

      <div className="statusbar">
        <span className="status-dot" style={paused ? { background: "var(--amber)", boxShadow: "0 0 7px var(--amber)" } : undefined} />
        <span>
          {statusFlash
            ? statusFlash
            : metrics
              ? `${metrics.os_name || "Windows"} · ${metrics.process_count} processes · up ${fmtUptime(metrics.uptime_seconds)}`
              : "Collecting system info…"}
        </span>
        <span className="status-actions">
          <button
            className="status-copy"
            onClick={copySummary}
            title="Copy system summary"
            aria-label="Copy system summary"
          >
            <IconCopy size={13} />
          </button>
          <span className="status-time">
            {paused
              ? "paused"
              : metrics
                ? `updated ${new Date(metrics.timestamp).toLocaleTimeString()}`
                : ""}
          </span>
        </span>
      </div>
    </div>
  );
}
