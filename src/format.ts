/** User-facing formatters and error mapping. Keep units consistent across the UI. */

export function fmtUptime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "—";
  const s = Math.floor(seconds);
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function fmtUpdated(iso: string, nowMs = Date.now()): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "";
  const sec = Math.max(0, Math.round((nowMs - t) / 1000));
  if (sec < 2) return "just now";
  if (sec < 60) return `${sec}s ago`;
  const min = Math.round(sec / 60);
  if (min < 60) return `${min}m ago`;
  return fmtUptime(sec);
}

/** Compact process/memory size with stable units. */
export function fmtMem(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "0 B";
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
  if (bytes >= 1024 * 1024) return `${Math.round(bytes / 1024 / 1024)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${Math.round(bytes)} B`;
}

export function diskLabel(mountPoint: string, name?: string): string {
  const mp = (mountPoint || "").replace(/[\\/]+$/, "");
  const drive = mp.match(/^([A-Za-z]:)/);
  if (drive) return drive[1].toUpperCase();
  if (mp) return mp;
  return name || "Disk";
}

export function filterProcs<T extends { name: string; exe?: string }>(
  procs: T[],
  query: string
): T[] {
  const q = query.trim().toLowerCase();
  if (!q) return procs;
  return procs.filter(
    (p) => p.name.toLowerCase().includes(q) || (p.exe ?? "").toLowerCase().includes(q)
  );
}

export interface FriendlyError {
  message: string;
  detail: string;
}

export function friendlyError(err: unknown, fallback = "Something went wrong."): FriendlyError {
  const detail = err instanceof Error ? err.message : String(err ?? "");
  const d = detail.toLowerCase();

  if (d.includes("protected windows") || d.includes("cannot be terminated")) {
    return { message: "Windows is protecting that process, so it can't be ended.", detail };
  }
  if (d.includes("cannot terminate windash") || d.includes("within itself")) {
    return { message: "Windash can't end its own process.", detail };
  }
  if (d.includes("unable to terminate") || d.includes("taskkill") || d.includes("administrator")) {
    return {
      message: "Couldn't end that process. You may need administrator permission.",
      detail,
    };
  }
  if (d.includes("could not locate") || d.includes("file location") || d.includes("executable")) {
    return { message: "Couldn't find that process's file location.", detail };
  }
  if (d.includes("clipboard")) {
    return { message: "Couldn't copy to the clipboard.", detail };
  }
  if (d.includes("note id") || d.includes("not found")) {
    return { message: "That note was already removed.", detail };
  }
  if (d.includes("too long") || d.includes("500")) {
    return { message: "Notes are limited to 500 characters.", detail };
  }
  if (d.includes("too many notes")) {
    return { message: "Too many notes. Delete a few and try again.", detail };
  }
  if (d.includes("explorer") || d.includes("open explorer")) {
    return { message: "Couldn't open File Explorer.", detail };
  }
  if (d.includes("windows search") || d.includes("search-ms")) {
    return { message: "Couldn't open Windows Search.", detail };
  }
  if (d.includes("parse") || d.includes("json") || d.includes("corrupted")) {
    return { message: "Saved data was repaired because it was unreadable.", detail };
  }
  if (d.includes("get_metrics") || d.includes("refresh") || d.includes("sysinfo")) {
    return { message: "Couldn't refresh system metrics. Retrying…", detail };
  }
  if (d.includes("webview") || d.includes("ipc") || d.includes("invoke")) {
    return { message: fallback, detail };
  }
  if (!detail || detail === "undefined" || detail === "null") {
    return { message: fallback, detail: "" };
  }
  return { message: fallback, detail };
}
