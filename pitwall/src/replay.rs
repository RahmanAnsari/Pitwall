//! Telemetry replay - reconstruct any lap frame-by-frame from stored Parquet.

use std::path::Path;

use arrow::array::*;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

pub fn replay_lap(session_dir: &Path, lap_number: u16) -> anyhow::Result<()> {
    let telem_path = session_dir.join("telemetry.parquet");
    if !telem_path.exists() {
        anyhow::bail!("No telemetry file found at {}", telem_path.display());
    }

    let file = std::fs::File::open(&telem_path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let reader = builder.build()?;

    println!("🔄 Replaying lap {lap_number}");
    println!("{:-<70}", "");

    let mut frame_count = 0u64;
    let mut first_ts: Option<f32> = None;
    let mut last_ts: f32 = 0.0;

    for batch_result in reader {
        let batch = batch_result?;
        let num_rows = batch.num_rows();

        let lap_col = batch
            .column_by_name("lap_number")
            .and_then(|c| c.as_any().downcast_ref::<UInt8Array>());
        let ts_col = batch
            .column_by_name("timestamp")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
        let speed_col = batch
            .column_by_name("speed")
            .and_then(|c| c.as_any().downcast_ref::<UInt16Array>());
        let throttle_col = batch
            .column_by_name("throttle")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
        let brake_col = batch
            .column_by_name("brake")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
        let gear_col = batch
            .column_by_name("gear")
            .and_then(|c| c.as_any().downcast_ref::<Int8Array>());
        let dist_col = batch
            .column_by_name("lap_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

        for i in 0..num_rows {
            let lap = lap_col.map(|c| c.value(i)).unwrap_or(0);
            if lap as u16 != lap_number {
                continue;
            }

            let ts = ts_col.map(|c| c.value(i)).unwrap_or(0.0);
            let speed = speed_col.map(|c| c.value(i)).unwrap_or(0);
            let throttle = throttle_col.map(|c| c.value(i)).unwrap_or(0.0);
            let brake = brake_col.map(|c| c.value(i)).unwrap_or(0.0);
            let gear = gear_col.map(|c| c.value(i)).unwrap_or(0);
            let dist = dist_col.map(|c| c.value(i)).unwrap_or(0.0);

            if first_ts.is_none() {
                first_ts = Some(ts);
            }
            last_ts = ts;

            println!(
                "  [{frame_count:5}] t={ts:8.3}s dist={dist:7.1}m spd={speed:3}km/h gear={gear} thr={throttle:.2} brk={brake:.2}"
            );
            frame_count += 1;
        }
    }

    let duration = last_ts - first_ts.unwrap_or(0.0);
    println!("{:-<70}", "");
    println!("  {frame_count} frames, {duration:.1}s duration");

    Ok(())
}
