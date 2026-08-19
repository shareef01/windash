import type { Settings } from "../types";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  settings: Settings;
  onClose: () => void;
}

export function SettingsPanel({ settings, onClose }: Props) {
  async function patch(p: Partial<Settings>) {
    try {
      await invoke("update_settings", { patch: p });
    } catch (e) {
      alert(String(e));
    }
  }

  return (
    <div className="popover">
      <div className="popover-head">
        <span>Settings</span>
        <button className="iconbtn" onClick={onClose} title="Close">
          &#10005;
        </button>
      </div>

      <label className="row">
        <span>Always on top</span>
        <input
          type="checkbox"
          checked={settings.always_on_top}
          onChange={(e) => patch({ always_on_top: e.target.checked })}
        />
      </label>

      <label className="row">
        <span>Refresh interval</span>
        <select
          value={settings.refresh_ms}
          onChange={(e) => patch({ refresh_ms: Number(e.target.value) })}
        >
          <option value={1000}>1s</option>
          <option value={2000}>2s</option>
          <option value={3000}>3s</option>
          <option value={5000}>5s</option>
        </select>
      </label>

      <label className="row">
        <span>Theme</span>
        <select
          value={settings.theme}
          onChange={(e) => patch({ theme: e.target.value as Settings["theme"] })}
        >
          <option value="system">Follow Windows</option>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </label>

      <div className="row col">
        <span>Show sections</span>
        <label>
          <input
            type="checkbox"
            checked={settings.show_disks}
            onChange={(e) => patch({ show_disks: e.target.checked })}
          />{" "}
          Disks
        </label>
        <label>
          <input
            type="checkbox"
            checked={settings.show_processes}
            onChange={(e) => patch({ show_processes: e.target.checked })}
          />{" "}
          Processes
        </label>
        <label>
          <input
            type="checkbox"
            checked={settings.show_notes}
            onChange={(e) => patch({ show_notes: e.target.checked })}
          />{" "}
          Notes
        </label>
        <label>
          <input
            type="checkbox"
            checked={settings.show_actions}
            onChange={(e) => patch({ show_actions: e.target.checked })}
          />{" "}
          Quick actions
        </label>
      </div>
    </div>
  );
}
