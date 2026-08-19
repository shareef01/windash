import type { Note } from "../types";

interface Props {
  notes: Note[];
  text: string;
  onText: (v: string) => void;
  onAdd: () => void;
  onDelete: (id: number) => void;
}

export function NotesStrip({ notes, text, onText, onAdd, onDelete }: Props) {
  return (
    <section className="card">
      <h3>Notes</h3>
      <div className="note-input">
        <input
          value={text}
          placeholder="Write a quick note…"
          onChange={(e) => onText(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && onAdd()}
        />
        <button className="btn" onClick={onAdd}>
          Add
        </button>
      </div>
      <ul className="notes">
        {notes.map((n) => (
          <li key={n.id}>
            <span>{n.text}</span>
            <button className="x" onClick={() => onDelete(n.id)} title="delete">
              ×
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
