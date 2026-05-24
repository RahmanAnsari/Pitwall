//! UDP telemetry receiver.
//!
//! Async UDP listener that dispatches packets to parsers and drives session state.

use std::path::PathBuf;

use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

use crate::live_server::{LiveFrame, WsBroadcaster};
use crate::packets::{event_codes, PacketId};
use crate::parsers::*;
use crate::recorder::PacketRecorder;
use crate::session::SessionState;
use crate::storage;

const BUFFER_SIZE: usize = 2048;

pub async fn run(port: u16, storage_path: PathBuf, record_raw: bool, dry_run: bool, broadcaster: Option<WsBroadcaster>) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{port}");
    let socket = UdpSocket::bind(&addr).await?;
    info!("Listening on {addr}");

    let mut state = SessionState::new();
    let mut buf = [0u8; BUFFER_SIZE];
    let mut packets_received: u64 = 0;
    let mut last_status_count: u64 = 0;
    let mut last_status_time = tokio::time::Instant::now();
    let mut recorder: Option<PacketRecorder> = None;

    // Handle Ctrl+C gracefully
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        r.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    loop {
        if !running.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        let recv = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            socket.recv_from(&mut buf),
        )
        .await;

        let (len, _addr) = match recv {
            Ok(Ok((len, addr))) => (len, addr),
            Ok(Err(e)) => {
                warn!("Receive error: {e}");
                continue;
            }
            Err(_) => {
                // Timeout - print periodic status if receiving data
                if packets_received > 0 && packets_received > last_status_count {
                    let elapsed = last_status_time.elapsed().as_secs_f32();
                    if elapsed >= 5.0 {
                        let rate = (packets_received - last_status_count) as f32 / elapsed;
                        info!(
                            "Receiving: {} packets total ({:.0} pkt/s) | {} samples buffered",
                            packets_received,
                            rate,
                            state.telemetry.len()
                        );
                        last_status_count = packets_received;
                        last_status_time = tokio::time::Instant::now();
                    }
                }
                continue;
            }
        };

        let data = &buf[..len];
        packets_received += 1;

        // Log first packet received
        if packets_received == 1 {
            info!("First packet received ({len} bytes) - connection established");
        }

        // Record raw packet for later simulation
        if let Some(ref mut rec) = recorder {
            if let Err(e) = rec.record(data) {
                warn!("Recording error: {e}");
            }
        }

        if let Err(e) = process_packet(data, &mut state, &storage_path, &mut recorder, record_raw, dry_run) {
            debug!("Packet processing error: {e}");
        }

        // Broadcast live frame to WebSocket clients
        if let Some(ref bc) = broadcaster {
            if let Some(sample) = state.latest_sample() {
                let frame = LiveFrame::from_state(
                    sample,
                    state.meta.as_ref(),
                    state.current_lap,
                    &state.laps,
                    state.current_sector,
                    state.live_sector1_ms,
                    state.live_sector2_ms,
                );
                bc.broadcast(&frame);
            }
        }
    }

    // Finalize on shutdown
    if state.active {
        finalize_session(&mut state, &storage_path, &mut recorder, dry_run)?;
    }

    info!("Total packets received: {packets_received}");
    Ok(())
}

