import { invoke } from "@tauri-apps/api/core";

const REPO_URL = "https://github.com/shareef01/windash"; // single config constant

interface Props {
  onOpen: (url: string) => void;
}

export function QuickActions({ onOpen }: Props) {
  async function files() {
    try {
      await invoke("open_explorer", { pid: null });
    } catch (e) {
      alert(String(e));
    }
  }

  async function search() {
    try {
      await invoke("windows_search", { query: null });
    } catch (e) {
      alert(String(e));
    }
  }

  return (
    <section className="section">
      <div className="section-head">
        <span>QUICK ACTIONS</span>
      </div>
      <div className="actions">
        <button className="btn" onClick={() => onOpen(REPO_URL)} title="Open Windash on GitHub">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden>
            <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
          </svg>
          GitHub
        </button>
        <button className="btn" onClick={files} title="Open File Explorer">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden>
            <path d="M1.5 2A1.5 1.5 0 0 0 0 3.5v9A1.5 1.5 0 0 0 1.5 14h13a1.5 1.5 0 0 0 1.5-1.5v-7A1.5 1.5 0 0 0 14.5 4H7.6L6.3 2.7A1.5 1.5 0 0 0 5.2 2H1.5Z" />
          </svg>
          Files
        </button>
        <button className="btn" onClick={search} title="Open Windows Search">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden>
            <path d="M11.74 10.34a6 6 0 1 0-1.4 1.4l3.2 3.2a1 1 0 0 0 1.42-1.42l-3.22-3.18ZM2 6.5a4.5 4.5 0 1 1 9 0 4.5 4.5 0 0 1-9 0Z" />
          </svg>
          Search
        </button>
      </div>
    </section>
  );
}
