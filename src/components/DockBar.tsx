import type { DockConfig } from "../types";
import { invoke } from "@tauri-apps/api/core";
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

interface Props {
  dock: DockConfig | null;
  paused: boolean;
  onTogglePause: () => void;
  onDock: (edge: "none" | "left" | "right") => void;
  onSettings: () => void;
}

export function DockBar({ dock, paused, onTogglePause, onDock, onSettings }: Props) {
  const edge = dock?.edge ?? "right";
  return (
    <div className="header">
      <div className="wordmark" data-tauri-drag-region>
        <span className="logo">
          <span className="logo-glyph" />
        </span>
        <span>Windash</span>
      </div>
      <span className="dock-controls">
        <button
          className="iconbtn"
          onClick={onTogglePause}
          title={paused ? "Resume monitoring" : "Pause monitoring"}
          aria-label={paused ? "Resume monitoring" : "Pause monitoring"}
        >
          {paused ? <IconPlay size={15} /> : <IconPause size={15} />}
        </button>
        <button
          className={"iconbtn" + (edge === "left" ? " active" : "")}
          onClick={() => onDock("left")}
          title="Dock left"
          aria-label="Dock left"
        >
          <IconDockLeft size={15} />
        </button>
        <button
          className={"iconbtn" + (edge === "right" ? " active" : "")}
          onClick={() => onDock("right")}
          title="Dock right"
          aria-label="Dock right"
        >
          <IconDockRight size={15} />
        </button>
        <button
          className={"iconbtn" + (edge === "none" ? " active" : "")}
          onClick={() => onDock("none")}
          title="Float window"
          aria-label="Float window"
        >
          <IconFloat size={15} />
        </button>
        <button className="iconbtn" onClick={onSettings} title="Settings" aria-label="Settings">
          <IconSettings size={15} />
        </button>
        <span className="sep" />
        <button
          className="iconbtn"
          onClick={() => invoke("window_minimize")}
          title="Minimize"
          aria-label="Minimize"
        >
          <IconMinimize size={15} />
        </button>
        <button
          className="iconbtn iconbtn-close"
          onClick={() => invoke("window_hide")}
          title="Close to tray"
          aria-label="Close to tray"
        >
          <IconClose size={15} />
        </button>
      </span>
    </div>
  );
}
