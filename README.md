# ⚡ Windash

A personal **Windows system dashboard** — a tiny, tray-resident desktop app that shows
live system stats, quick actions, and a persistent notes strip. Built as a portfolio piece
to learn **Rust** while keeping a fast **React** UI.

> Stack: **Tauri 2** (Rust core) · **React 18 + TypeScript** (UI) · **Vite** (bundler)
> Metrics via [`sysinfo`](https://crates.io/crates/sysinfo); notes persisted as JSON in AppData.

## Features

- **Live CPU %** gauge with color thresholds
- **Memory** usage bar (used / total)
- **Network** up/down throughput with a sparkline (last ~60 ticks)
- **Top processes** by CPU
- **Quick actions** — open GitHub, file explorer, web search
- **Notes strip** — add/delete notes, persisted across restarts
- **Dockable window** — docks to the left/right screen edge as an always-on-top
  sidebar (or floats freely); state persists in AppData
- **System tray** — left-click to toggle, menu to show / hide / quit

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Node.js](https://nodejs.org/) ≥ 18
- Windows + MSVC C++ build tools (via Visual Studio Build Tools)

### Run in dev

```bash
npm install
npm run tauri dev
```

### Build the installer / `.exe`

```bash
npm run tauri build
```

The NSIS installer (and the standalone `.exe`) lands in `src-tauri/target/release/bundle/`.

## Project layout

```
windash/
├── src/                 # React + TypeScript frontend
│   ├── App.tsx          # dashboard shell + polling loop
│   ├── components/      # Gauge, MemBar, NetSpark, QuickActions, NotesStrip
│   ├── types.ts         # shared TS types mirroring the Rust commands
│   └── styles.css       # dark theme
└── src-tauri/           # Rust backend (Tauri)
    ├── src/lib.rs       # commands + tray setup
    ├── src/metrics.rs   # sysinfo wrapper
    └── src/notes.rs     # JSON notes store
```

## How it works

The frontend polls the Rust backend via Tauri's `invoke` bridge every 2 seconds.
`get_metrics` returns a snapshot (CPU %, memory, per-disk, network deltas, top processes);
notes are read/written through `get_notes` / `add_note` / `delete_note` and stored as
`windash-notes.json` in the app's AppData directory.

## License

MIT
