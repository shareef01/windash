import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { MetricsSnapshot, Note, Settings, DockConfig } from "./types";

const nowIso = () => new Date().toISOString();

const previewSettings: Settings = {
  always_on_top: true,
  refresh_ms: 2000,
  theme: "system",
  show_disks: true,
  show_processes: true,
  show_notes: true,
  show_actions: true,
  mica_enabled: false,
  sort_key: "cpu",
};

const previewNotes: Note[] = [
  { id: 1, text: "Pin this beside the editor while building.", created_at: nowIso() },
  { id: 2, text: "Check disk space on D: before the backup.", created_at: nowIso() },
];

function previewMetrics(): MetricsSnapshot {
  return {
    timestamp: nowIso(),
    cpu_percent: 24.6,
    cpu_cores: 8,
    cpu_brand: "Preview CPU",
    memory_total_mb: 16384,
    memory_used_mb: 9340,
    memory_percent: 57,
    disk_infos: [
      {
        name: "Windows",
        file_system: "NTFS",
        total: 512 * 1024 * 1024 * 1024,
        available: 164 * 1024 * 1024 * 1024,
        mount_point: "C:\\",
      },
      {
        name: "Data",
        file_system: "NTFS",
        total: 1024 * 1024 * 1024 * 1024,
        available: 605 * 1024 * 1024 * 1024,
        mount_point: "D:\\",
      },
    ],
    network_rx_bytes: 5.4 * 1024 * 1024,
    network_tx_bytes: 840 * 1024,
    process_count: 186,
    uptime_seconds: 12 * 86400 + 3 * 3600,
    os_name: "Windows 11",
    top_processes: [
      { name: "chrome.exe", cpu: 8.2, mem: 1.2 * 1024 * 1024 * 1024, pid: 4120, start_time: 1700000000, exe: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe" },
      { name: "Code.exe", cpu: 5.1, mem: 890 * 1024 * 1024, pid: 1884, start_time: 1700000100, exe: "C:\\Users\\me\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe" },
      { name: "MsMpEng.exe", cpu: 2.4, mem: 210 * 1024 * 1024, pid: 904, start_time: 1700000010, exe: "C:\\Program Files\\Windows Defender\\MsMpEng.exe" },
      { name: "explorer.exe", cpu: 1.1, mem: 140 * 1024 * 1024, pid: 1288, start_time: 1700000020, exe: "C:\\Windows\\explorer.exe" },
      { name: "Windash.exe", cpu: 0.4, mem: 48 * 1024 * 1024, pid: 2201, start_time: 1700000200, exe: "C:\\Program Files\\Windash\\Windash.exe" },
    ],
  };
}

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function invokeCmd<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    return invoke<T>(cmd, args);
  }
  return previewInvoke(cmd, args) as T;
}

export async function listenSafe<T>(
  event: string,
  handler: (payload: T) => void
): Promise<() => void> {
  if (!isTauri()) return () => {};
  return listen<T>(event, (e) => handler(e.payload));
}

function previewInvoke(cmd: string, args?: Record<string, unknown>): unknown {
  switch (cmd) {
    case "get_metrics":
      return previewMetrics();
    case "get_notes":
      return previewNotes;
    case "get_settings":
      return previewSettings;
    case "get_dock":
      return { edge: "none", width: 390, always_on_top: true } satisfies DockConfig;
    case "get_system_theme":
      return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
    case "update_settings":
      Object.assign(previewSettings, args?.patch ?? {});
      return { ...previewSettings };
    case "add_note": {
      const text = String(args?.text ?? "").trim();
      const note: Note = { id: Date.now(), text, created_at: nowIso() };
      previewNotes.unshift(note);
      return note;
    }
    case "delete_note": {
      const id = Number(args?.id);
      const idx = previewNotes.findIndex((n) => n.id === id);
      if (idx >= 0) previewNotes.splice(idx, 1);
      return;
    }
    case "set_dock":
      return { edge: args?.edge ?? "none", width: 390, always_on_top: true };
    case "apply_immersive":
    case "window_minimize":
    case "window_hide":
    case "open_explorer":
    case "windows_search":
    case "end_process":
      return;
    default:
      throw new Error(`Preview cannot run ${cmd}`);
  }
}
