//! Lossless telemetry storage using Parquet + JSON metadata.
//!
//! Layout:
//!   sessions/{year}/{track}/{session_id}/
//!     metadata.json
//!     telemetry.parquet
//!     laps.parquet
//!     events.parquet

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow::array::*;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;

use crate::session::{EventRecord, LapRecord, SessionMeta, TelemetrySample};

/// Track ID to name mapping (official F1 24 spec).
pub fn track_name(id: i8) -> &'static str {
    match id {
        0 => "melbourne",
        1 => "paul_ricard",
        2 => "shanghai",
        3 => "sakhir",
        4 => "catalunya",
        5 => "monaco",
        6 => "montreal",
        7 => "silverstone",
        8 => "hockenheim",
        9 => "hungaroring",
        10 => "spa",
        11 => "monza",
        12 => "singapore",
        13 => "suzuka",
        14 => "abu_dhabi",
        15 => "texas",
        16 => "brazil",
        17 => "austria",
        18 => "sochi",
        19 => "mexico",
        20 => "baku",
        21 => "sakhir_short",
        22 => "silverstone_short",
        23 => "texas_short",
        24 => "suzuka_short",
        25 => "hanoi",
        26 => "zandvoort",
        27 => "imola",
        28 => "portimao",
        29 => "jeddah",
        30 => "miami",
        31 => "las_vegas",
        32 => "losail",
        _ => "unknown",
    }
}

/// Compute session storage directory.
pub fn session_dir(base: &Path, meta: &SessionMeta) -> PathBuf {
    let year = &meta.start_time[..4]; // ISO 8601 starts with year
    let track = track_name(meta.track_id);
    base.join("sessions").join(year).join(track).join(&meta.session_id)
}

/// Save a complete session to disk.
pub fn save_session(
    base: &Path,
    meta: &SessionMeta,
    laps: &[LapRecord],
    telemetry: &[TelemetrySample],
    events: &[EventRecord],
) -> anyhow::Result<PathBuf> {
    let dir = session_dir(base, meta);
    fs::create_dir_all(&dir)?;

    // 1. Metadata JSON
    let meta_json = serde_json::to_string_pretty(meta)?;
    fs::write(dir.join("metadata.json"), meta_json)?;

    // 2. Laps
    if !laps.is_empty() {
        write_laps_parquet(&dir.join("laps.parquet"), laps)?;
    }

    // 3. Telemetry
    if !telemetry.is_empty() {
        write_telemetry_parquet(&dir.join("telemetry.parquet"), telemetry)?;
    }

    // 4. Events
    if !events.is_empty() {
        write_events_parquet(&dir.join("events.parquet"), events)?;
    }

    Ok(dir)
}

fn writer_props() -> WriterProperties {
    WriterProperties::builder()
        .set_compression(Compression::ZSTD(Default::default()))
        .build()
}

fn write_laps_parquet(path: &Path, laps: &[LapRecord]) -> anyhow::Result<()> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("lap_id", DataType::Utf8, false),
        Field::new("session_id", DataType::Utf8, false),
        Field::new("lap_number", DataType::UInt8, false),
        Field::new("lap_time_ms", DataType::UInt32, false),
        Field::new("sector1_ms", DataType::Float32, false),
        Field::new("sector2_ms", DataType::Float32, false),
        Field::new("sector3_ms", DataType::Float32, false),
        Field::new("position", DataType::UInt8, false),
        Field::new("penalties", DataType::UInt8, false),
        Field::new("warnings", DataType::UInt8, false),
    ]));

    let batch = RecordBatch::try_new(schema.clone(), vec![
        Arc::new(StringArray::from(laps.iter().map(|l| l.lap_id.as_str()).collect::<Vec<_>>())),
        Arc::new(StringArray::from(laps.iter().map(|l| l.session_id.as_str()).collect::<Vec<_>>())),
        Arc::new(UInt8Array::from(laps.iter().map(|l| l.lap_number).collect::<Vec<_>>())),
        Arc::new(UInt32Array::from(laps.iter().map(|l| l.lap_time_ms).collect::<Vec<_>>())),
        Arc::new(Float32Array::from(laps.iter().map(|l| l.sector1_ms).collect::<Vec<_>>())),
        Arc::new(Float32Array::from(laps.iter().map(|l| l.sector2_ms).collect::<Vec<_>>())),
        Arc::new(Float32Array::from(laps.iter().map(|l| l.sector3_ms).collect::<Vec<_>>())),
        Arc::new(UInt8Array::from(laps.iter().map(|l| l.position).collect::<Vec<_>>())),
        Arc::new(UInt8Array::from(laps.iter().map(|l| l.penalties).collect::<Vec<_>>())),
        Arc::new(UInt8Array::from(laps.iter().map(|l| l.warnings).collect::<Vec<_>>())),
    ])?;

    let file = fs::File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(writer_props()))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

