import type { DockConfig } from "../types";

interface Props {
  dock: DockConfig | null;
  onDock: (edge: "none" | "left" | "right") => void;
}

export function DockBar({ dock, onDock }: Props) {
  const edge = dock?.edge ?? "right";
  const dotColor = edge === "none" ? "var(--muted)" : "var(--accent)";
  return (
    <div className="header">
      <span className="wordmark">
        <span className="dot" style={{ background: dotColor }} />
        Windash
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
      </span>
    </div>
  );
}
