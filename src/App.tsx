import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { MetricsSnapshot, Note, DockConfig } from "./types";
import {
  Gauge,
  MemBar,
  NetSpark,
  NotesStrip,
  QuickActions,
  DockBar,
  DiskList,
  ProcList,
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
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<number | null>(null);

  async function refresh() {
    try {
      const m = await invoke<MetricsSnapshot>("get_metrics");
      setMetrics(m);
      setStream((s) => [...s.slice(-59), m.network_rx_bytes + m.network_tx_bytes]);
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

  async function loadDock() {
    try {
      const cfg = await invoke<DockConfig>("get_dock");
      setDockState(cfg);
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
    loadDock();
    timer.current = window.setInterval(refresh, 2000);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
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

  return (
    <div className="app">
      <DockBar dock={dock} onDock={setDock} />

      {error && <div className="error">⚠ {error}</div>}

      <div className="hero">
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

      <DiskList disks={metrics?.disk_infos ?? []} />

      <ProcList procs={metrics?.top_processes ?? []} />

      <QuickActions onOpen={openUrl} />

      <NotesStrip
        notes={notes}
        text={noteText}
        onText={setNoteText}
        onAdd={addNote}
        onDelete={delNote}
      />

      {metrics && (
        <div className="sysinfo">
          <span>{metrics.os_name || "Windows"}</span>
          <span>·</span>
          <span>{metrics.process_count} processes</span>
          <span>·</span>
          <span>up {fmtUptime(metrics.uptime_seconds)}</span>
        </div>
      )}

      <footer className="foot">
        {metrics
          ? `updated ${new Date(metrics.timestamp).toLocaleTimeString()}`
          : "loading…"}
      </footer>
    </div>
  );
}
