use serde::Deserialize;
use std::path::Path;

/// All settings that can be placed in a .git-scanline.yml config file.
/// Every field is optional — omitted fields fall back to CLI defaults.
/// CLI flags always take precedence over values set here.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanlineConfig {
    // Analysis defaults (overridden by the corresponding CLI flag)
    pub since: Option<String>,
    pub path: Option<String>,
    pub top: Option<usize>,
    pub bugs_only: Option<bool>,
    pub format: Option<String>,
    pub output: Option<String>,

    // File-filter overrides
    pub exclude_dirs: Option<Vec<String>>,
    pub include_dirs: Option<Vec<String>>,
    pub exclude_files: Option<Vec<String>>,
    pub exclude_extensions: Option<Vec<String>>,

    // Scoring weight overrides
    pub weights: Option<ConfigWeights>,
}

/// Optional per-signal weight overrides. All weights are normalized at runtime.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigWeights {
    pub churn: Option<f64>,
    pub bugs: Option<f64>,
    pub reverts: Option<f64>,
    pub bursts: Option<f64>,
    pub coupling: Option<f64>,
    pub silo: Option<f64>,
    pub commit_quality: Option<f64>,
}

/// Resolved filter customizations extracted from [`ScanlineConfig`] and
/// threaded into [`crate::filters::filter_files`].
#[derive(Debug, Default)]
pub struct FilterOverrides {
    /// Directory names to add to the built-in exclusion list.
    pub extra_exclude_dirs: Vec<String>,
    /// Directory names to remove from the built-in exclusion list.
    pub allow_dirs: Vec<String>,
    /// Exact filenames to add to the built-in exclusion list.
    pub extra_exclude_files: Vec<String>,
    /// File extensions to add to the built-in exclusion list.
    pub extra_exclude_extensions: Vec<String>,
}

impl ScanlineConfig {
    /// Extracts the filter-related fields into a [`FilterOverrides`] value.
    pub fn filter_overrides(&self) -> FilterOverrides {
        FilterOverrides {
            extra_exclude_dirs: self.exclude_dirs.clone().unwrap_or_default(),
            allow_dirs: self.include_dirs.clone().unwrap_or_default(),
            extra_exclude_files: self.exclude_files.clone().unwrap_or_default(),
            extra_exclude_extensions: self.exclude_extensions.clone().unwrap_or_default(),
        }
    }

