import type { Settings } from "../types";
import { invokeCmd } from "../bridge";
import { friendlyError } from "../format";
import { useEffect, useRef } from "react";

interface Props {
  settings: Settings;
  onClose: () => void;
  onApply?: (s: Settings) => void;
  onError?: (message: string, detail?: string) => void;
}

export function SettingsPanel({ settings, onClose, onApply, onError }: Props) {
  const panelRef = useRef<HTMLDivElement | null>(null);
  const closeRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    closeRef.current?.focus();
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
        return;
      }
      if (e.key !== "Tab" || !panelRef.current) return;
      const focusable = panelRef.current.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  async function patch(p: Partial<Settings>) {
    try {
      const updated = await invokeCmd<Settings>("update_settings", { patch: p });
      onApply?.(updated);
    } catch (e) {
      const f = friendlyError(e, "Couldn't save settings.");
      onError?.(f.message, f.detail);
    }
  }

  return (
    <div
      ref={panelRef}
      className="popover"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-title"
    >
      <div className="popover-head">
        <span id="settings-title">Settings</span>
        <button
          ref={closeRef}
          type="button"
          className="iconbtn"
          onClick={onClose}
          title="Close"
          aria-label="Close settings"
        >
          ×
        </button>
      </div>

      <fieldset className="set-group">
        <legend>Appearance</legend>
        <label className="row">
          <span>Theme</span>
          <select
            value={settings.theme}
            onChange={(e) => patch({ theme: e.target.value as Settings["theme"] })}
            aria-label="Theme"
          >
            <option value="system">Follow Windows</option>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </label>
        <label className="row">
          <span>Windows 11 Mica</span>
          <input
            type="checkbox"
            checked={settings.mica_enabled}
            onChange={(e) => patch({ mica_enabled: e.target.checked })}
          />
        </label>
      </fieldset>

      <fieldset className="set-group">
        <legend>Monitoring</legend>
        <label className="row">
          <span>Refresh interval</span>
          <select
            value={settings.refresh_ms}
            onChange={(e) => patch({ refresh_ms: Number(e.target.value) })}
            aria-label="Refresh interval"
          >
            <option value={1000}>1 second</option>
            <option value={2000}>2 seconds</option>
            <option value={3000}>3 seconds</option>
            <option value={5000}>5 seconds</option>
            <option value={10000}>10 seconds</option>
          </select>
        </label>
        <label className="row">
          <span>Always on top</span>
          <input
            type="checkbox"
            checked={settings.always_on_top}
            onChange={(e) => patch({ always_on_top: e.target.checked })}
          />
        </label>
      </fieldset>

      <fieldset className="set-group">
        <legend>Sections</legend>
        <label className="check">
          <input
            type="checkbox"
            checked={settings.show_disks}
            onChange={(e) => patch({ show_disks: e.target.checked })}
          />
          Storage
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={settings.show_processes}
            onChange={(e) => patch({ show_processes: e.target.checked })}
          />
          Processes
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={settings.show_notes}
            onChange={(e) => patch({ show_notes: e.target.checked })}
          />
          Notes
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={settings.show_actions}
            onChange={(e) => patch({ show_actions: e.target.checked })}
          />
          Quick actions
        </label>
      </fieldset>

      <div className="popover-foot">
        Toggle window: <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>D</kbd>
        <span className="foot-sep">·</span>
        Filter: <kbd>/</kbd> or <kbd>Ctrl</kbd>+<kbd>F</kbd>
      </div>
    </div>
  );
}
