interface Props {
  onOpen: (url: string) => void;
}

const ACTIONS: { label: string; url: string }[] = [
  { label: "Open GitHub", url: "https://github.com" },
  { label: "Open Files", url: "file:///" },
  { label: "Web Search", url: "https://duckduckgo.com" },
];

export function QuickActions({ onOpen }: Props) {
  return (
    <section className="card">
      <h3>Quick actions</h3>
      <div className="actions">
        {ACTIONS.map((a) => (
          <button key={a.label} className="btn" onClick={() => onOpen(a.url)}>
            {a.label}
          </button>
        ))}
      </div>
    </section>
  );
}
