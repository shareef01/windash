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
  const remaining = 500 - [...text].length;
  return (
    <section className="section">
      <div className="section-head">
        <span className="section-title">
          <IconNotes size={14} /> Notes
        </span>
      </div>
      <div className="note-input">
        <input
          value={text}
          placeholder="Write a quick note"
          onChange={(e) => onText(e.target.value.slice(0, 500))}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              onAdd();
            }
          }}
          aria-label="New note"
          maxLength={500}
        />
        <button
          type="button"
          className="btn btn-add"
          onClick={onAdd}
          title="Add note"
          aria-label="Add note"
          disabled={!text.trim()}
        >
          <IconPlus size={14} />
        </button>
      </div>
      {text.length > 400 && (
        <div className="note-limit" aria-live="polite">
          {remaining} characters left
        </div>
      )}
      {notes.length === 0 ? (
        <div className="empty">No notes yet. Press Enter to save one.</div>
      ) : (
        <ul className="notes">
          {notes.map((n) => (
            <li key={n.id}>
              <span className="note-text" title={n.text}>
                {n.text}
              </span>
              <button
                type="button"
                className="x"
                onClick={() => onDelete(n.id)}
                title="Delete note"
                aria-label={`Delete note: ${n.text.slice(0, 40)}`}
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
