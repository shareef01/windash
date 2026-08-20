import { invoke } from "@tauri-apps/api/core";
import { IconGitHub, IconFolder, IconSearch } from "./icons";

const REPO_URL = "https://github.com/shareef01/windash"; // single config constant

interface Props {
  onOpen: (url: string) => void;
  onError?: (e: string) => void;
}

export function QuickActions({ onOpen, onError }: Props) {
  async function run(cmd: string, args: Record<string, unknown>) {
    try {
      await invoke(cmd, args);
    } catch (e) {
      if (onError) onError(String(e));
      else alert(String(e));
    }
  }

  return (
    <section className="section">
      <div className="section-head">
        <span className="section-title">Quick actions</span>
      </div>
      <div className="actions">
        <button
          className="btn"
          onClick={() => onOpen(REPO_URL)}
          title="Open Windash on GitHub"
        >
          <IconGitHub size={14} /> GitHub
        </button>
        <button className="btn" onClick={() => run("open_explorer", { pid: null })} title="Open File Explorer">
          <IconFolder size={14} /> Files
        </button>
        <button className="btn" onClick={() => run("windows_search", { query: null })} title="Open Windows Search">
          <IconSearch size={14} /> Search
        </button>
      </div>
    </section>
  );
}
