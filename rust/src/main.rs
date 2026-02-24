mod types;
mod git;
mod analyzers;
mod scoring;
mod filters;
mod reporters;
mod animation;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use types::*;


#[derive(Parser, Debug)]
#[command(
    name = "git-scanline",
    about = "🔥 Scan git history to surface bug-prone code hotspots",
    version,
    long_about = "Scans your local git history to surface code hotspots.\n\n\
                  Accepts a git repo directory OR a parent folder — when a parent\n\
                  folder is given, all nested git repos are discovered and analyzed\n\
                  independently with the same settings.\n\n\
                  Tip: Drag any folder onto this executable to analyze it."
)]
struct Args {
    /// Path to a git repository OR a parent folder containing multiple repos.
    #[arg(value_name = "PATH")]
    repo_path: Option<PathBuf>,

    /// Leave empty (default) to include all history, or e.g. "6 months ago", "2024-01-01"
    #[arg(long, default_value = "")]
    since: String,

    #[arg(long)]
    path: Option<String>,

    #[arg(long, default_value_t = 20)]
    top: usize,

    #[arg(long)]
    bugs_only: bool,

    /// Output format: terminal, json, html
    #[arg(long, default_value = "terminal")]
    format: String,

    /// Output file (single repo). For multiple repos, repo names are appended automatically.
    /// For --format html, defaults to ~/Desktop/hotspot-report.html
    #[arg(long)]
    output: Option<PathBuf>,

    #[arg(long)]
    no_interactive: bool,

    #[arg(long = "weight-churn",          default_value_t = 0.27)] weight_churn:          f64,
    #[arg(long = "weight-bugs",           default_value_t = 0.27)] weight_bugs:           f64,
    #[arg(long = "weight-reverts",        default_value_t = 0.14)] weight_reverts:        f64,
    #[arg(long = "weight-bursts",         default_value_t = 0.09)] weight_bursts:         f64,
    #[arg(long = "weight-coupling",       default_value_t = 0.09)] weight_coupling:       f64,
    #[arg(long = "weight-silo",           default_value_t = 0.05)] weight_silo:           f64,
    #[arg(long = "weight-commit-quality", default_value_t = 0.09)] weight_commit_quality: f64,
}

fn main() {
    let mut args = Args::parse();

    let explicit_args = std::env::args().len() > 1;
    let run_interactive_mode = args.repo_path.is_none() && !args.no_interactive && !explicit_args;

    if run_interactive_mode {
        args = run_interactive(args);
    } else if args.repo_path.is_none() {
        args.repo_path = Some(std::env::current_dir().expect("Failed to get current directory"));
    }

    loop {
        let input_path = args.repo_path.as_ref().unwrap().clone();

        if !input_path.exists() {
            eprintln!("Error: path does not exist: {}", input_path.display());
            if run_interactive_mode { wait_for_enter(); }
            std::process::exit(1);
        }

        // ── Discover all git repos under the given path ────────────────────────
        let repos = find_git_repos(&input_path);
        if repos.is_empty() {
            eprintln!("Error: No git repositories found under: {}", input_path.display());
            eprintln!("       Make sure the path contains a .git directory.");
            if run_interactive_mode { wait_for_enter(); }
            std::process::exit(1);
        }

        // Animate ZORP then freeze it in place — it stays visible at the top while
        // the spinner and report output appear below.
        if args.format == "terminal" {
            animation::start_zorp().freeze();
        }

        let is_multi = repos.len() > 1;
        if is_multi {
            eprintln!("🔍 Found {} git repositories:", repos.len());
            for r in &repos {
                eprintln!("   • {}", r.display());
            }
            eprintln!();
        }

        // ── Normalize weights ────────────────────────────────────────────────────
        let raw = Weights {
            churn:          args.weight_churn,
            bugs:           args.weight_bugs,
            reverts:        args.weight_reverts,
            bursts:         args.weight_bursts,
            coupling:       args.weight_coupling,
            silo:           args.weight_silo,
            commit_quality: args.weight_commit_quality,
        };
        let wsum = raw.churn + raw.bugs + raw.reverts + raw.bursts + raw.coupling + raw.silo + raw.commit_quality;
        let weights = Weights {
            churn:          raw.churn          / wsum,
            bugs:           raw.bugs           / wsum,
            reverts:        raw.reverts        / wsum,
            bursts:         raw.bursts         / wsum,
            coupling:       raw.coupling       / wsum,
            silo:           raw.silo           / wsum,
            commit_quality: raw.commit_quality / wsum,
        };

        // ── Base output path (used for single repo or as template for multi) ─────
        let base_output: Option<PathBuf> = match args.format.as_str() {
            "html" => Some(args.output.clone().unwrap_or_else(|| {
                dirs::desktop_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("hotspot-report.html")
            })),
            _ => args.output.clone(),
        };

        // ── Analyze each repo ────────────────────────────────────────────────────
        for repo_path in &repos {
            let repo_name = repo_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repo");

            // Resolve output path for this repo
            let output_path = base_output.as_deref().map(|base| {
                if is_multi { make_output_path(base, repo_name) } else { base.to_path_buf() }
            });

            if is_multi {
                eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                eprintln!("  Analyzing: {} ({})", repo_name, repo_path.display());
                eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            }

            if let Err(e) = run_analysis(
                repo_path,
                repo_name,
                &args,
                &weights,
                output_path.as_deref(),
                is_multi,
                run_interactive_mode,
            ) {
                eprintln!("Error analyzing {}: {}", repo_name, e);
            }
        }

        // Print ZORP as a footer after all report output
        if args.format == "terminal" {
            animation::print_zorp_footer();
        }

        // ── Offer to analyze another repo ────────────────────────────────────────
        if run_interactive_mode {
            println!();
            let answer = prompt("  Analyze another repo? [no]: ");
            if matches!(answer.trim().to_lowercase().as_str(), "yes" | "y") {
                args.repo_path = None;
                args = run_interactive(args);
                continue;
            }
            wait_for_enter();
        }
        break;
    }
}

