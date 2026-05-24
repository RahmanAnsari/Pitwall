mod packets;
mod parsers;
mod session;
mod storage;
mod receiver;
mod recorder;
mod replay;
mod export;
mod live_server;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "pitwall", about = "F1 24 Telemetry Capture & Analysis")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Live mode - capture telemetry in real-time
    Live {
        #[command(subcommand)]
        source: LiveSource,
    },
    /// Replay mode - work with recorded sessions
    Replay {
        #[command(subcommand)]
        action: ReplayAction,
    },
    /// Simulate a recorded stream over UDP for testing
    Sim {
        /// Path to .pitraw recording file
        recording: PathBuf,
        /// Target UDP port to send packets to
        #[arg(short, long, default_value_t = 20777)]
        port: u16,
        /// Playback speed multiplier (e.g. 2.0 = double speed)
        #[arg(short, long, default_value_t = 1.0)]
        speed: f32,
    },
}

#[derive(Subcommand)]
enum LiveSource {
    /// Connect to F1 24 UDP telemetry stream
    F1 {
        /// UDP port to listen on
        #[arg(short, long, default_value_t = 20777)]
        port: u16,
        /// Storage directory
        #[arg(short, long, default_value = "./pitwall_data")]
        output: PathBuf,
        /// Also record raw UDP packets for simulation replay
        #[arg(long)]
        raw: bool,
        /// Process packets without writing to disk (for testing with sim)
        #[arg(long)]
        dry_run: bool,
        /// Enable live WebSocket server for the UI
        #[arg(long, default_value_t = 8765)]
        ws_port: u16,
    },
}

#[derive(Subcommand)]
enum ReplayAction {
    /// List all recorded sessions
    Sessions {
        /// Storage directory
        #[arg(short, long, default_value = "./pitwall_data")]
        data_dir: PathBuf,
    },
    /// List raw recordings available for simulation
    Recordings {
        /// Storage directory
        #[arg(short, long, default_value = "./pitwall_data")]
        data_dir: PathBuf,
    },
    /// Replay a lap frame-by-frame
    Lap {
        /// Path to session directory
        session_dir: PathBuf,
        /// Lap number to replay
        #[arg(short, long)]
        lap: u16,
    },
    /// Export telemetry to Parquet
    Export {
        /// Path to session directory
        session_dir: PathBuf,
        /// Specific lap to export (omit for session summary)
        #[arg(short, long)]
        lap: Option<u16>,
        /// Output file path
        #[arg(short, long, default_value = "./export.parquet")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("pitwall=info".parse()?))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Live { source } => match source {
            LiveSource::F1 { port, output, raw, dry_run, ws_port } => {
                println!("🏎️  PitWall Live Mode → F1 24 UDP");
                println!("   Port: {port}");
                println!("   Live UI: ws://localhost:{ws_port}/ws");
                if dry_run {
                    println!("   Mode: dry-run (no storage)");
                } else {
                    println!("   Storage: {}", output.display());
                    if raw {
                        println!("   Raw recording: enabled");
                    }
                }
                println!("   Press Ctrl+C to stop\n");

                let broadcaster = live_server::WsBroadcaster::new();
                let bc = broadcaster.clone();

                // Start WebSocket server in background
                tokio::spawn(async move {
                    if let Err(e) = live_server::start_server(bc, ws_port).await {
                        tracing::error!("WebSocket server error: {e}");
                    }
                });

                receiver::run(port, output, raw, dry_run, Some(broadcaster)).await?;
            }
        },
        Commands::Replay { action } => match action {
            ReplayAction::Sessions { data_dir } => {
                export::list_sessions(&data_dir)?;
            }
            ReplayAction::Recordings { data_dir } => {
                recorder::list_recordings(&data_dir)?;
            }
            ReplayAction::Lap { session_dir, lap } => {
                replay::replay_lap(&session_dir, lap)?;
            }
            ReplayAction::Export { session_dir, lap, output } => {
                export::export(&session_dir, lap, &output)?;
            }
        },
        Commands::Sim { recording, port, speed } => {
            println!("📡 PitWall Simulator");
            println!("   Recording: {}", recording.display());
            println!("   Target: 127.0.0.1:{port}");
            println!("   Speed: {speed}x\n");
            recorder::simulate(&recording, port, speed).await?;
        },
    }

    Ok(())
}