fn write_telemetry_parquet(path: &Path, samples: &[TelemetrySample]) -> anyhow::Result<()> {
    let schema = Arc::new(telemetry_schema());

    // Build columnar arrays from samples
    let n = samples.len();
    let mut timestamp = Vec::with_capacity(n);
    let mut frame = Vec::with_capacity(n);
    let mut lap_number = Vec::with_capacity(n);
    let mut lap_distance = Vec::with_capacity(n);
    let mut total_distance = Vec::with_capacity(n);
    let mut throttle = Vec::with_capacity(n);
    let mut brake = Vec::with_capacity(n);
    let mut steer = Vec::with_capacity(n);
    let mut clutch = Vec::with_capacity(n);
    let mut gear = Vec::with_capacity(n);
    let mut speed = Vec::with_capacity(n);
    let mut rpm = Vec::with_capacity(n);
    let mut drs = Vec::with_capacity(n);
    let mut drs_allowed = Vec::with_capacity(n);
    let mut ers_energy = Vec::with_capacity(n);
    let mut ers_mode = Vec::with_capacity(n);
    let mut fuel = Vec::with_capacity(n);
    let mut world_x = Vec::with_capacity(n);
    let mut world_y = Vec::with_capacity(n);
    let mut world_z = Vec::with_capacity(n);
    let mut vel_x = Vec::with_capacity(n);
    let mut vel_y = Vec::with_capacity(n);
    let mut vel_z = Vec::with_capacity(n);
    let mut yaw = Vec::with_capacity(n);
    let mut pitch = Vec::with_capacity(n);
    let mut roll = Vec::with_capacity(n);
    let mut g_lat = Vec::with_capacity(n);
    let mut g_long = Vec::with_capacity(n);
    let mut g_vert = Vec::with_capacity(n);
    let mut ts_fl = Vec::with_capacity(n);
    let mut ts_fr = Vec::with_capacity(n);
    let mut ts_rl = Vec::with_capacity(n);
    let mut ts_rr = Vec::with_capacity(n);
    let mut ti_fl = Vec::with_capacity(n);
    let mut ti_fr = Vec::with_capacity(n);
    let mut ti_rl = Vec::with_capacity(n);
    let mut ti_rr = Vec::with_capacity(n);
    let mut bt_fl = Vec::with_capacity(n);
    let mut bt_fr = Vec::with_capacity(n);
    let mut bt_rl = Vec::with_capacity(n);
    let mut bt_rr = Vec::with_capacity(n);
    let mut sp_fl = Vec::with_capacity(n);
    let mut sp_fr = Vec::with_capacity(n);
    let mut sp_rl = Vec::with_capacity(n);
    let mut sp_rr = Vec::with_capacity(n);
    let mut sv_fl = Vec::with_capacity(n);
    let mut sv_fr = Vec::with_capacity(n);
    let mut sv_rl = Vec::with_capacity(n);
    let mut sv_rr = Vec::with_capacity(n);
    let mut sa_fl = Vec::with_capacity(n);
    let mut sa_fr = Vec::with_capacity(n);
    let mut sa_rl = Vec::with_capacity(n);
    let mut sa_rr = Vec::with_capacity(n);
    let mut ws_fl = Vec::with_capacity(n);
    let mut ws_fr = Vec::with_capacity(n);
    let mut ws_rl = Vec::with_capacity(n);
    let mut ws_rr = Vec::with_capacity(n);

    for s in samples {
        timestamp.push(s.timestamp);
        frame.push(s.frame);
        lap_number.push(s.lap_number);
        lap_distance.push(s.lap_distance);
        total_distance.push(s.total_distance);
        throttle.push(s.throttle);
        brake.push(s.brake);
        steer.push(s.steer);
        clutch.push(s.clutch);
        gear.push(s.gear);
        speed.push(s.speed);
        rpm.push(s.rpm);
        drs.push(s.drs);
        drs_allowed.push(s.drs_allowed);
        ers_energy.push(s.ers_store_energy);
        ers_mode.push(s.ers_deploy_mode);
        fuel.push(s.fuel_in_tank);
        world_x.push(s.world_x);
        world_y.push(s.world_y);
        world_z.push(s.world_z);
        vel_x.push(s.velocity_x);
        vel_y.push(s.velocity_y);
        vel_z.push(s.velocity_z);
        yaw.push(s.yaw);
        pitch.push(s.pitch);
        roll.push(s.roll);
        g_lat.push(s.g_lateral);
        g_long.push(s.g_longitudinal);
        g_vert.push(s.g_vertical);
        ts_fl.push(s.tyre_surface_fl);
        ts_fr.push(s.tyre_surface_fr);
        ts_rl.push(s.tyre_surface_rl);
        ts_rr.push(s.tyre_surface_rr);
        ti_fl.push(s.tyre_inner_fl);
        ti_fr.push(s.tyre_inner_fr);
        ti_rl.push(s.tyre_inner_rl);
        ti_rr.push(s.tyre_inner_rr);
        bt_fl.push(s.brake_temp_fl);
        bt_fr.push(s.brake_temp_fr);
        bt_rl.push(s.brake_temp_rl);
        bt_rr.push(s.brake_temp_rr);
        sp_fl.push(s.suspension_pos_fl);
        sp_fr.push(s.suspension_pos_fr);
        sp_rl.push(s.suspension_pos_rl);
        sp_rr.push(s.suspension_pos_rr);
        sv_fl.push(s.suspension_vel_fl);
        sv_fr.push(s.suspension_vel_fr);
        sv_rl.push(s.suspension_vel_rl);
        sv_rr.push(s.suspension_vel_rr);
        sa_fl.push(s.suspension_acc_fl);
        sa_fr.push(s.suspension_acc_fr);
        sa_rl.push(s.suspension_acc_rl);
        sa_rr.push(s.suspension_acc_rr);
        ws_fl.push(s.wheel_speed_fl);
        ws_fr.push(s.wheel_speed_fr);
        ws_rl.push(s.wheel_speed_rl);
        ws_rr.push(s.wheel_speed_rr);
    }

    let columns: Vec<Arc<dyn arrow::array::Array>> = vec![
        Arc::new(Float32Array::from(timestamp)),
        Arc::new(UInt32Array::from(frame)),
        Arc::new(UInt8Array::from(lap_number)),
        Arc::new(Float32Array::from(lap_distance)),
        Arc::new(Float32Array::from(total_distance)),
        Arc::new(Float32Array::from(throttle)),
        Arc::new(Float32Array::from(brake)),
        Arc::new(Float32Array::from(steer)),
        Arc::new(UInt8Array::from(clutch)),
        Arc::new(Int8Array::from(gear)),
        Arc::new(UInt16Array::from(speed)),
        Arc::new(UInt16Array::from(rpm)),
        Arc::new(UInt8Array::from(drs)),
        Arc::new(UInt8Array::from(drs_allowed)),
        Arc::new(Float32Array::from(ers_energy)),
        Arc::new(UInt8Array::from(ers_mode)),
        Arc::new(Float32Array::from(fuel)),
        Arc::new(Float32Array::from(world_x)),
        Arc::new(Float32Array::from(world_y)),
        Arc::new(Float32Array::from(world_z)),
        Arc::new(Float32Array::from(vel_x)),
        Arc::new(Float32Array::from(vel_y)),
        Arc::new(Float32Array::from(vel_z)),
        Arc::new(Float32Array::from(yaw)),
        Arc::new(Float32Array::from(pitch)),
        Arc::new(Float32Array::from(roll)),
        Arc::new(Float32Array::from(g_lat)),
        Arc::new(Float32Array::from(g_long)),
        Arc::new(Float32Array::from(g_vert)),
        Arc::new(UInt8Array::from(ts_fl)),
        Arc::new(UInt8Array::from(ts_fr)),
        Arc::new(UInt8Array::from(ts_rl)),
        Arc::new(UInt8Array::from(ts_rr)),
        Arc::new(UInt8Array::from(ti_fl)),
        Arc::new(UInt8Array::from(ti_fr)),
        Arc::new(UInt8Array::from(ti_rl)),
        Arc::new(UInt8Array::from(ti_rr)),
        Arc::new(UInt16Array::from(bt_fl)),
        Arc::new(UInt16Array::from(bt_fr)),
        Arc::new(UInt16Array::from(bt_rl)),
        Arc::new(UInt16Array::from(bt_rr)),
        Arc::new(Float32Array::from(sp_fl)),
        Arc::new(Float32Array::from(sp_fr)),
        Arc::new(Float32Array::from(sp_rl)),
        Arc::new(Float32Array::from(sp_rr)),
        Arc::new(Float32Array::from(sv_fl)),
        Arc::new(Float32Array::from(sv_fr)),
        Arc::new(Float32Array::from(sv_rl)),
        Arc::new(Float32Array::from(sv_rr)),
        Arc::new(Float32Array::from(sa_fl)),
        Arc::new(Float32Array::from(sa_fr)),
        Arc::new(Float32Array::from(sa_rl)),
        Arc::new(Float32Array::from(sa_rr)),
        Arc::new(Float32Array::from(ws_fl)),
        Arc::new(Float32Array::from(ws_fr)),
        Arc::new(Float32Array::from(ws_rl)),
        Arc::new(Float32Array::from(ws_rr)),
    ];

    let batch = RecordBatch::try_new(schema.clone(), columns)?;
    let file = fs::File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(writer_props()))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

