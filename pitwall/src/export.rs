//! Export and session listing utilities.

use std::fs;
use std::path::Path;

use arrow::array::*;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use parquet::file::reader::FileReader;

use crate::storage::telemetry_schema;

/// Export a specific lap or show session summary.
pub fn export(session_dir: &Path, lap: Option<u16>, output: &Path) -> anyhow::Result<()> {
    match lap {
        Some(lap_num) => export_lap(session_dir, lap_num, output),
        None => show_summary(session_dir),
    }
}

fn export_lap(session_dir: &Path, lap_number: u16, output: &Path) -> anyhow::Result<()> {
    let telem_path = session_dir.join("telemetry.parquet");
    if !telem_path.exists() {
        anyhow::bail!("No telemetry at {}", telem_path.display());
    }

    let file = fs::File::open(&telem_path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let reader = builder.build()?;

    let schema = std::sync::Arc::new(telemetry_schema());
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(Default::default()))
        .build();

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let out_file = fs::File::create(output)?;
    let mut writer = ArrowWriter::try_new(out_file, schema, Some(props))?;

    let mut exported_rows = 0u64;

    for batch_result in reader {
        let batch = batch_result?;
        let lap_col = batch
            .column_by_name("lap_number")
            .and_then(|c| c.as_any().downcast_ref::<UInt8Array>());

        if let Some(laps) = lap_col {
            // Build filter mask
            let mask: BooleanArray = (0..batch.num_rows())
                .map(|i| Some(laps.value(i) as u16 == lap_number))
                .collect();

            let filtered = arrow::compute::filter_record_batch(&batch, &mask)?;
            if filtered.num_rows() > 0 {
                exported_rows += filtered.num_rows() as u64;
                writer.write(&filtered)?;
            }
        }
    }

    writer.close()?;
    println!("📦 Exported lap {lap_number}: {exported_rows} rows → {}", output.display());
    Ok(())
}

fn show_summary(session_dir: &Path) -> anyhow::Result<()> {
    println!("📊 Session: {}", session_dir.display());

    // Metadata
    let meta_path = session_dir.join("metadata.json");
    if meta_path.exists() {
        let meta: serde_json::Value = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
        if let Some(obj) = meta.as_object() {
            for (k, v) in obj {
                println!("   {k}: {v}");
            }
        }
    }

    // Laps
    let laps_path = session_dir.join("laps.parquet");
    if laps_path.exists() {
        let file = fs::File::open(&laps_path)?;
        let meta = parquet::file::reader::SerializedFileReader::new(file)?;
        let num_rows = meta.metadata().file_metadata().num_rows();
        println!("   laps: {num_rows}");
    }

    // Telemetry
    let telem_path = session_dir.join("telemetry.parquet");
    if telem_path.exists() {
        let file = fs::File::open(&telem_path)?;
        let meta = parquet::file::reader::SerializedFileReader::new(file)?;
        let num_rows = meta.metadata().file_metadata().num_rows();
        let size_mb = fs::metadata(&telem_path)?.len() as f64 / (1024.0 * 1024.0);
        println!("   telemetry: {num_rows} samples ({size_mb:.2} MB)");
    }

    Ok(())
}

/// List all recorded sessions.
pub fn list_sessions(data_dir: &Path) -> anyhow::Result<()> {
    let sessions_dir = data_dir.join("sessions");
    if !sessions_dir.exists() {
        println!("No sessions found in {}", data_dir.display());
        return Ok(());
    }

    let mut sessions = Vec::new();
    find_sessions_recursive(&sessions_dir, &mut sessions);

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    println!("📋 Found {} session(s):\n", sessions.len());

    for session_dir in &sessions {
        let name = session_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let track = session_dir
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        print!("  {track}/{name}");

        let telem_path = session_dir.join("telemetry.parquet");
        if telem_path.exists() {
            if let Ok(file) = fs::File::open(&telem_path) {
                if let Ok(meta) = parquet::file::reader::SerializedFileReader::new(file) {
                    let rows = meta.metadata().file_metadata().num_rows();
                    let size = fs::metadata(&telem_path).map(|m| m.len()).unwrap_or(0);
                    print!(" | {rows} samples | {:.1} MB", size as f64 / 1_048_576.0);
                }
            }
        }
        println!();
    }

    Ok(())
}

fn find_sessions_recursive(dir: &Path, results: &mut Vec<std::path::PathBuf>) {
    if dir.join("metadata.json").exists() {
        results.push(dir.to_path_buf());
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_sessions_recursive(&path, results);
            }
        }
    }
}
