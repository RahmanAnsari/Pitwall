---
inclusion: always
---

# PitWall Project Steering

## Overview

PitWall is an F1 24 telemetry capture, storage, and live dashboard tool. It has two parts:

- **pitwall/** — Rust backend (UDP receiver, session state, Parquet storage, WebSocket server)
- **pitwall-ui/** — React + TypeScript frontend (Vite, connects via WebSocket)

## Data Flow

```
F1 24 Game → UDP packets → receiver.rs → SessionState → WsBroadcaster → WebSocket → React UI
                                              ↓
                                     storage.rs → Parquet files
```

## F1 24 UDP Conventions

- Wheel array ordering in packets: `[0]=RL, [1]=RR, [2]=FL, [3]=FR`
- Sector times use split format: `minutes_part * 60000 + ms_part` (both fields needed)
- Sector time fields in LapDataCar reflect the *current* lap's completed sectors — they reset to 0 on lap transition. Cache them before the lap number increments.
- Session types (F1 24 spec):
  - 0=Unknown, 1=P1, 2=P2, 3=P3, 4=Short Practice
  - 5=Q1, 6=Q2, 7=Q3, 8=Short Quali, 9=OSQ
  - 10=Sprint Shootout 1, 11=Sprint Shootout 2, 12=Sprint Shootout 3
  - 13=Short Sprint Shootout, 14=One-Shot Sprint Shootout
  - 15=Race, 16=Race 2, 17=Race 3, 18=Time Trial
- Packet structs use `#[repr(C, packed)]` + `bytemuck` for zero-copy parsing. Never add padding or reorder fields.

## Rust Patterns

- **Zero-copy parsing**: Use `bytemuck::from_bytes` on raw UDP buffers. Structs must derive `Pod, Zeroable`.
- **Session state machine** (`session.rs`): `SessionState` accumulates samples, detects lap completions, tracks live sector times. It's the single source of truth during capture.
- **Sample merging**: Multiple packet types arrive per frame. `get_sample(timestamp, frame)` returns the current sample; when frame ID changes, the previous sample is flushed to the telemetry buffer.
- **WebSocket broadcast** (`live_server.rs`): `WsBroadcaster` uses `tokio::sync::broadcast`. Only serializes if clients are connected. `LiveFrame` is the JSON payload sent to the UI.
- **Error handling**: Use `anyhow` for application errors, `thiserror` for library-style errors. Log with `tracing`.
- **Async**: Tokio runtime. The receiver loop and WebSocket server run concurrently via `tokio::spawn`.

## React Patterns

- **WebSocket hook** (`useTelemetry.ts`): Auto-connects, auto-reconnects on disconnect (2s delay). Parses incoming JSON into `LiveFrame`.
- **Component structure**: Each panel is a self-contained component receiving props from `App.tsx`. No global state management — the WebSocket hook is the single data source.
- **CSS**: Dark theme using CSS custom properties (defined in `App.css`). Key variables: `--bg`, `--panel-bg`, `--border`, `--text`, `--text-dim`, `--accent` (red), `--green`, `--blue`, `--yellow`. Use `font-variant-numeric: tabular-nums` for numbers that change rapidly.
- **F1 broadcast style**: Purple (`#a855f7`) for fastest times. Color-coded tyre temps (blue=cold, green=optimal, yellow=warm, red=hot). Monospace font throughout.
- **CSS specificity**: When adding conditional classes (like `.purple`) to elements inside styled rows, ensure the conditional class has equal or higher specificity than the row's base styles.

## Build & Test Workflow

```bash
# Build Rust backend
cd pitwall && cargo build

# Build React frontend
cd pitwall-ui && npm run build

# Dev workflow (3 terminals):
# T1: cargo run -- live f1 --dry-run
# T2: cargo run -- sim pitwall_data/recordings/<id>.pitraw
# T3: cd pitwall-ui && npm run dev
```

## Storage

- Parquet files with ZSTD compression via `arrow` + `parquet` crates
- Layout: `pitwall_data/sessions/{year}/{track}/{session_id}/`
- Track names resolved by `storage::track_name(id)` — use official F1 24 track IDs
- Raw recordings (`.pitraw`) are timestamped packet dumps for simulation replay

## Key Gotchas

- Sector times are 0 in the packet where lap_number increments — always use cached values from the previous frame
- The `laps` vec in `LiveFrame` is sent newest-first (`.rev().take(10)`)
- WebSocket only serializes when `receiver_count() > 0` to avoid wasted work
- The React UI at `localhost:5173` connects to WebSocket at `localhost:8765/ws` — CORS is permissive
