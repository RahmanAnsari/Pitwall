# PitWall

F1 24 telemetry capture, storage, and analysis tool built in Rust. Designed for indie sim racers and amateurs who want lossless, high-frequency telemetry data they can query, replay, and analyze — with a live web-based dashboard.

## What It Does

- Connects to F1 24's UDP telemetry stream (port 20777)
- Detects session start/end automatically
- Records every telemetry sample (all Tier 1 + Tier 2 channels) at game tick rate
- Stores data losslessly in compressed Parquet files (ZSTD)
- Reconstructs and replays any lap frame-by-frame
- Exports individual laps to standalone Parquet for DuckDB/pandas analytics
- Optionally records raw UDP packets for deterministic replay during development
- **Live web UI** — real-time F1-style dashboard via WebSocket

## Live UI

A React-based dashboard that connects to the backend via WebSocket and displays real-time telemetry in an F1 broadcast style.

**Features:**
- Speed / RPM / Gear display with animated RPM bar
- Throttle and brake input bars
- ERS energy level, deploy mode, DRS status, fuel remaining
- Tyre temperature grid with color-coded heat indicators (blue → green → yellow → red)
- Live lap timing table with sectors that fill in as they complete
- Purple highlighting on fastest lap time and individual fastest sectors (live, not just after session)
- Session header with track name, session type, live/offline indicator, session clock

**Running the UI:**

```bash
# Terminal 1: Start the backend (includes WebSocket server)
cargo run -- live f1

# Terminal 2: Start the React dev server
cd pitwall-ui
npm run dev
```

Open `http://localhost:5173` — it auto-connects to `ws://localhost:8765/ws`.

For testing without the game:

```bash
# Terminal 1: Backend in dry-run mode
cargo run -- live f1 --dry-run

# Terminal 2: Simulate recorded packets
cargo run -- sim pitwall_data/recordings/<session_id>.pitraw

# Terminal 3: React UI
cd pitwall-ui && npm run dev
```

| Option | Default | Description |
|--------|---------|-------------|
| `--ws-port` | 8765 | WebSocket server port for the live UI |

## Telemetry Channels Captured

**Driver Inputs:** throttle, brake, steer, clutch, gear

**Vehicle State:** speed, RPM, DRS status, DRS allowed, ERS energy, ERS deploy mode, fuel remaining

**Position & Motion:** world XYZ, velocity XYZ, yaw, pitch, roll

**G-Forces:** lateral, longitudinal, vertical

**Tyre Temps:** surface + inner for all 4 wheels (FL, FR, RL, RR)

**Brake Temps:** all 4 wheels

**Suspension:** position, velocity, acceleration for all 4 wheels

**Wheel Speeds:** all 4 wheels

## Storage Layout

```
pitwall_data/
├── sessions/
│   └── {year}/
│       └── {track}/
│           └── {session_id}/
│               ├── metadata.json
│               ├── telemetry.parquet
│               ├── laps.parquet
│               └── events.parquet
└── recordings/
    └── {session_id}.pitraw
```

- Parquet files use ZSTD compression
- A 60-minute race produces ~10-50 MB of telemetry
- Metadata is JSON for easy inspection
- Raw recordings (`.pitraw`) are only created with `--raw` flag

## Installation

```bash
# From the pitwall directory
cargo install --path .

# Or run directly without installing
cargo run -- <command>
```

Requires Rust 1.70+.

For the live UI:

```bash
cd pitwall-ui
npm install
```

Requires Node.js 18+.

## Commands

### `pitwall live f1`

Capture live telemetry from F1 24 with the WebSocket server for the live UI.

```bash
pitwall live f1 [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-p, --port` | 20777 | UDP port to listen on |
| `-o, --output` | ./pitwall_data | Storage directory |
| `--raw` | off | Also record raw UDP packets (.pitraw) for simulation |
| `--dry-run` | off | Process packets without writing to disk |
| `--ws-port` | 8765 | WebSocket port for the live UI |

**Examples:**

```bash
# Standard capture with live UI
pitwall live f1

# Capture with raw recording for later simulation
pitwall live f1 --raw

# Listen on a custom port
pitwall live f1 --port 20778

# Process packets without storing (for testing with sim)
pitwall live f1 --dry-run

# Use a different WebSocket port
pitwall live f1 --ws-port 9000
```

**Terminal output during capture:**

```
🏎️  PitWall Live Mode → F1 24 UDP
   Port: 20777
   Live UI: ws://localhost:8765/ws
   Storage: ./pitwall_data
   Press Ctrl+C to stop

[INFO] Listening on 0.0.0.0:20777
[INFO] Live UI WebSocket server on ws://localhost:8765/ws
[INFO] First packet received (1349 bytes) - connection established
[INFO] Recording: track=11 type=15
[INFO] Lap 1 completed: 81.234s (P1)
[INFO] Lap 2 completed: 80.891s (P1)
```

---

### `pitwall replay sessions`

List all recorded sessions.

