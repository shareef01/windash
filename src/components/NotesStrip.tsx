import type { Note } from "../types";
import { IconNotes, IconPlus } from "./icons";

interface Props {
  notes: Note[];
  text: string;
  onText: (v: string) => void;
  onAdd: () => void;
  onDelete: (id: number) => void;
}

export function NotesStrip({ notes, text, onText, onAdd, onDelete }: Props) {
  return (
    <section className="section">
      <div className="section-head">
        <span className="section-title">
          <IconNotes size={13} /> Notes
        </span>
      </div>
      <div className="note-input">
        <input
          value={text}
          placeholder="Write a quick note…"
          onChange={(e) => onText(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && onAdd()}
          aria-label="New note"
        />
        <button className="btn btn-add" onClick={onAdd} title="Add note" aria-label="Add note">
          <IconPlus size={14} />
        </button>
      </div>
      {notes.length === 0 ? (
        <div className="empty">No notes yet.</div>
      ) : (
        <ul className="notes">
          {notes.map((n) => (
            <li key={n.id}>
              <span className="note-text">{n.text}</span>
              <button
                className="x"
                onClick={() => onDelete(n.id)}
                title="delete"
                aria-label="Delete note"
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
