// Centralized design tokens + helpers (the "design system").
// Single source of truth for semantic colors so every component reads the same
// palette instead of inlining threshold logic.

export const colors = {
  accent: "var(--accent)",
  amber: "var(--amber)",
  red: "var(--red)",
  green: "var(--green)",
  muted: "var(--muted)",
} as const;

export type UsageTone = "normal" | "elevated" | "high";

/**
 * Semantic color for a 0..100 utilization value.
 * Stay calm through ordinary load; warn only when the machine is actually busy.
 */
export function usageTone(pct: number): UsageTone {
  if (pct >= 90) return "high";
  if (pct >= 75) return "elevated";
  return "normal";
}

export function usageColor(pct: number): string {
  const t = usageTone(pct);
  if (t === "high") return colors.red;
  if (t === "elevated") return colors.amber;
  return colors.accent;
}

/**
 * Color for a process' CPU bar. Process CPU is per-logical-core, so a single
 * busy core reads 100%; keep the warm threshold higher than the resource tiles.
 */
export function cpuColor(cpu: number): string {
  if (cpu >= 70) return colors.red;
  if (cpu >= 30) return colors.amber;
  return colors.accent;
}

/** Format a byte count as a compact rate string, e.g. "60.1 KB/s". */
export function fmtRate(bytesPerSec: number): string {
  if (bytesPerSec >= 1024 * 1024) return `${(bytesPerSec / 1024 / 1024).toFixed(1)} MB/s`;
  if (bytesPerSec >= 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  return `${bytesPerSec} B/s`;
}

/** Format a byte count as a size, e.g. "205 GB" / "1.2 TB". */
export function fmtSize(bytes: number): string {
  const gb = bytes / 1024 / 1024 / 1024;
  if (gb >= 1024) return `${(gb / 1024).toFixed(1)} TB`;
  if (gb >= 1) return `${gb.toFixed(0)} GB`;
  return `${Math.max(0, Math.round(bytes / 1024 / 1024))} MB`;
}
