import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { MetricsSnapshot, Note, DockConfig, Settings, SortKey } from "./types";
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
  const timer = useRef<number | null>(null);

  // Derive a responsive layout mode from the current window width.
  const layout: "compact" | "normal" | "expanded" =
    width < 320 ? "compact" : width > 480 ? "expanded" : "normal";

  // Effective theme: "system" follows the OS; otherwise use the explicit choice.
  const effectiveTheme: "dark" | "light" =
    settings?.theme === "system" || !settings
      ? systemTheme
      : (settings.theme as "dark" | "light");

  async function refresh() {
    try {
      const m = await invoke<MetricsSnapshot>("get_metrics");
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
      setSettings(await invoke<Settings>("get_settings"));
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
    timer.current = window.setInterval(refresh, settings?.refresh_ms ?? 2000);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
  }, []);

  // React to native window events emitted from Rust (resize / auto-dock).
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
    return () => unlisten.forEach((u) => u());
  }, []);

  // Re-arm the interval when the refresh setting changes.
  useEffect(() => {
    if (timer.current) window.clearInterval(timer.current);
    timer.current = window.setInterval(refresh, settings?.refresh_ms ?? 2000);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
  }, [settings?.refresh_ms]);

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

  return (
    <div className={`app layout-${layout}`} data-theme={effectiveTheme}>
      <DockBar
        dock={dock}
        onDock={setDock}
        onSettings={() => setShowSettings((v) => !v)}
      />

      {showSettings && settings && (
        <SettingsPanel settings={settings} onClose={() => setShowSettings(false)} />
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
        />
      )}

      {settings?.show_actions !== false && <QuickActions onOpen={openUrl} />}

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
        <span className="status-dot" />
        <span>
          {metrics
            ? `${metrics.os_name || "Windows"} · ${metrics.process_count} processes · up ${fmtUptime(metrics.uptime_seconds)}`
            : "Collecting system info…"}
        </span>
        <span className="status-time">
          {metrics ? `updated ${new Date(metrics.timestamp).toLocaleTimeString()}` : ""}
        </span>
      </div>
    </div>
  );
}