fn write_events_parquet(path: &Path, events: &[EventRecord]) -> anyhow::Result<()> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("timestamp", DataType::Float32, false),
        Field::new("frame", DataType::UInt32, false),
        Field::new("event_code", DataType::Utf8, false),
    ]));

    let batch = RecordBatch::try_new(schema.clone(), vec![
        Arc::new(Float32Array::from(events.iter().map(|e| e.timestamp).collect::<Vec<_>>())),
        Arc::new(UInt32Array::from(events.iter().map(|e| e.frame).collect::<Vec<_>>())),
        Arc::new(StringArray::from(events.iter().map(|e| e.event_code.as_str()).collect::<Vec<_>>())),
    ])?;

    let file = fs::File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(writer_props()))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

/// Full telemetry schema matching TelemetrySample fields.
pub fn telemetry_schema() -> Schema {
    Schema::new(vec![
        Field::new("timestamp", DataType::Float32, false),
        Field::new("frame", DataType::UInt32, false),
        Field::new("lap_number", DataType::UInt8, false),
        Field::new("lap_distance", DataType::Float32, false),
        Field::new("total_distance", DataType::Float32, false),
        Field::new("throttle", DataType::Float32, false),
        Field::new("brake", DataType::Float32, false),
        Field::new("steer", DataType::Float32, false),
        Field::new("clutch", DataType::UInt8, false),
        Field::new("gear", DataType::Int8, false),
        Field::new("speed", DataType::UInt16, false),
        Field::new("rpm", DataType::UInt16, false),
        Field::new("drs", DataType::UInt8, false),
        Field::new("drs_allowed", DataType::UInt8, false),
        Field::new("ers_store_energy", DataType::Float32, false),
        Field::new("ers_deploy_mode", DataType::UInt8, false),
        Field::new("fuel_in_tank", DataType::Float32, false),
        Field::new("world_x", DataType::Float32, false),
        Field::new("world_y", DataType::Float32, false),
        Field::new("world_z", DataType::Float32, false),
        Field::new("velocity_x", DataType::Float32, false),
        Field::new("velocity_y", DataType::Float32, false),
        Field::new("velocity_z", DataType::Float32, false),
        Field::new("yaw", DataType::Float32, false),
        Field::new("pitch", DataType::Float32, false),
        Field::new("roll", DataType::Float32, false),
        Field::new("g_lateral", DataType::Float32, false),
        Field::new("g_longitudinal", DataType::Float32, false),
        Field::new("g_vertical", DataType::Float32, false),
        Field::new("tyre_surface_fl", DataType::UInt8, false),
        Field::new("tyre_surface_fr", DataType::UInt8, false),
        Field::new("tyre_surface_rl", DataType::UInt8, false),
        Field::new("tyre_surface_rr", DataType::UInt8, false),
        Field::new("tyre_inner_fl", DataType::UInt8, false),
        Field::new("tyre_inner_fr", DataType::UInt8, false),
        Field::new("tyre_inner_rl", DataType::UInt8, false),
        Field::new("tyre_inner_rr", DataType::UInt8, false),
        Field::new("brake_temp_fl", DataType::UInt16, false),
        Field::new("brake_temp_fr", DataType::UInt16, false),
        Field::new("brake_temp_rl", DataType::UInt16, false),
        Field::new("brake_temp_rr", DataType::UInt16, false),
        Field::new("suspension_pos_fl", DataType::Float32, false),
        Field::new("suspension_pos_fr", DataType::Float32, false),
        Field::new("suspension_pos_rl", DataType::Float32, false),
        Field::new("suspension_pos_rr", DataType::Float32, false),
        Field::new("suspension_vel_fl", DataType::Float32, false),
        Field::new("suspension_vel_fr", DataType::Float32, false),
        Field::new("suspension_vel_rl", DataType::Float32, false),
        Field::new("suspension_vel_rr", DataType::Float32, false),
        Field::new("suspension_acc_fl", DataType::Float32, false),
        Field::new("suspension_acc_fr", DataType::Float32, false),
        Field::new("suspension_acc_rl", DataType::Float32, false),
        Field::new("suspension_acc_rr", DataType::Float32, false),
        Field::new("wheel_speed_fl", DataType::Float32, false),
        Field::new("wheel_speed_fr", DataType::Float32, false),
        Field::new("wheel_speed_rl", DataType::Float32, false),
        Field::new("wheel_speed_rr", DataType::Float32, false),
    ])
}
