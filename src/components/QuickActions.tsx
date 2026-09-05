import { invokeCmd } from "../bridge";
import { friendlyError } from "../format";
import { IconGitHub, IconFolder, IconSearch, IconActions } from "./icons";

const REPO_URL = "https://github.com/shareef01/windash";

interface Props {
  onOpen: (url: string) => void | Promise<void>;
  onError?: (message: string, detail?: string) => void;
}

export function QuickActions({ onOpen, onError }: Props) {
  async function run(cmd: string, args: Record<string, unknown>, fallback: string) {
    try {
      await invokeCmd(cmd, args);
    } catch (e) {
      const f = friendlyError(e, fallback);
      onError?.(f.message, f.detail);
    }
  }

  return (
    <section className="section">
      <div className="section-head">
        <span className="section-title">
          <IconActions size={14} /> Quick actions
        </span>
      </div>
      <div className="actions">
        <button
          type="button"
          className="btn"
          onClick={() => {
            void Promise.resolve(onOpen(REPO_URL)).catch((e) => {
              const f = friendlyError(e, "Couldn't open GitHub.");
              onError?.(f.message, f.detail);
            });
          }}
          title="Open Windash on GitHub"
        >
          <IconGitHub size={14} /> GitHub
        </button>
        <button
          type="button"
          className="btn"
          onClick={() => run("open_explorer", { pid: null }, "Couldn't open File Explorer.")}
          title="Open File Explorer"
        >
          <IconFolder size={14} /> Files
        </button>
        <button
          type="button"
          className="btn"
          onClick={() => run("windows_search", { query: null }, "Couldn't open Windows Search.")}
          title="Open Windows Search"
        >
          <IconSearch size={14} /> Search
        </button>
      </div>
    </section>
  );
}
