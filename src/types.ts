export interface DockConfig {
  edge: "none" | "left" | "right";
  width: number;
  always_on_top: boolean;
}

export interface DiskInfo {
  name: string;
  file_system: string;
  total: number;
  available: number;
  mount_point: string;
}

export interface ProcInfo {
  name: string;
  cpu: number;
  mem: number;
  pid: number;
  start_time: number;
  exe: string;
}

export interface MetricsSnapshot {
  timestamp: string;
  cpu_percent: number;
  cpu_cores: number;
  cpu_brand: string;
  memory_total_mb: number;
  memory_used_mb: number;
  memory_percent: number;
  disk_infos: DiskInfo[];
  network_rx_bytes: number;
  network_tx_bytes: number;
  process_count: number;
  uptime_seconds: number;
  os_name: string;
  top_processes: ProcInfo[];
}

export interface Note {
  id: number;
  text: string;
  created_at: string;
}

export interface Settings {
  always_on_top: boolean;
  refresh_ms: number;
  theme: "dark" | "light" | "system";
  show_disks: boolean;
  show_processes: boolean;
  show_notes: boolean;
  show_actions: boolean;
  mica_enabled: boolean;
  sort_key: "cpu" | "mem" | "name";
}

export type SortKey = "cpu" | "mem" | "name";

export interface NetSample {
  rx: number;
  tx: number;
}