// ── Analysis pipeline ──────────────────────────────────────────────────────────

fn run_analysis(
    repo_path:        &Path,
    repo_name:        &str,
    args:             &Args,
    weights:          &Weights,
    output_path:      Option<&Path>,
    is_multi:         bool,
    interactive_mode: bool,
) -> Result<(), String> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.enable_steady_tick(Duration::from_millis(80));

    let pfx = if is_multi { format!("[{}] ", repo_name) } else { String::new() };

    let total_start = Instant::now();
    let mut step_start = Instant::now();

    pb.set_message(format!("{}[1/5] Parsing commit log + diff stats...", pfx));
    let (commits, diff_stats) = match git::log_parser::parse_log(repo_path, &args.since, args.path.as_deref()) {
        Ok((c, _)) if c.is_empty() => {
            pb.finish_and_clear();
            return Err(format!(
                "No commits found in '{}'. Try --since=\"4 years ago\"",
                repo_path.display()
            ));
        }
        Ok((c, d)) => (c, d),
        Err(e) => {
            pb.finish_and_clear();
            return Err(e.to_string());
        }
    };
    let t1 = fmt_dur(step_start.elapsed()); step_start = Instant::now();
    pb.println(format!("  ✓ [1/5] Parsing commit log + diff stats       {t1}"));

    pb.set_message(format!("{}[2/5] Scanning for security risks...", pfx));
    let security_risks = analyzers::security::analyze_security(&commits);
    let t2 = fmt_dur(step_start.elapsed()); step_start = Instant::now();
    pb.println(format!("  ✓ [2/5] Scanning for security risks           {t2}"));

    pb.set_message(format!("{}[3/5] Filtering files...", pfx));
    let all_files: HashSet<String> = commits.iter()
        .flat_map(|c| c.files.iter().cloned())
        .collect();
    let filtered_files = filters::filter_files(
        &all_files.into_iter().collect::<Vec<_>>(),
        args.path.as_deref(),
    );
    if filtered_files.is_empty() {
        pb.finish_and_clear();
        return Err("No files found after filtering. Try --path or --since.".to_string());
    }
    let t3 = fmt_dur(step_start.elapsed()); step_start = Instant::now();
    pb.println(format!("  ✓ [3/5] Filtering files                       {t3}"));

    pb.set_message(format!("{}[4/5] Running all analyzers in parallel...", pfx));
    let (
        (churn_data, (bug_data, revert_data)),
        (burst_data, (coupling_data, (silo_data, commit_quality_data)))
    ) = rayon::join(
        || rayon::join(
            || analyzers::churn::analyze_churn(&commits, &filtered_files),
            || rayon::join(
                || analyzers::bug_correlation::analyze_bug_correlation(&commits, &filtered_files),
                || analyzers::revert_tracker::analyze_reverts(&commits, &filtered_files),
            ),
        ),
        || rayon::join(
            || analyzers::burst_detector::analyze_bursts(&commits, &filtered_files),
            || rayon::join(
                || analyzers::coupling::analyze_coupling(&commits, &filtered_files),
                || rayon::join(
                    || analyzers::blame::analyze_authors(&commits, &filtered_files),
                    || analyzers::commit_quality::analyze_commit_quality(&commits, &filtered_files),
                ),
            ),
        ),
    );
    let t4 = fmt_dur(step_start.elapsed()); step_start = Instant::now();
    pb.println(format!("  ✓ [4/5] All 7 analyzers (parallel)            {t4}"));

    pb.set_message(format!("{}[5/5] Scoring hotspots...", pfx));
    let mut results = scoring::score_hotspots(
        &filtered_files, &churn_data, &bug_data, &revert_data,
        &burst_data, &coupling_data, &silo_data, &commit_quality_data,
        &diff_stats, weights,
    );
    results.sort_by(|a, b| b.hotspot_score.partial_cmp(&a.hotspot_score).unwrap_or(std::cmp::Ordering::Equal));
    if args.bugs_only { results.retain(|r| r.details.bug_commits > 0); }
    results.truncate(args.top);

    let file_set: HashSet<&str> = filtered_files.iter().map(|s| s.as_str()).collect();
    let top_couplings: Vec<CouplingEntry> = coupling_data.into_iter()
        .filter(|c| file_set.contains(c.file_a.as_str()) && file_set.contains(c.file_b.as_str()))
        .take(10)
        .collect();

    let t5 = fmt_dur(step_start.elapsed());
    pb.println(format!("  ✓ [5/5] Scoring hotspots                      {t5}"));
    let total_time = fmt_dur(total_start.elapsed());

    pb.finish_and_clear();
    eprintln!("✔ [{}] {} commits, {} files — ⏱ {}{}",
        repo_name,
        commits.len(),
        filtered_files.len(),
        total_time,
        if security_risks.is_empty() { String::new() } else {
            format!(" — ⚠ {} security risk(s)", security_risks.len())
        }
    );

    let report = Report {
        meta: ReportMeta {
            since:        if args.since.is_empty() { "all history".to_string() } else { args.since.clone() },
            commit_count: commits.len(),
            file_count:   filtered_files.len(),
            analyzed_at:  chrono::Utc::now().to_rfc3339(),
            repo_path:    repo_path.display().to_string(),
        },
        results,
        couplings: top_couplings,
        security_risks,
    };

    match args.format.as_str() {
        "json" => reporters::json::report_json(&report, output_path)?,
        "html" => {
            let path = output_path.ok_or("output path required for html")?;
            reporters::html::report_html(&report, path)?;
        }
        _ => {
            if is_multi {
                // Print a visible repo header before the terminal report
                println!();
                println!("╔══════════════════════════════════════════════════════╗");
                println!("║  {}  {}",
                    format!("📁 {}", repo_name),
                    format!("({})", repo_path.display())
                        .chars().take(45).collect::<String>()
                );
                println!("╚══════════════════════════════════════════════════════╝");
            }
            reporters::terminal::report_terminal(&report);
        }
    }

    // In file mode + multi-repo, confirm where file was written
    if is_multi && args.format != "terminal" {
        if let Some(p) = output_path {
            eprintln!("   → {}", p.display());
        }
    }

    let _ = interactive_mode; // pause is handled once in main after all repos
    Ok(())
}

