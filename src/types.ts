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
}

export interface MetricsSnapshot {
  timestamp: string;
  cpu_percent: number;
  cpu_cores: number;
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
