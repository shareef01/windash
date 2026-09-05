import type { DockConfig } from "../types";
import { invokeCmd } from "../bridge";
import {
  IconDockLeft,
  IconDockRight,
  IconFloat,
  IconSettings,
  IconMinimize,
  IconClose,
  IconPause,
  IconPlay,
} from "./icons";
import { forwardRef } from "react";

export type MonitorStatus = "live" | "paused" | "loading" | "stale" | "error";

interface Props {
  dock: DockConfig | null;
  paused: boolean;
  status: MonitorStatus;
  settingsOpen: boolean;
  onTogglePause: () => void;
  onDock: (edge: "none" | "left" | "right") => void;
  onSettings: () => void;
}

export const DockBar = forwardRef<HTMLButtonElement, Props>(function DockBar(
  { dock, paused, status, settingsOpen, onTogglePause, onDock, onSettings },
  settingsRef
) {
  const edge = dock?.edge ?? "none";
  const statusLabel =
    status === "paused"
      ? "Paused"
      : status === "loading"
        ? "Loading"
        : status === "error"
          ? "Error"
          : status === "stale"
            ? "Stale"
            : "Live";

  return (
    <header className="header">
      <div className="titlebar-main" data-tauri-drag-region>
        <div className="wordmark">
          <span className="logo" aria-hidden="true">
            <span className="logo-glyph" />
          </span>
          <span className="wordmark-text">Windash</span>
        </div>
        <span className={`live-pill live-${status}`} title={statusLabel} aria-live="polite">
          <span className="status-dot" aria-hidden="true" />
          <span className="status-label">{statusLabel}</span>
        </span>
      </div>
      <div className="dock-controls">
        <button
          type="button"
          className={"iconbtn" + (paused ? " active" : "")}
          onClick={onTogglePause}
          title={paused ? "Resume monitoring" : "Pause monitoring"}
          aria-label={paused ? "Resume monitoring" : "Pause monitoring"}
          aria-pressed={paused}
        >
          {paused ? <IconPlay size={14} /> : <IconPause size={14} />}
        </button>
        <div className="seg" role="group" aria-label="Window placement">
          <button
            type="button"
            className={"seg-btn" + (edge === "left" ? " active" : "")}
            onClick={() => onDock("left")}
            title="Dock left"
            aria-label="Dock left"
            aria-pressed={edge === "left"}
          >
            <IconDockLeft size={14} />
          </button>
          <button
            type="button"
            className={"seg-btn" + (edge === "right" ? " active" : "")}
            onClick={() => onDock("right")}
            title="Dock right"
            aria-label="Dock right"
            aria-pressed={edge === "right"}
          >
            <IconDockRight size={14} />
          </button>
          <button
            type="button"
            className={"seg-btn" + (edge === "none" ? " active" : "")}
            onClick={() => onDock("none")}
            title="Float window"
            aria-label="Float window"
            aria-pressed={edge === "none"}
          >
            <IconFloat size={14} />
          </button>
        </div>
        <button
          ref={settingsRef}
          type="button"
          className={"iconbtn" + (settingsOpen ? " active" : "")}
          onClick={onSettings}
          title="Settings"
          aria-label="Settings"
          aria-haspopup="dialog"
          aria-expanded={settingsOpen}
        >
          <IconSettings size={14} />
        </button>
        <span className="sep" />
        <button
          type="button"
          className="iconbtn"
          onClick={() => invokeCmd("window_minimize")}
          title="Minimize"
          aria-label="Minimize"
        >
          <IconMinimize size={14} />
        </button>
        <button
          type="button"
          className="iconbtn iconbtn-close"
          onClick={() => invokeCmd("window_hide")}
          title="Close to tray"
          aria-label="Close to tray"
        >
          <IconClose size={14} />
        </button>
      </div>
    </header>
  );
});
