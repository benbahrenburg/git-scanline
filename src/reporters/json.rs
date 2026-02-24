use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use crate::types::Report;

/// Outputs the report as JSON. Writes to a file if given, otherwise stdout.
pub fn report_json(report: &Report, output_file: Option<&Path>) -> Result<(), String> {
    if let Some(path) = output_file {
        let file = File::create(path)
            .map_err(|e| format!("Failed to open {} for writing: {e}", path.display()))?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, report)
            .map_err(|e| format!("JSON serialization failed: {e}"))?;
        writer
            .write_all(b"\n")
            .map_err(|e| format!("Failed to finalize {}: {e}", path.display()))?;
        eprintln!("✓ JSON report written to {}", path.display());
    } else {
        let stdout = std::io::stdout();
        let mut writer = BufWriter::new(stdout.lock());
        serde_json::to_writer_pretty(&mut writer, report)
            .map_err(|e| format!("JSON serialization failed: {e}"))?;
        writer
            .write_all(b"\n")
            .map_err(|e| format!("Failed to write stdout: {e}"))?;
    }

    Ok(())
}
