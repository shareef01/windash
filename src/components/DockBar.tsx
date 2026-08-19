import type { DockConfig } from "../types";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  dock: DockConfig | null;
  onDock: (edge: "none" | "left" | "right") => void;
  onSettings: () => void;
}

export function DockBar({ dock, onDock, onSettings }: Props) {
  const edge = dock?.edge ?? "right";
  const dotColor = edge === "none" ? "var(--muted)" : "var(--accent)";
  return (
    <div className="header">
      <div className="wordmark" data-tauri-drag-region>
        <span className="dot" style={{ background: dotColor }} />
        <span>Windash</span>
      </div>
      <span className="dock-controls">
        <button className="iconbtn" onClick={() => onDock("left")} title="Dock left">
          ◧
        </button>
        <button className="iconbtn" onClick={() => onDock("right")} title="Dock right">
          ◨
        </button>
        <button className="iconbtn" onClick={() => onDock("none")} title="Float">
          ▢
        </button>
        <button className="iconbtn" onClick={onSettings} title="Settings">
          ⚙
        </button>
        <span className="sep" />
        <button
          className="iconbtn"
          onClick={() => invoke("window_minimize")}
          title="Minimize"
        >
          &#8211;
        </button>
        <button
          className="iconbtn iconbtn-close"
          onClick={() => invoke("window_hide")}
          title="Close to tray"
        >
          &#10005;
        </button>
      </span>
    </div>
  );
}
