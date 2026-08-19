import type { DockConfig } from "../types";

interface Props {
  dock: DockConfig | null;
  onDock: (edge: "none" | "left" | "right") => void;
}

export function DockBar({ dock, onDock }: Props) {
  const edge = dock?.edge ?? "right";
  return (
    <div className="dockbar" data-edge={edge}>
      <span className="grip" title="Drag to move · double-click to toggle dock">
        ⠿
      </span>
      <span className="dock-label">
        {edge === "none" ? "floating" : `docked ${edge}`}
      </span>
      <span className="dock-actions">
        <button className="dockbtn" onClick={() => onDock("left")} title="Dock left">
          ◧
        </button>
        <button className="dockbtn" onClick={() => onDock("right")} title="Dock right">
          ◨
        </button>
        <button className="dockbtn" onClick={() => onDock("none")} title="Undock (float)">
          ▢
        </button>
      </span>
    </div>
  );
}