    /// Validates semantic constraints that serde cannot enforce.
    ///
    /// Returns a human-readable error describing exactly what is wrong and what
    /// values are accepted. Called automatically by [`load_config`].
    pub fn validate(&self) -> Result<(), String> {
        // format must be one of the three supported output drivers
        if let Some(fmt) = &self.format {
            match fmt.as_str() {
                "terminal" | "json" | "html" => {}
                other => {
                    return Err(format!(
                        "Invalid 'format' value: \"{other}\". \
                         Expected one of: \"terminal\", \"json\", \"html\""
                    ))
                }
            }
        }

        // top: 0 would silently produce an empty report — almost certainly a mistake
        if let Some(0) = self.top {
            return Err("Invalid 'top' value: 0. \
                 Must be 1 or greater (use --bugs-only to narrow results instead)"
                .to_string());
        }

        // Weights must be positive and finite.
        // Zero or negative makes no sense (they are normalized, so only ratios matter).
        if let Some(w) = &self.weights {
            let fields: &[(&str, Option<f64>)] = &[
                ("churn", w.churn),
                ("bugs", w.bugs),
                ("reverts", w.reverts),
                ("bursts", w.bursts),
                ("coupling", w.coupling),
                ("silo", w.silo),
                ("commit_quality", w.commit_quality),
            ];
            for (name, val) in fields {
                if let Some(v) = val {
                    if !v.is_finite() {
                        return Err(format!(
                            "Invalid weight 'weights.{name}': {v} is not a finite number"
                        ));
                    }
                    if *v <= 0.0 {
                        return Err(format!(
                            "Invalid weight 'weights.{name}': {v}. \
                             Weights must be greater than 0. \
                             They are normalized automatically, so only the ratios matter \
                             (e.g. churn: 2 and bugs: 1 makes churn twice as influential)"
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Reads, parses, and validates a YAML config file from `path`.
pub fn load_config(path: &Path) -> Result<ScanlineConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read config file '{}': {e}", path.display()))?;
    let cfg: ScanlineConfig = serde_yaml::from_str(&content)
        .map_err(|e| format!("Invalid config file '{}': {e}", path.display()))?;
    cfg.validate()
        .map_err(|e| format!("Config file '{}': {e}", path.display()))?;
    Ok(cfg)
}

/// Annotated YAML template — printed by `--generate-config`.
pub static TEMPLATE: &str = r#"# git-scanline configuration file
# Generated by: git-scanline --generate-config
#
# All settings are optional. Omit any field to use the built-in default.
# CLI flags always take precedence over values in this file.
# Save this file as .git-scanline.yml in your repository root, then run:
#
#   git-scanline --config .git-scanline.yml [path]

# ── Analysis scope ─────────────────────────────────────────────────────────────

# Analyze commits since this date. Leave empty (or omit) for all history.
# Accepts any git date format: "6 months ago", "2024-01-01", "1 year ago"
# since: ""

# Limit analysis to a subdirectory (relative path from the repo root).
# Equivalent to --path.
# path: "src"

# Number of hotspot results to display. All files are always analyzed.
# top: 20

# Only show files that appear in bug-fix commits.
# bugs_only: false

# ── Output ─────────────────────────────────────────────────────────────────────

# Output format: terminal, json, html
# format: "terminal"

# Output file path. For HTML, defaults to ~/Desktop/hotspot-report.html
# output: "hotspot-report.json"

# ── File filtering ─────────────────────────────────────────────────────────────

# Additional directories to exclude (merged with the built-in list).
# exclude_dirs:
#   - "generated"
#   - "proto"
#   - "migrations"
#   - "fixtures"

# Built-in excluded directories to allow back into analysis.
# Useful when a normally-noise directory contains real source code.
# include_dirs:
#   - "dist"
#   - "public"

# Additional filenames to exclude (exact match against the filename, not full path).
# exclude_files:
#   - "schema.graphql"
#   - "openapi.json"

# Additional file extensions to exclude.
# exclude_extensions:
#   - ".pb.go"
#   - ".generated.ts"
#   - ".d.ts"

# ── Scoring weights ────────────────────────────────────────────────────────────
# All weights are normalized at runtime so they always sum to 1.0.
# Increase a weight to emphasize that signal; decrease to de-emphasize it.

# weights:
#   churn:          0.27   # Commit frequency with recency decay
#   bugs:           0.27   # Correlation with bug-fix commit messages
#   reverts:        0.14   # Files that have been reverted
#   bursts:         0.09   # Rapid-commit windows (many commits in 24 h)
#   coupling:       0.09   # Files that always change together
#   silo:           0.05   # Single-author concentration risk
#   commit_quality: 0.09   # WIP and oversized commits
"#;

/// Prints the config template to stdout, or writes it to `output_path` if given.
pub fn print_template(output_path: Option<&Path>) -> Result<(), String> {
    match output_path {
        Some(path) => std::fs::write(path, TEMPLATE)
            .map_err(|e| format!("Cannot write config template to '{}': {e}", path.display())),
        None => {
            print!("{TEMPLATE}");
            Ok(())
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_template_is_valid_yaml() {
        let result: Result<ScanlineConfig, _> = serde_yaml::from_str(TEMPLATE);
        assert!(
            result.is_ok(),
            "TEMPLATE must parse as valid ScanlineConfig: {:?}",
            result.err()
        );
        let cfg = result.unwrap();
        // All fields should be None (everything is commented out in the template)
        assert!(cfg.since.is_none());
        assert!(cfg.top.is_none());
        assert!(cfg.weights.is_none());
    }

    #[test]
    fn test_empty_config_is_valid() {
        let cfg: ScanlineConfig = serde_yaml::from_str("{}").expect("empty map should parse");
        assert!(cfg.since.is_none());
        assert!(cfg.path.is_none());
        assert!(cfg.top.is_none());
        assert!(cfg.exclude_dirs.is_none());
        assert!(cfg.weights.is_none());
    }

    #[test]
    fn test_exclude_dirs_parsed() {
        let yaml = "exclude_dirs:\n  - proto\n  - generated\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        let dirs = cfg.exclude_dirs.expect("exclude_dirs should be Some");
        assert!(dirs.contains(&"proto".to_string()));
        assert!(dirs.contains(&"generated".to_string()));
    }

    #[test]
    fn test_include_dirs_parsed() {
        let yaml = "include_dirs:\n  - dist\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        let dirs = cfg.include_dirs.expect("include_dirs should be Some");
        assert!(dirs.contains(&"dist".to_string()));
    }

    #[test]
    fn test_weights_parsed() {
        let yaml = "weights:\n  churn: 0.5\n  bugs: 0.5\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        let w = cfg.weights.expect("weights should be Some");
        assert!((w.churn.unwrap() - 0.5).abs() < 1e-9);
        assert!((w.bugs.unwrap() - 0.5).abs() < 1e-9);
        assert!(w.reverts.is_none());
    }

    #[test]
    fn test_analysis_defaults_parsed() {
        let yaml = "since: \"6 months ago\"\ntop: 10\nbugs_only: true\nformat: json\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        assert_eq!(cfg.since.as_deref(), Some("6 months ago"));
        assert_eq!(cfg.top, Some(10));
        assert_eq!(cfg.bugs_only, Some(true));
        assert_eq!(cfg.format.as_deref(), Some("json"));
    }

    #[test]
    fn test_unknown_field_rejected() {
        let yaml = "unknown_setting: true\n";
        let result: Result<ScanlineConfig, _> = serde_yaml::from_str(yaml);
        assert!(
            result.is_err(),
            "Unknown fields should be rejected by deny_unknown_fields"
        );
    }

    #[test]
    fn test_filter_overrides_empty_by_default() {
        let cfg = ScanlineConfig::default();
        let fo = cfg.filter_overrides();
        assert!(fo.extra_exclude_dirs.is_empty());
        assert!(fo.allow_dirs.is_empty());
        assert!(fo.extra_exclude_files.is_empty());
        assert!(fo.extra_exclude_extensions.is_empty());
    }

    #[test]
    fn test_filter_overrides_populated() {
        let yaml =
            "exclude_dirs:\n  - proto\ninclude_dirs:\n  - dist\nexclude_extensions:\n  - .pb.go\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        let fo = cfg.filter_overrides();
        assert_eq!(fo.extra_exclude_dirs, vec!["proto"]);
        assert_eq!(fo.allow_dirs, vec!["dist"]);
        assert_eq!(fo.extra_exclude_extensions, vec![".pb.go"]);
    }

    // ── validate() tests ──────────────────────────────────────────────────────

    #[test]
    fn test_validate_valid_config_passes() {
        let yaml = "format: \"json\"\ntop: 10\nweights:\n  churn: 0.5\n  bugs: 0.5\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        assert!(
            cfg.validate().is_ok(),
            "Valid config should pass validation"
        );
    }

    #[test]
    fn test_validate_invalid_format_rejected() {
        let yaml = "format: \"csv\"\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        let result = cfg.validate();
        assert!(result.is_err(), "Invalid format should be rejected");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("format"),
            "Error should mention 'format': {msg}"
        );
        assert!(
            msg.contains("terminal") && msg.contains("json") && msg.contains("html"),
            "Error should list all valid values: {msg}"
        );
    }

    #[test]
    fn test_validate_zero_top_rejected() {
        let yaml = "top: 0\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        let result = cfg.validate();
        assert!(result.is_err(), "top: 0 should be rejected");
        let msg = result.unwrap_err();
        assert!(msg.contains("top"), "Error should mention 'top': {msg}");
    }

    #[test]
    fn test_validate_negative_weight_rejected() {
        let yaml = "weights:\n  churn: -0.5\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        let result = cfg.validate();
        assert!(result.is_err(), "Negative weight should be rejected");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("churn"),
            "Error should name the invalid field: {msg}"
        );
        assert!(
            msg.contains("greater than 0"),
            "Error should explain the requirement: {msg}"
        );
    }

    #[test]
    fn test_validate_zero_weight_rejected() {
        let yaml = "weights:\n  bugs: 0.0\n";
        let cfg: ScanlineConfig = serde_yaml::from_str(yaml).expect("should parse");
        let result = cfg.validate();
        assert!(result.is_err(), "Zero weight should be rejected");
        let msg = result.unwrap_err();
        assert!(msg.contains("bugs"), "Error should name the field: {msg}");
    }

    #[test]
    fn test_validate_all_weight_fields_checked() {
        // Each weight field is independently validated
        let field_names = [
            "churn",
            "bugs",
            "reverts",
            "bursts",
            "coupling",
            "silo",
            "commit_quality",
        ];
        for field in field_names {
            let yaml = format!("weights:\n  {field}: -1.0\n");
            let cfg: ScanlineConfig = serde_yaml::from_str(&yaml).expect("should parse");
            let result = cfg.validate();
            assert!(
                result.is_err(),
                "Negative weight for '{field}' should be rejected"
            );
            assert!(
                result.unwrap_err().contains(field),
                "Error for '{field}' should name the field"
            );
        }
    }

    // ── Example file test ─────────────────────────────────────────────────────

    #[test]
    fn test_load_example_file() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let example_path = manifest_dir.join(".git-scanline.example.yml");

        let cfg = load_config(&example_path).unwrap_or_else(|e| {
            panic!("Example config file should parse and validate successfully: {e}")
        });

        // Analysis scope
        assert_eq!(
            cfg.since.as_deref(),
            Some("6 months ago"),
            "since should match example file"
        );
        assert_eq!(
            cfg.path.as_deref(),
            Some("src"),
            "path should match example file"
        );
        assert_eq!(cfg.top, Some(20), "top should match example file");
        assert_eq!(
            cfg.bugs_only,
            Some(false),
            "bugs_only should match example file"
        );
        assert_eq!(
            cfg.format.as_deref(),
            Some("terminal"),
            "format should match example file"
        );

        // File filtering
        let dirs = cfg
            .exclude_dirs
            .as_ref()
            .expect("exclude_dirs should be set in example file");
        assert!(
            dirs.contains(&"generated".to_string()),
            "exclude_dirs should contain 'generated'"
        );
        assert!(
            dirs.contains(&"proto".to_string()),
            "exclude_dirs should contain 'proto'"
        );
        assert!(
            dirs.contains(&"migrations".to_string()),
            "exclude_dirs should contain 'migrations'"
        );
        assert!(
            dirs.contains(&"fixtures".to_string()),
            "exclude_dirs should contain 'fixtures'"
        );

        let incl = cfg
            .include_dirs
            .as_ref()
            .expect("include_dirs should be set in example file");
        assert!(
            incl.contains(&"dist".to_string()),
            "include_dirs should contain 'dist'"
        );

        let files = cfg
            .exclude_files
            .as_ref()
            .expect("exclude_files should be set in example file");
        assert!(
            files.contains(&"schema.graphql".to_string()),
            "exclude_files should contain 'schema.graphql'"
        );
        assert!(
            files.contains(&"openapi.json".to_string()),
            "exclude_files should contain 'openapi.json'"
        );

        let exts = cfg
            .exclude_extensions
            .as_ref()
            .expect("exclude_extensions should be set in example file");
        assert!(
            exts.contains(&".pb.go".to_string()),
            "exclude_extensions should contain '.pb.go'"
        );
        assert!(
            exts.contains(&".generated.ts".to_string()),
            "exclude_extensions should contain '.generated.ts'"
        );
        assert!(
            exts.contains(&".d.ts".to_string()),
            "exclude_extensions should contain '.d.ts'"
        );

        // Weights
        let w = cfg
            .weights
            .as_ref()
            .expect("weights should be set in example file");
        assert!(
            (w.churn.unwrap() - 0.27).abs() < 1e-9,
            "churn weight should be 0.27"
        );
        assert!(
            (w.bugs.unwrap() - 0.40).abs() < 1e-9,
            "bugs weight should be 0.40"
        );
        assert!(
            (w.commit_quality.unwrap() - 0.09).abs() < 1e-9,
            "commit_quality weight should be 0.09"
        );

        // Filter overrides roundtrip
        let fo = cfg.filter_overrides();
        assert!(fo.extra_exclude_dirs.contains(&"proto".to_string()));
        assert!(fo.allow_dirs.contains(&"dist".to_string()));
        assert!(fo.extra_exclude_extensions.contains(&".d.ts".to_string()));
    }
}