// ── Duration formatting ────────────────────────────────────────────────────────

fn fmt_dur(d: Duration) -> String {
    let ms = d.as_millis();
    if ms >= 1000 { format!("{:.1}s", d.as_secs_f64()) } else { format!("{ms}ms") }
}

// ── Git repo discovery ─────────────────────────────────────────────────────────

/// Recursively finds all git repository roots under `root`.
/// Stops descending into a directory once a `.git` folder is found.
pub fn find_git_repos(root: &Path) -> Vec<PathBuf> {
    // If the root itself is a git repo, return it immediately
    if root.join(".git").exists() {
        return vec![root.to_path_buf()];
    }
    let mut repos = Vec::new();
    scan_for_repos(root, 0, &mut repos);
    repos.sort();
    repos
}

const SKIP_SCAN_DIRS: &[&str] = &[
    "node_modules", "vendor", "target", "dist", "build",
    ".cache", ".git", "__pycache__", ".npm", ".yarn",
];

fn scan_for_repos(dir: &Path, depth: usize, repos: &mut Vec<PathBuf>) {
    if depth > 6 { return; }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Skip hidden dirs and known noise dirs
        if name.starts_with('.') || SKIP_SCAN_DIRS.contains(&name) { continue; }
        if path.join(".git").exists() {
            repos.push(path);
            // Don't recurse into a repo — nested repos are uncommon and confusing
        } else {
            scan_for_repos(&path, depth + 1, repos);
        }
    }
}