fn process_packet(
    data: &[u8],
    state: &mut SessionState,
    storage_path: &PathBuf,
    recorder: &mut Option<PacketRecorder>,
    record_raw: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    let header = parse_header(data).ok_or_else(|| anyhow::anyhow!("header too short"))?;
    let packet_id = PacketId::from_u8(header.packet_id)
        .ok_or_else(|| anyhow::anyhow!("unknown packet id: {}", header.packet_id))?;

    match packet_id {
        PacketId::Event => {
            if let Some(code) = parse_event_code(data) {
                handle_event(code, &header, state, storage_path, recorder, dry_run)?;
            }
        }
        PacketId::Session => {
            if let Some(session_data) = parse_session_data(data) {
                if !state.active {
                    state.start_session(
                        &session_data,
                        header.session_uid,
                        header.game_year,
                        header.game_major_version,
                        header.game_minor_version,
                    );
                    info!(
                        "Recording: track={} type={}",
                        session_data.track_id, session_data.session_type
                    );

                    // Start raw packet recording if --raw flag was passed
                    if record_raw {
                        if let Some(ref meta) = state.meta {
                            let rec_path = crate::recorder::recording_path(storage_path, &meta.session_id);
                            match PacketRecorder::new(&rec_path) {
                                Ok(rec) => {
                                    info!("Raw recording: {}", rec_path.display());
                                    *recorder = Some(rec);
                                }
                                Err(e) => warn!("Failed to start recording: {e}"),
                            }
                        }
                    }
                }
            }
        }
        PacketId::LapData => {
            if !state.active {
                return Ok(());
            }
            if let Some(lap) = parse_player_lap_data(data, header.player_car_index) {
                let prev_lap_count = state.laps.len();
                state.check_lap_completion(&lap);
                if state.laps.len() > prev_lap_count {
                    let completed = state.laps.last().unwrap();
                    info!(
                        "Lap {} completed: {:.3}s (P{})",
                        completed.lap_number,
                        completed.lap_time_ms as f64 / 1000.0,
                        completed.position
                    );
                }
                let sample = state.get_sample(header.session_time, header.frame_identifier);
                sample.merge_lap_data(&lap);
            }
        }
        PacketId::Motion => {
            if !state.active {
                return Ok(());
            }
            if let Some(motion) = parse_player_motion(data, header.player_car_index) {
                let sample = state.get_sample(header.session_time, header.frame_identifier);
                sample.merge_motion(&motion);
            }
        }
        PacketId::CarTelemetry => {
            if !state.active {
                return Ok(());
            }
            if let Some(telem) = parse_player_car_telemetry(data, header.player_car_index) {
                let sample = state.get_sample(header.session_time, header.frame_identifier);
                sample.merge_car_telemetry(&telem);
            }
        }
        PacketId::CarStatus => {
            if !state.active {
                return Ok(());
            }
            if let Some(status) = parse_player_car_status(data, header.player_car_index) {
                let sample = state.get_sample(header.session_time, header.frame_identifier);
                sample.merge_car_status(&status);
            }
        }
        PacketId::MotionEx => {
            if !state.active {
                return Ok(());
            }
            if let Some(ex) = parse_motion_ex(data) {
                let sample = state.get_sample(header.session_time, header.frame_identifier);
                sample.merge_motion_ex(&ex);
            }
        }
        _ => {} // Other packets not needed for V1
    }

    Ok(())
}

fn handle_event(
    code: [u8; 4],
    header: &crate::packets::PacketHeader,
    state: &mut SessionState,
    storage_path: &PathBuf,
    recorder: &mut Option<PacketRecorder>,
    dry_run: bool,
) -> anyhow::Result<()> {
    if &code == event_codes::SESSION_STARTED {
        info!("Event: Session Started");
        if state.active {
            finalize_session(state, storage_path, recorder, dry_run)?;
        }
    } else if &code == event_codes::SESSION_ENDED {
        info!("Event: Session Ended");
        finalize_session(state, storage_path, recorder, dry_run)?;
    }

    if state.active {
        state.add_event(header.session_time, header.frame_identifier, code);
    }

    Ok(())
}

fn finalize_session(state: &mut SessionState, storage_path: &PathBuf, recorder: &mut Option<PacketRecorder>, dry_run: bool) -> anyhow::Result<()> {
    state.end_session();

    // Finish raw recording
    if let Some(rec) = recorder.take() {
        match rec.finish() {
            Ok(count) => info!("Raw recording saved: {count} packets"),
            Err(e) => warn!("Failed to finalize recording: {e}"),
        }
    }

    if let Some(ref meta) = state.meta {
        info!(
            "Session complete: {} samples, {} laps",
            state.telemetry.len(),
            state.laps.len()
        );

        if !dry_run {
            let dir = storage::save_session(
                storage_path,
                meta,
                &state.laps,
                &state.telemetry,
                &state.events,
            )?;
            info!("Saved to: {}", dir.display());
        } else {
            info!("Dry-run: skipped storage");
        }
    }

    state.reset();
    Ok(())
}
