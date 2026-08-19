import type { DockConfig } from "../types";

interface Props {
  dock: DockConfig | null;
  onDock: (edge: "none" | "left" | "right") => void;
  onMinimize: () => void;
  onClose: () => void;
}

export function DockBar({ dock, onDock, onMinimize, onClose }: Props) {
  const edge = dock?.edge ?? "right";
  const dotColor = edge === "none" ? "var(--muted)" : "var(--accent)";
  return (
    <div className="header">
      <span className="wordmark" data-tauri-drag-region>
        <span className="dot" style={{ background: dotColor }} />
        <span data-tauri-drag-region>Windash</span>
      </span>
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
        <span className="sep" />
        <button className="iconbtn" onClick={onMinimize} title="Minimize">
          &#8211;
        </button>
        <button className="iconbtn iconbtn-close" onClick={onClose} title="Close">
          &#10005;
        </button>
      </span>
    </div>
  );
}
