import { useCallback, useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { MetricsSnapshot, Note, DockConfig, Settings, SortKey, NetSample } from "./types";
import { fmtSize } from "./theme";
import { fmtUptime, fmtUpdated, friendlyError } from "./format";
import { invokeCmd, listenSafe, isTauri } from "./bridge";
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
  ErrorBanner,
} from "./components";
import { IconCopy } from "./components/icons";
import type { MonitorStatus } from "./components/DockBar";

interface Banner {
  message: string;
  detail?: string;
  retry?: () => void;
}

export default function App() {
  const [metrics, setMetrics] = useState<MetricsSnapshot | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [stream, setStream] = useState<NetSample[]>([]);
  const [noteText, setNoteText] = useState("");
  const [dock, setDockState] = useState<DockConfig | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [sortKey, setSortKey] = useState<SortKey>("cpu");
  const [selectedPid, setSelectedPid] = useState<number | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [banner, setBanner] = useState<Banner | null>(null);
  const [width, setWidth] = useState<number>(typeof window !== "undefined" ? window.innerWidth : 390);
  const [systemTheme, setSystemTheme] = useState<"dark" | "light">("dark");
  const [paused, setPaused] = useState(false);
  const focusFilterRef = useRef<(() => void) | null>(null);
  const settingsRef = useRef<Settings | null>(null);
  const settingsBtnRef = useRef<HTMLButtonElement | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const toastTimer = useRef<number | null>(null);
  const refreshSeq = useRef(0);

  settingsRef.current = settings;

  useEffect(() => {
    return () => {
      if (toastTimer.current) window.clearTimeout(toastTimer.current);
    };
  }, []);

  const layout: "compact" | "normal" | "expanded" =
    width < 360 ? "compact" : width > 560 ? "expanded" : "normal";

  const effectiveTheme: "dark" | "light" =
    settings?.theme === "system" || !settings ? systemTheme : settings.theme;

  const status: MonitorStatus = paused
    ? "paused"
    : !metrics
      ? "loading"
      : banner?.retry
        ? "error"
        : "live";

  function flash(message: string) {
    setToast(message);
    if (toastTimer.current) window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 1600);
  }

  function showError(message: string, detail?: string, retry?: () => void) {
    setBanner({ message, detail, retry });
  }

  const refresh = useCallback(async () => {
    const seq = ++refreshSeq.current;
    try {
      const m = await invokeCmd<MetricsSnapshot>("get_metrics", { sort_by: sortKey });
      if (seq !== refreshSeq.current) return;
      setMetrics(m);
      setStream((s) => {
        const next = { rx: m.network_rx_bytes, tx: m.network_tx_bytes };
        if (s.length === 0 && !isTauri()) {
          const seed = Array.from({ length: 28 }, (_, i) => ({
            rx: Math.round(m.network_rx_bytes * (0.35 + 0.65 * Math.abs(Math.sin(i / 3.2)))),
            tx: Math.round(m.network_tx_bytes * (0.25 + 0.75 * Math.abs(Math.cos(i / 4.1)))),
          }));
          return [...seed, next];
        }
        return [...s.slice(-59), next];
      });
      setBanner((b) => (b?.retry ? null : b));
    } catch (e) {
      if (seq !== refreshSeq.current) return;
      const f = friendlyError(e, "Couldn't refresh system metrics. Last readings are still shown.");
      showError(f.message, f.detail, () => {
        void refresh();
      });
    }
  }, [sortKey]);

  async function loadNotes() {
    try {
      setNotes(await invokeCmd<Note[]>("get_notes"));
    } catch (e) {
      const f = friendlyError(e, "Couldn't load notes.");
      showError(f.message, f.detail);
    }
  }

  async function loadSettings() {
    try {
      const s = await invokeCmd<Settings>("get_settings");
      setSettings(s);
      if (s.sort_key === "mem" || s.sort_key === "name" || s.sort_key === "cpu") setSortKey(s.sort_key);
    } catch (e) {
      const f = friendlyError(e, "Couldn't load settings.");
      showError(f.message, f.detail);
    }
  }

  async function loadSystemTheme() {
    try {
      const t = await invokeCmd<string>("get_system_theme");
      if (t === "light" || t === "dark") setSystemTheme(t);
    } catch {
      /* keep current default */
    }
  }

  async function loadDock() {
    try {
      const cfg = await invokeCmd<DockConfig>("get_dock");
      setDockState(cfg);
      await invokeCmd("apply_immersive", { dark: effectiveTheme === "dark" });
      await invokeCmd<DockConfig>("set_dock", { edge: cfg.edge });
    } catch (e) {
      const f = friendlyError(e, "Couldn't restore window position.");
      showError(f.message, f.detail);
    }
  }

  async function setDock(edge: "none" | "left" | "right") {
    try {
      setDockState(await invokeCmd<DockConfig>("set_dock", { edge }));
    } catch (e) {
      const f = friendlyError(e, "Couldn't change window docking.");
      showError(f.message, f.detail);
    }
  }

  useEffect(() => {
    void loadNotes();
    void loadSettings();
    void loadSystemTheme();
    void loadDock();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const refreshMs = settings?.refresh_ms ?? 2000;

  useEffect(() => {
    if (paused) return;
    let cancelled = false;
    let timeoutId = 0;
    const tick = async () => {
      await refresh();
      if (cancelled) return;
      timeoutId = window.setTimeout(tick, refreshMs);
    };
    void tick();
    return () => {
      cancelled = true;
      window.clearTimeout(timeoutId);
    };
  }, [paused, sortKey, refreshMs, refresh]);

  useEffect(() => {
    let isMounted = true;
    const unlistens: Array<() => void> = [];

    listenSafe<{ width: number; height: number }>("windash://resized", (payload) => {
      setWidth(payload.width);
    }).then((u) => {
      if (isMounted) unlistens.push(u);
      else u();
    });

    listenSafe<string>("windash://dock", (payload) => {
      setDockState((prev) =>
        prev ? { ...prev, edge: payload as DockConfig["edge"] } : prev
      );
    }).then((u) => {
      if (isMounted) unlistens.push(u);
      else u();
    });

    listenSafe<unknown>("windash://focused", () => {
      if (settingsRef.current?.theme === "system" || !settingsRef.current) {
        void loadSystemTheme();
      }
    }).then((u) => {
      if (isMounted) unlistens.push(u);
      else u();
    });

    listenSafe<string>("windash://theme", (payload) => {
      if (payload === "light" || payload === "dark") {
        setSystemTheme(payload);
      }
    }).then((u) => {
      if (isMounted) unlistens.push(u);
      else u();
    });

    const onResize = () => {
      if (!isTauri()) setWidth(window.innerWidth);
    };
    window.addEventListener("resize", onResize);

    return () => {
      isMounted = false;
      unlistens.forEach((u) => u());
      window.removeEventListener("resize", onResize);
    };
  }, []);

  useEffect(() => {
    if (settings?.theme !== "system") return;
    void loadSystemTheme();
  }, [settings?.theme]);

  useEffect(() => {
    document.documentElement.dataset.theme = effectiveTheme;
    void invokeCmd("apply_immersive", { dark: effectiveTheme === "dark" }).catch(() => {});
  }, [effectiveTheme, settings?.mica_enabled]);

  useEffect(() => {
    if (!settings) return;
    if (settings.sort_key === sortKey) return;
    invokeCmd("update_settings", { patch: { sort_key: sortKey } }).catch(() => {});
  }, [sortKey, settings]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const el = document.activeElement;
      const typing =
        el instanceof HTMLInputElement ||
        el instanceof HTMLTextAreaElement ||
        (el as HTMLElement | null)?.isContentEditable;
      if ((e.key === "/" && !typing) || ((e.key === "f" || e.key === "F") && e.ctrlKey && !typing)) {
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
      await invokeCmd("add_note", { text: t });
      setNoteText("");
      await loadNotes();
    } catch (e) {
      const f = friendlyError(e, "Couldn't save that note.");
      showError(f.message, f.detail);
    }
  }

  async function delNote(id: number) {
    try {
      await invokeCmd("delete_note", { id });
      await loadNotes();
    } catch (e) {
      const f = friendlyError(e, "Couldn't delete that note.");
      showError(f.message, f.detail);
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
      flash("Copied system summary");
    } catch {
      showError("Couldn't copy to the clipboard.");
    }
  }

  function closeSettings() {
    setShowSettings(false);
    window.setTimeout(() => settingsBtnRef.current?.focus(), 0);
  }

  const statusText = toast
    ? toast
    : paused
      ? metrics
        ? `Paused · last update ${fmtUpdated(metrics.timestamp)}`
        : "Paused"
      : metrics
        ? `${metrics.os_name || "Windows"} · ${metrics.process_count} processes · up ${fmtUptime(metrics.uptime_seconds)}`
        : "Collecting system metrics…";

  return (
    <div className={`app layout-${layout}`} data-theme={effectiveTheme}>
      <DockBar
        ref={settingsBtnRef}
        dock={dock}
        paused={paused}
        status={status}
        settingsOpen={showSettings}
        onTogglePause={() => setPaused((p) => !p)}
        onDock={setDock}
        onSettings={() => (showSettings ? closeSettings() : setShowSettings(true))}
      />

      <div className="dashboard">
      {showSettings && settings && (
        <SettingsPanel
          settings={settings}
          onClose={closeSettings}
          onApply={(s) => setSettings(s)}
          onError={(message, detail) => showError(message, detail)}
        />
      )}

      {banner && (
        <ErrorBanner
          message={banner.message}
          detail={banner.detail}
          onDismiss={() => setBanner(null)}
          onRetry={banner.retry}
        />
      )}

      <div className="stats-grid">
        <Gauge value={metrics?.cpu_percent ?? null} cores={metrics?.cpu_cores} />
        <MemBar
          usedMb={metrics?.memory_used_mb ?? null}
          totalMb={metrics?.memory_total_mb ?? null}
          pct={metrics?.memory_percent ?? null}
        />
      </div>

      <NetSpark
        rx={metrics?.network_rx_bytes ?? null}
        tx={metrics?.network_tx_bytes ?? null}
        stream={stream}
        loading={!metrics}
      />

      {settings?.show_disks !== false && (
        <DiskList disks={metrics?.disk_infos ?? []} loading={!metrics} />
      )}

      {settings?.show_processes !== false && (
        <ProcList
          procs={metrics?.top_processes ?? []}
          sortKey={sortKey}
          onSort={setSortKey}
          selectedPid={selectedPid}
          onSelect={setSelectedPid}
          onFocusFilter={(fn) => {
            focusFilterRef.current = fn;
          }}
          onError={(message, detail) => showError(message, detail)}
          onToast={flash}
          onProcessEnded={refresh}
          loading={!metrics}
        />
      )}

      <div className="secondary-row">
        {settings?.show_actions !== false && (
          <QuickActions
            onOpen={async (url) => {
              if (isTauri()) await openUrl(url);
              else window.open(url, "_blank", "noopener,noreferrer");
            }}
            onError={(message, detail) => showError(message, detail)}
          />
        )}

        {settings?.show_notes !== false && (
          <NotesStrip
            notes={notes}
            text={noteText}
            onText={setNoteText}
            onAdd={addNote}
            onDelete={delNote}
          />
        )}
      </div>
      </div>

      <div className="statusbar">
        <span className={`status-dot live-${status}`} />
        <span className="status-copy-text" aria-live="polite">
          {statusText}
        </span>
        <span className="status-actions">
          <button
            type="button"
            className="status-copy"
            onClick={copySummary}
            title="Copy system summary"
            aria-label="Copy system summary"
            disabled={!metrics}
          >
            <IconCopy size={13} />
          </button>
          <span className="status-time">
            {paused || !metrics ? "" : `updated ${fmtUpdated(metrics.timestamp)}`}
          </span>
        </span>
      </div>
    </div>
  );
}