// ── Output path helpers ────────────────────────────────────────────────────────

/// Given a base output path and a repo name, insert the repo name before the extension.
/// e.g. `hotspot-report.html` + `my-app` → `hotspot-report-my-app.html`
fn make_output_path(base: &Path, repo_name: &str) -> PathBuf {
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("hotspot");
    let ext  = base.extension().and_then(|s| s.to_str()).unwrap_or("html");
    let dir  = base.parent().unwrap_or(Path::new("."));
    let safe: String = repo_name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    dir.join(format!("{stem}-{safe}.{ext}"))
}

// ── Interactive setup ──────────────────────────────────────────────────────────

fn run_interactive(mut args: Args) -> Args {
    // ZORP is the welcome screen; stop() waits for MIN_DISPLAY_MS then clears.
    animation::start_zorp().stop();
    println!("  Interactive Setup");
    println!("  Accepts a single repo OR a parent folder containing multiple repos.");
    println!("  Tip: Drag any folder into this window to insert its path.");
    println!("  Press Enter to accept [defaults], or type a new value.\n");

    // ── Input path: loop until valid ───────────────────────────────────────────
    loop {
        let hint = detect_git_cwd();
        let prompt_text = match &hint {
            Some(p) => format!("  Path [{}]: ", p.display()),
            None    => "  Path (drag a folder here or type a path): ".to_string(),
        };

        let input = prompt(&prompt_text);
        let candidate = if input.trim().is_empty() {
            match &hint {
                Some(p) => p.clone(),
                None    => { println!("  ⚠  Please enter a path."); continue; }
            }
        } else {
            PathBuf::from(input.trim().trim_matches('"').trim_matches('\''))
        };

        if !candidate.exists() {
            println!("  ⚠  Path not found: {}", candidate.display());
            continue;
        }

        // Preview how many repos we find
        let repos = find_git_repos(&candidate);
        match repos.len() {
            0 => {
                println!("  ⚠  No git repositories found under: {}", candidate.display());
                continue;
            }
            1 => println!("  ✓  Found 1 git repository: {}", repos[0].display()),
            n => {
                println!("  ✓  Found {} git repositories:", n);
                for r in &repos { println!("       • {}", r.display()); }
            }
        }

        args.repo_path = Some(candidate);
        break;
    }

    let since_display = if args.since.is_empty() { "all history".to_string() } else { args.since.clone() };
    let input = prompt(&format!("  Analyze since [{}]: ", since_display));
    if !input.trim().is_empty() {
        let v = input.trim();
        args.since = if v.eq_ignore_ascii_case("all") || v.eq_ignore_ascii_case("all history") {
            String::new()
        } else {
            v.to_string()
        };
    }

    let input = prompt(&format!("  Output format [{}] (terminal/json/html): ", args.format));
    if !input.trim().is_empty() { args.format = input.trim().to_string(); }

    if args.format == "json" || args.format == "html" {
        let repos = find_git_repos(args.repo_path.as_ref().unwrap());
        let is_multi = repos.len() > 1;
        let default_out = if args.format == "html" {
            let base = dirs::desktop_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("hotspot-report.html");
            if is_multi {
                let dir = base.parent().unwrap_or(Path::new(".")).display().to_string();
                format!("{dir}/hotspot-report-<repo-name>.html  (one per repo)")
            } else {
                base.display().to_string()
            }
        } else {
            if is_multi { "hotspot-report-<repo-name>.json  (one per repo)".to_string() }
            else { "hotspot-report.json".to_string() }
        };
        let input = prompt(&format!("  Output base path [{}]: ", default_out));
        if !input.trim().is_empty() && !input.contains("<repo-name>") {
            args.output = Some(PathBuf::from(input.trim().trim_matches('"').trim_matches('\'')));
        }
    }

    let input = prompt(&format!("  Top N results to show in report (all files are always scanned) [{}]: ", args.top));
    if !input.trim().is_empty() { args.top = input.trim().parse().unwrap_or(args.top); }

    let input = prompt(&format!("  Bugs-only mode [{}] (yes/no): ", if args.bugs_only { "yes" } else { "no" }));
    if !input.trim().is_empty() {
        args.bugs_only = matches!(input.trim().to_lowercase().as_str(), "yes" | "y" | "true");
    }

    let input = prompt("  Restrict to subdirectory (e.g. src/app/) [none]: ");
    if !input.trim().is_empty() { args.path = Some(input.trim().to_string()); }

    let customize = prompt("  Customize scoring weights? [no] (yes/no): ");
    if matches!(customize.trim().to_lowercase().as_str(), "yes" | "y") {
        println!("  (press Enter to keep each default)");
        args.weight_churn          = prompt_f64(&format!("    churn          [{:.2}]: ", args.weight_churn),          args.weight_churn);
        args.weight_bugs           = prompt_f64(&format!("    bugs           [{:.2}]: ", args.weight_bugs),           args.weight_bugs);
        args.weight_reverts        = prompt_f64(&format!("    reverts        [{:.2}]: ", args.weight_reverts),        args.weight_reverts);
        args.weight_bursts         = prompt_f64(&format!("    bursts         [{:.2}]: ", args.weight_bursts),         args.weight_bursts);
        args.weight_coupling       = prompt_f64(&format!("    coupling       [{:.2}]: ", args.weight_coupling),       args.weight_coupling);
        args.weight_silo           = prompt_f64(&format!("    silo           [{:.2}]: ", args.weight_silo),           args.weight_silo);
        args.weight_commit_quality = prompt_f64(&format!("    commit-quality [{:.2}]: ", args.weight_commit_quality), args.weight_commit_quality);
    }

    println!();
    args
}

