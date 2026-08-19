import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { MetricsSnapshot, Note, ProcInfo, DockConfig } from "./types";
import { Gauge, MemBar, NetSpark, NotesStrip, QuickActions, DockBar } from "./components";

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
      // Re-apply the saved dock now that the window is fully created/visible,
      // so monitor geometry is available and the position actually takes effect.
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

      <header className="topbar">
        <span className="brand">⚡ Windash</span>
        <span className="sub">personal windows dashboard</span>
      </header>

      {error && <div className="error">⚠ {error}</div>}

      <Gauge label="CPU" value={metrics?.cpu_percent ?? 0} unit="%" />
      <MemBar
        usedMb={metrics?.memory_used_mb ?? 0}
        totalMb={metrics?.memory_total_mb ?? 0}
        pct={metrics?.memory_percent ?? 0}
      />
      <NetSpark rx={metrics?.network_rx_bytes ?? 0} tx={metrics?.network_tx_bytes ?? 0} stream={stream} />
      <QuickActions onOpen={openUrl} />

      {metrics?.top_processes && metrics.top_processes.length > 0 && (
        <section className="card">
          <h3>Top processes</h3>
          <ul className="procs">
            {metrics.top_processes.map((p: ProcInfo) => (
              <li key={p.pid}>
                <span className="pname">{p.name}</span>
                <span className="pmeta">{p.cpu.toFixed(1)}% · {(p.mem / 1024 / 1024).toFixed(0)} MB</span>
              </li>
            ))}
          </ul>
        </section>
      )}

      <NotesStrip
        notes={notes}
        text={noteText}
        onText={setNoteText}
        onAdd={addNote}
        onDelete={delNote}
      />

      <footer className="foot">
        {metrics ? `updated ${new Date(metrics.timestamp).toLocaleTimeString()}` : "loading…"}
      </footer>
    </div>
  );
}
