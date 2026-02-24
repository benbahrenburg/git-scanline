use std::fs;
use std::io::Write;
use std::path::Path;
use crate::types::Report;

/// Outputs the report as JSON. Writes to a file if given, otherwise stdout.
pub fn report_json(report: &Report, output_file: Option<&Path>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| format!("JSON serialization failed: {e}"))?;

    if let Some(path) = output_file {
        fs::write(path, &json)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        eprintln!("✓ JSON report written to {}", path.display());
    } else {
        std::io::stdout().write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write stdout: {e}"))?;
        println!();
    }

    Ok(())
}