fn detect_git_cwd() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let ok = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&cwd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok { Some(cwd) } else { None }
}

fn prompt(msg: &str) -> String {
    print!("{msg}");
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
    buf.trim_end_matches('\n').trim_end_matches('\r').to_string()
}

fn prompt_f64(msg: &str, default: f64) -> f64 {
    let input = prompt(msg);
    if input.trim().is_empty() { default } else { input.trim().parse().unwrap_or(default) }
}

fn wait_for_enter() {
    print!("\nPress Enter to close...");
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Loads the workspace-root .env file (two levels above this crate's Cargo.toml)
    /// and returns a map of key → value.
    fn load_env() -> std::collections::HashMap<String, String> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().unwrap_or(&manifest_dir);
        let env_path = workspace_root.join(".env");
        let mut map = std::collections::HashMap::new();
        if let Ok(content) = std::fs::read_to_string(&env_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
                if let Some(eq) = trimmed.find('=') {
                    let key = trimmed[..eq].trim().to_string();
                    let val = trimmed[eq + 1..].trim().trim_matches('"').trim_matches('\'').to_string();
                    map.insert(key, val);
                }
            }
        }
        map
    }

    fn test_repo_path() -> Option<PathBuf> {
        let env = load_env();
        let path_str = env.get("TEST_REPO_PATH")
            .cloned()
            .or_else(|| std::env::var("TEST_REPO_PATH").ok())?;
        let path = PathBuf::from(path_str.trim());
        if path.exists() { Some(path) } else { None }
    }

    #[test]
    fn test_fmt_dur_milliseconds() {
        let d = Duration::from_millis(250);
        let s = fmt_dur(d);
        assert!(s.ends_with("ms"), "Sub-second durations should use 'ms': got '{s}'");
        assert!(s.contains("250"), "Should show the millisecond value: got '{s}'");
    }

    #[test]
    fn test_fmt_dur_seconds() {
        let d = Duration::from_millis(1_500);
        let s = fmt_dur(d);
        assert!(s.ends_with('s'), "Durations >= 1s should use 's': got '{s}'");
        assert!(s.contains("1.5"), "Should show decimal seconds: got '{s}'");
    }

    #[test]
    fn test_fmt_dur_exactly_one_second() {
        let d = Duration::from_millis(1_000);
        let s = fmt_dur(d);
        assert!(s.ends_with('s'), "Exactly 1s should use 's': got '{s}'");
    }

    #[test]
    fn test_find_git_repos_on_non_git_dir() {
        // A temp dir with no .git should return empty
        let tmp = std::env::temp_dir().join("git-scanline-test-no-repo");
        std::fs::create_dir_all(&tmp).ok();
        let repos = find_git_repos(&tmp);
        assert!(repos.is_empty(), "Non-git directory should return no repos");
    }

    #[test]
    fn test_make_output_path() {
        let base = PathBuf::from("report.html");
        let result = make_output_path(&base, "my-app");
        assert_eq!(result, PathBuf::from("report-my-app.html"));
    }

    #[test]
    fn test_make_output_path_special_chars() {
        let base = PathBuf::from("out/report.json");
        let result = make_output_path(&base, "my app/v2");
        // Special chars replaced with '-'
        assert!(result.to_str().unwrap().contains("my-app-v2"), "Special chars should be sanitized");
    }

    #[test]
    fn test_parse_log_real_repo() {
        let Some(repo) = test_repo_path() else {
            eprintln!("Skipping: TEST_REPO_PATH not set or path does not exist");
            return;
        };
        let (commits, _) = git::log_parser::parse_log(&repo, "", None)
            .expect("parse_log should succeed on a valid repo");
        assert!(!commits.is_empty(), "Real repo should have commits");
        assert!(!commits[0].hash.is_empty(), "Commit should have a hash");
        assert!(!commits[0].author.is_empty(), "Commit should have an author");
    }

    #[test]
    fn test_full_pipeline_scores_in_range() {
        let Some(repo) = test_repo_path() else {
            eprintln!("Skipping: TEST_REPO_PATH not set or path does not exist");
            return;
        };
        let (commits, _) = git::log_parser::parse_log(&repo, "", None)
            .expect("parse_log should succeed");
        assert!(!commits.is_empty(), "Repo must have commits");

        let all_files: HashSet<String> = commits.iter()
            .flat_map(|c| c.files.iter().cloned())
            .collect();
        let files = filters::filter_files(&all_files.into_iter().collect::<Vec<_>>(), None);
        let files: Vec<String> = files.into_iter().take(50).collect();
        if files.is_empty() { return; }

        let churn    = analyzers::churn::analyze_churn(&commits, &files);
        let bugs     = analyzers::bug_correlation::analyze_bug_correlation(&commits, &files);
        let reverts  = analyzers::revert_tracker::analyze_reverts(&commits, &files);
        let bursts   = analyzers::burst_detector::analyze_bursts(&commits, &files);
        let coupling = analyzers::coupling::analyze_coupling(&commits, &files);
        let silo     = analyzers::blame::analyze_authors(&commits, &files);
        let quality  = analyzers::commit_quality::analyze_commit_quality(&commits, &files);
        let diff_stats = Default::default();
        let weights  = Weights::default();

        let results = scoring::score_hotspots(
            &files, &churn, &bugs, &reverts, &bursts, &coupling, &silo, &quality,
            &diff_stats, &weights,
        );
        for r in &results {
            assert!(r.hotspot_score >= 0.0 && r.hotspot_score <= 100.0,
                "Score {} out of range for {}", r.hotspot_score, r.file);
        }
    }

    #[test]
    fn test_security_detects_env_files() {
        let commit = crate::types::Commit {
            hash: "abc".to_string(),
            author: "dev@example.com".to_string(),
            timestamp: 1700000000,
            subject: "add secrets".to_string(),
            files: vec![".env".to_string(), ".env.production".to_string(), "src/app.rs".to_string()],
        };
        let risks = analyzers::security::analyze_security(&[commit]);
        let flagged: Vec<&str> = risks.iter().map(|r| r.file.as_str()).collect();
        assert!(flagged.contains(&".env"), ".env must be flagged");
        assert!(flagged.contains(&".env.production"), ".env.production must be flagged");
        assert!(!flagged.contains(&"src/app.rs"), "src/app.rs must not be flagged");
    }
}
