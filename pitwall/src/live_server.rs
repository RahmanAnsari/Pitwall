//! WebSocket server for broadcasting live telemetry to the React UI.

use std::sync::Arc;
use std::net::SocketAddr;

use axum::{
    extract::{State, Path, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::session::{LapRecord, SessionMeta, TelemetrySample};

/// Snapshot of live state sent over WebSocket as JSON.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveFrame {
    // Session
    pub session_active: bool,
    pub track: String,
    pub session_type: String,
    pub current_lap: u8,
    pub total_laps: u8,

    // Telemetry
    pub speed: u16,
    pub rpm: u16,
    pub gear: i8,
    pub throttle: f32,
    pub brake: f32,
    pub steer: f32,
    pub drs: u8,
    pub drs_allowed: u8,
    pub ers_deploy_mode: u8,
    pub ers_store_energy: f32,
    pub fuel_in_tank: f32,

    // Tyres (surface temps)
    pub tyre_fl: u8,
    pub tyre_fr: u8,
    pub tyre_rl: u8,
    pub tyre_rr: u8,

    // Brake temps
    pub brake_temp_fl: u16,
    pub brake_temp_fr: u16,
    pub brake_temp_rl: u16,
    pub brake_temp_rr: u16,

    // Lap timing
    pub lap_distance: f32,
    pub total_distance: f32,
    pub session_time: f32,

    // G-forces
    pub g_lateral: f32,
    pub g_longitudinal: f32,

    // Current lap live sectors
    pub current_sector: u8,
    pub current_sector1_ms: f32,
    pub current_sector2_ms: f32,

    // Last completed laps
    pub laps: Vec<LapSummary>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LapSummary {
    pub lap_number: u8,
    pub lap_time_ms: u32,
    pub sector1_ms: f32,
    pub sector2_ms: f32,
    pub sector3_ms: f32,
    pub position: u8,
}

impl LiveFrame {
    pub fn from_state(
        sample: &TelemetrySample,
        meta: Option<&SessionMeta>,
        current_lap: u8,
        laps: &[LapRecord],
        current_sector: u8,
        live_sector1_ms: f32,
        live_sector2_ms: f32,
    ) -> Self {
        let (track, session_type, total_laps, session_active) = match meta {
            Some(m) => (
                crate::storage::track_name(m.track_id).to_string(),
                session_type_name(m.session_type),
                m.total_laps,
                true,
            ),
            None => (String::new(), String::new(), 0, false),
        };

        let lap_summaries: Vec<LapSummary> = laps.iter().rev().take(10).map(|l| LapSummary {
            lap_number: l.lap_number,
            lap_time_ms: l.lap_time_ms,
            sector1_ms: l.sector1_ms,
            sector2_ms: l.sector2_ms,
            sector3_ms: l.sector3_ms,
            position: l.position,
        }).collect();

        Self {
            session_active,
            track,
            session_type,
            current_lap,
            total_laps,
            speed: sample.speed,
            rpm: sample.rpm,
            gear: sample.gear,
            throttle: sample.throttle,
            brake: sample.brake,
            steer: sample.steer,
            drs: sample.drs,
            drs_allowed: sample.drs_allowed,
            ers_deploy_mode: sample.ers_deploy_mode,
            ers_store_energy: sample.ers_store_energy,
            fuel_in_tank: sample.fuel_in_tank,
            tyre_fl: sample.tyre_surface_fl,
            tyre_fr: sample.tyre_surface_fr,
            tyre_rl: sample.tyre_surface_rl,
            tyre_rr: sample.tyre_surface_rr,
            brake_temp_fl: sample.brake_temp_fl,
            brake_temp_fr: sample.brake_temp_fr,
            brake_temp_rl: sample.brake_temp_rl,
            brake_temp_rr: sample.brake_temp_rr,
            lap_distance: sample.lap_distance,
            total_distance: sample.total_distance,
            session_time: sample.timestamp,
            g_lateral: sample.g_lateral,
            g_longitudinal: sample.g_longitudinal,
            current_sector,
            current_sector1_ms: live_sector1_ms,
            current_sector2_ms: live_sector2_ms,
            laps: lap_summaries,
        }
    }
}

fn session_type_name(t: u8) -> String {
    match t {
        0 => "Unknown",
        1 => "P1",
        2 => "P2",
        3 => "P3",
        4 => "Short Practice",
        5 => "Q1",
        6 => "Q2",
        7 => "Q3",
        8 => "Short Quali",
        9 => "OSQ",
        10 => "Sprint Shootout 1",
        11 => "Sprint Shootout 2",
        12 => "Sprint Shootout 3",
        13 => "Short Sprint Shootout",
        14 => "OSS",
        15 => "Race",
        16 => "Race 2",
        17 => "Race 3",
        18 => "Time Trial",
        _ => "Unknown",
    }.to_string()
}

/// Shared state for the WebSocket server.
#[derive(Clone)]
pub struct WsBroadcaster {
    tx: broadcast::Sender<String>,
}

impl WsBroadcaster {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        Self { tx }
    }

    /// Send a frame to all connected WebSocket clients.
    pub fn broadcast(&self, frame: &LiveFrame) {
        if self.tx.receiver_count() > 0 {
            if let Ok(json) = serde_json::to_string(frame) {
                let _ = self.tx.send(json);
            }
        }
    }
}

/// Serve a circuit SVG by track name.
async fn circuit_svg_handler(Path(name): Path<String>) -> impl IntoResponse {
    let safe_name = name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "");
    let filename = format!("{}.svg", safe_name);

    // Use CARGO_MANIFEST_DIR baked in at compile time as the reliable base path
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("circuits/svg")
        .join(&filename);

    // Also try CWD-relative as fallback (for installed binaries)
    let cwd_path = std::path::PathBuf::from("circuits/svg").join(&filename);

    let path = if manifest_path.exists() {
        &manifest_path
    } else {
        &cwd_path
    };

    info!("Circuit SVG request: name={:?} resolved={:?}", name, path);

    match tokio::fs::read_to_string(path).await {
        Ok(svg) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/svg+xml"),
                (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            ],
            svg,
        ).into_response(),
        Err(e) => {
            info!("Circuit SVG not found: {:?} error={}", path, e);
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

/// Start the WebSocket server on the given port.
pub async fn start_server(broadcaster: WsBroadcaster, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/circuits/:name", get(circuit_svg_handler))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(broadcaster));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Live UI WebSocket server on ws://localhost:{port}/ws");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsBroadcaster>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<WsBroadcaster>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();

    // Spawn task to forward broadcast messages to this client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Consume incoming messages (we don't need them, but must drain)
    while receiver.next().await.is_some() {}

    send_task.abort();
}
