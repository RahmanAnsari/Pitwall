//! Raw UDP packet recorder and replay simulator.
//!
//! Records raw packet bytes with timestamps to a binary file during capture.
//! Can replay them back onto a UDP port at original timing for development testing.
//!
//! File format (.pitraw):
//!   [u64 timestamp_nanos][u16 packet_len][packet_bytes...]
//!   repeated for each packet

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use tracing::info;

const MAGIC: &[u8; 8] = b"PITRAW01";

/// Records raw UDP packets to a .pitraw file.
pub struct PacketRecorder {
    writer: BufWriter<File>,
    start_time: Instant,
    packet_count: u64,
}

impl PacketRecorder {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(MAGIC)?;
        Ok(Self {
            writer,
            start_time: Instant::now(),
            packet_count: 0,
        })
    }

    pub fn record(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let elapsed_nanos = self.start_time.elapsed().as_nanos() as u64;
        let len = data.len() as u16;

        self.writer.write_all(&elapsed_nanos.to_le_bytes())?;
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(data)?;
        self.packet_count += 1;
        Ok(())
    }

    pub fn finish(mut self) -> anyhow::Result<u64> {
        self.writer.flush()?;
        Ok(self.packet_count)
    }
}

/// Replays a .pitraw file onto a UDP port at original timing.
pub async fn simulate(recording_path: &Path, target_port: u16, speed: f32) -> anyhow::Result<()> {
    let file = File::open(recording_path)?;
    let mut reader = BufReader::new(file);

    // Verify magic
    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        anyhow::bail!("Not a valid .pitraw file");
    }

    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let target = format!("127.0.0.1:{target_port}");
    socket.connect(&target).await?;

    info!("Simulating packets to {target} at {speed}x speed");

    let mut packet_count: u64 = 0;
    let playback_start = Instant::now();

    loop {
        // Read timestamp
        let mut ts_buf = [0u8; 8];
        if reader.read_exact(&mut ts_buf).is_err() {
            break; // EOF
        }
        let timestamp_nanos = u64::from_le_bytes(ts_buf);

        // Read packet length
        let mut len_buf = [0u8; 2];
        if reader.read_exact(&mut len_buf).is_err() {
            break;
        }
        let packet_len = u16::from_le_bytes(len_buf) as usize;

        // Read packet data
        let mut packet_data = vec![0u8; packet_len];
        if reader.read_exact(&mut packet_data).is_err() {
            break;
        }

        // Wait for correct timing (adjusted by speed multiplier)
        if packet_count > 0 {
            let target_time = playback_start
                + Duration::from_nanos((timestamp_nanos as f64 / speed as f64) as u64);
            let now = Instant::now();
            if target_time > now {
                tokio::time::sleep(target_time - now).await;
            }
        }

        // Send packet
        socket.send(&packet_data).await?;
        packet_count += 1;

        // Periodic status
        if packet_count % 1000 == 0 {
            let elapsed = playback_start.elapsed().as_secs_f32();
            info!("Sent {packet_count} packets ({elapsed:.1}s elapsed)");
        }
    }

    let elapsed = playback_start.elapsed().as_secs_f32();
    info!("Simulation complete: {packet_count} packets in {elapsed:.1}s");
    Ok(())
}

/// Get the recording path for a session.
pub fn recording_path(storage_path: &Path, session_id: &str) -> PathBuf {
    storage_path.join("recordings").join(format!("{session_id}.pitraw"))
}

/// List all available .pitraw recordings.
pub fn list_recordings(data_dir: &Path) -> anyhow::Result<()> {
    let recordings_dir = data_dir.join("recordings");
    if !recordings_dir.exists() {
        println!("No recordings found in {}", data_dir.display());
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(&recordings_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "pitraw")
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        println!("No recordings found.");
        return Ok(());
    }

    entries.sort_by_key(|e| e.path());

    println!("📼 Found {} recording(s):\n", entries.len());

    for entry in &entries {
        let path = entry.path();
        let name = path.file_stem().unwrap_or_default().to_string_lossy();
        let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let size_mb = size as f64 / 1_048_576.0;

        // Peek at packet count by reading file
        let packet_info = match File::open(&path) {
            Ok(mut f) => {
                let mut magic = [0u8; 8];
                if f.read_exact(&mut magic).is_ok() && &magic == MAGIC {
                    // Estimate packet count from file size (avg ~200 bytes per entry)
                    let data_size = size.saturating_sub(8);
                    let est_packets = data_size / 200; // rough estimate
                    format!("~{est_packets} packets")
                } else {
                    "invalid file".to_string()
                }
            }
            Err(_) => "unreadable".to_string(),
        };

        println!("  {name}");
        println!("    Size: {size_mb:.1} MB | {packet_info}");
        println!("    Path: {}", path.display());
        println!();
    }

    Ok(())
}