```bash
pitwall replay sessions [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-d, --data-dir` | ./pitwall_data | Storage directory |

---

### `pitwall replay recordings`

List raw `.pitraw` recordings available for simulation.

```bash
pitwall replay recordings [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-d, --data-dir` | ./pitwall_data | Storage directory |

---

### `pitwall replay lap`

Replay a recorded lap frame-by-frame from Parquet data.

```bash
pitwall replay lap <SESSION_DIR> --lap <LAP_NUMBER>
```

| Argument | Description |
|----------|-------------|
| `session_dir` | Path to session directory |
| `-l, --lap` | Lap number to replay |

---

### `pitwall replay export`

Export telemetry to a standalone Parquet file for analytics.

```bash
pitwall replay export <SESSION_DIR> [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-l, --lap` | (none) | Specific lap to export. Omit for session summary |
| `-o, --output` | ./export.parquet | Output file path |

---

### `pitwall sim`

Simulate a raw recording by sending packets over UDP. Used for development testing without the game running.

```bash
pitwall sim <RECORDING> [OPTIONS]
```

| Option | Default | Description |
|--------|---------|-------------|
| `-p, --port` | 20777 | Target UDP port to send packets to |
| `-s, --speed` | 1.0 | Playback speed multiplier |

**Typical dev workflow (three terminals):**

```bash
# Terminal 1: receiver in dry-run mode
pitwall live f1 --dry-run

# Terminal 2: simulate recorded packets
pitwall sim ./pitwall_data/recordings/abc-123-def.pitraw

# Terminal 3: live UI
cd pitwall-ui && npm run dev
```

---

## F1 24 Game Setup

In F1 24, go to **Settings → UDP Telemetry**:

| Setting | Value |
|---------|-------|
| UDP Telemetry | On |
| UDP Broadcast Mode | Off |
| UDP IP Address | Your computer's IP (e.g. 192.168.1.100) |
| UDP Port | 20777 |
| UDP Send Rate | 20Hz (or higher) |
| UDP Format | 2024 |

For local testing (game on same machine), use `127.0.0.1` as the IP.

## Querying with DuckDB

The Parquet files are directly queryable:

```sql
-- Load a session
SELECT * FROM 'pitwall_data/sessions/2026/monza/abc-123-def/telemetry.parquet'
WHERE lap_number = 3
ORDER BY timestamp;

-- Best braking point into T1
SELECT lap_distance, speed, brake, g_longitudinal
FROM 'telemetry.parquet'
WHERE lap_number = 3 AND lap_distance BETWEEN 800 AND 900;

-- Tyre temp comparison across laps
SELECT lap_number, AVG(tyre_surface_fl) as avg_fl, AVG(tyre_surface_fr) as avg_fr
FROM 'telemetry.parquet'
GROUP BY lap_number;

-- Lap times
SELECT * FROM 'pitwall_data/sessions/2026/monza/abc-123-def/laps.parquet';
```

## Circuit SVGs

Track outline SVGs are auto-generated from GeoJSON source files during `cargo build` via `build.rs`.

```
pitwall/circuits/
├── geojson/       ← source track outlines (checked in)
│   └── sakhir.geojson
└── svg/           ← generated at build time (gitignored)
    └── sakhir.svg
```

To add a new track: drop a `.geojson` file with a LineString geometry into `circuits/geojson/` and rebuild. The build script normalizes coordinates to a 0–1 space with aspect-ratio correction and outputs a clean SVG polyline.

## Architecture

```
pitwall/src/
├── main.rs          CLI (clap) - command routing
├── build.rs         Build script - generates circuit SVGs from GeoJSON
├── packets.rs       F1 24 UDP structs (repr(C, packed), zero-copy via bytemuck)
├── parsers.rs       Zero-copy packet parsing from raw bytes
├── session.rs       Session state machine, sample merging, lap detection
├── storage.rs       Parquet + JSON writer (ZSTD compressed, columnar)
├── receiver.rs      Async UDP listener (tokio), drives session lifecycle
├── recorder.rs      Raw packet recording (.pitraw) and UDP simulation
├── replay.rs        Frame-by-frame lap reconstruction from Parquet
├── export.rs        Lap export, session summary, session discovery
└── live_server.rs   WebSocket server (axum) broadcasting LiveFrame JSON

pitwall-ui/src/
├── App.tsx          Main dashboard layout
├── hooks/
│   └── useTelemetry.ts   WebSocket connection + auto-reconnect
└── components/
    ├── SessionHeader.tsx  Track, session type, live indicator, clock
    ├── SpeedPanel.tsx     Speed/RPM/Gear with animated RPM bar
    ├── InputBars.tsx      Throttle/Brake bars
    ├── ErsPanel.tsx       ERS, DRS, fuel status
    ├── TyreTemps.tsx      4-wheel temp grid with heat colors
    └── LapTable.tsx       Live sector timing + purple highlights
```

## License

Personal project. Not affiliated with EA, Codemasters, or Formula 1.
