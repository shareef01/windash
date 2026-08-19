// metrics.rs — sysinfo 0.32 wrapping.
// Uses sysinfo's refresh model: refresh_all() updates cpu_usage() to reflect
// the interval since the last refresh. No manual delta math needed.

use sysinfo::{Disks, Networks, System};
use std::time::{Duration, Instant};

pub struct SystemMetrics {
    system: System,
    networks: Networks,
    disks: Disks,
    last_update: Instant,
    /// Real wall-clock interval between the two most recent refreshes. Used to
    /// convert raw network deltas into accurate bytes-per-second rates.
    interval: Duration,
}

impl SystemMetrics {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();

        Self {
            system,
            networks,
            disks,
            last_update: Instant::now(),
            interval: Duration::from_secs(2),
        }
    }

    pub fn refresh(&mut self) {
        // Capture the real interval BEFORE resetting the timestamp, so the rate
        // math uses the wall-clock time the previous sample actually covered.
        self.interval = self.last_update.elapsed();
        self.last_update = Instant::now();
        self.system.refresh_all();
        self.networks.refresh_list();
    }

    /// Seconds covered by the most recent refresh interval, clamped to a sane
    /// range so a stalled timer can't yield divide-by-zero or an absurd rate.
    pub fn elapsed_secs(&self) -> f64 {
        let s = self.interval.as_secs_f64();
        if s <= 0.0 || s > 60.0 {
            1.0
        } else {
            s
        }
    }

    /// Resolve the directory containing a process' executable, for the
    /// "open file location" action. Returns None if the process isn't found or
    /// has no usable path.
    pub fn exe_dir_for_pid(&self, pid: u64) -> Option<String> {
        for (p, process) in self.system.processes() {
            if p.as_u32() as u64 == pid {
                if let Some(exe) = process.exe() {
                    if let Some(parent) = exe.parent() {
                        return Some(parent.to_string_lossy().to_string());
                    }
                }
            }
        }
        None
    }

    pub fn snapshot(&self) -> Result<MetricsSnapshot, String> {
        let sys = &self.system;

        // Memory (inherent methods on System in 0.32)
        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();
        let mem_pct = if total_mem > 0 {
            (used_mem as f64 / total_mem as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        // CPU usage — cpu_usage() returns % since last refresh_all().
        // Average across all cores.
        let cpus = sys.cpus();
        let cpu_pct = if !cpus.is_empty() {
            cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
        } else {
            0.0
        };

        // Disks
        let disk_infos: Vec<DiskInfo> = self
            .disks
            .iter()
            .map(|d| DiskInfo {
                name: d.name().to_string_lossy().to_string(),
                file_system: d.file_system().to_string_lossy().to_string(),
                total: d.total_space(),
                available: d.available_space(),
                mount_point: d.mount_point().to_string_lossy().to_string(),
            })
            .collect();

        // Network — received()/transmitted() are deltas since last refresh_list().
        // We sum across all interfaces for total delta, then divide by the
        // elapsed refresh interval to get an accurate bytes-per-second rate.
        let secs = self.elapsed_secs();
        let mut rx_delta: u64 = 0;
        let mut tx_delta: u64 = 0;
        for (_name, net) in self.networks.iter() {
            rx_delta += net.received();
            tx_delta += net.transmitted();
        }
        let rx_rate = (rx_delta as f64 / secs) as u64;
        let tx_rate = (tx_delta as f64 / secs) as u64;

        // Processes — Process::name() returns &OsStr directly in 0.32.
        // Return a larger pool (top 30 by CPU) so the frontend can re-sort
        // by CPU/Memory/Name without re-querying. Expose the exe path for the
        // "open file location" action.
        let process_count = sys.processes().len();

        let mut procs: Vec<ProcInfo> = Vec::new();
        for (pid, process) in sys.processes() {
            let name = process.name().to_string_lossy().to_string();
            let cpu = process.cpu_usage();
            let mem = process.memory();
            let exe = process
                .exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            procs.push(ProcInfo {
                name,
                cpu,
                mem,
                pid: pid.as_u32() as u64,
                exe,
            });
        }
        procs.sort_by(|a, b| {
            b.cpu
                .partial_cmp(&a.cpu)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        procs.truncate(30);

        Ok(MetricsSnapshot {
            timestamp: chrono::Utc::now().to_rfc3339(),
            cpu_percent: cpu_pct,
            cpu_cores: cpus.len(),
            memory_total_mb: total_mem / (1024 * 1024),
            memory_used_mb: used_mem / (1024 * 1024),
            memory_percent: mem_pct,
            disk_infos,
            network_rx_bytes: rx_rate,
            network_tx_bytes: tx_rate,
            process_count,
            uptime_seconds: System::uptime(),
            os_name: System::name().unwrap_or_default(),
            cpu_brand: sys
                .cpus()
                .first()
                .map(|c| c.brand().to_string())
                .unwrap_or_default(),
            top_processes: procs,
        })
    }
}

#[derive(Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub timestamp: String,
    pub cpu_percent: f32,
    pub cpu_cores: usize,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub memory_percent: f64,
    pub disk_infos: Vec<DiskInfo>,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub process_count: usize,
    pub uptime_seconds: u64,
    pub os_name: String,
    pub cpu_brand: String,
    pub top_processes: Vec<ProcInfo>,
}

#[derive(Clone, serde::Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub file_system: String,
    pub total: u64,
    pub available: u64,
    pub mount_point: String,
}

#[derive(Clone, serde::Serialize)]
pub struct ProcInfo {
    pub name: String,
    pub cpu: f32,
    pub mem: u64,
    pub pid: u64,
    /// Absolute path to the executable (may be empty for some system processes).
    pub exe: String,
}
