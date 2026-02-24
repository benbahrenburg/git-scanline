use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
use crate::types::{Report, HotspotResult, Tier};

pub fn report_terminal(report: &Report) {
    eprintln!();
    println!(
        "{} — since \"{}\" ({} commits, {} files)",
        "🔥 git-scanline".red().bold(),
        report.meta.since.bright_black(),
        report.meta.commit_count.to_string().bright_black(),
        report.meta.file_count.to_string().bright_black(),
    );
    println!();

    // ── Security warnings ──────────────────────────────────────────────────
    if !report.security_risks.is_empty() {
        println!("{}", "🔐 Security Risks — sensitive files found in git history:".red().bold());
        println!("{}", "   Even deleted files remain accessible via git history!".red());
        println!();
        for risk in &report.security_risks {
            println!(
                "   {}  {} [{}] {} (first: {}, last: {})",
                "⚠".red(),
                risk.file.cyan(),
                risk.risk_type.red(),
                format!("{} commit{}", risk.commit_count, if risk.commit_count != 1 { "s" } else { "" }).bright_black(),
                risk.first_seen.bright_black(),
                risk.last_seen.bright_black(),
            );
        }
        println!();
    }

    if report.results.is_empty() {
        println!("{}", "  No hotspots found with current filters.".yellow());
        println!();
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["RANK", "FILE", "SCORE", "CHURN", "BUGS", "REVERTS", "WIP", "RISK"]);

    for (i, r) in report.results.iter().enumerate() {
        let score    = r.hotspot_score.round() as u64;
        let tier_str = tier_label(&r.tier);
        let score_str = color_score(score, &format!("{score:3}"));
        let wip_str = if r.details.wip_commits > 0 {
            r.details.wip_commits.to_string().yellow().to_string()
        } else {
            "0".bright_black().to_string()
        };

        table.add_row(vec![
            format!("{:3}", i + 1),
            truncate_path(&r.file, 44),
            score_str,
            make_bar(r.churn_score),
            r.details.bug_commits.to_string(),
            r.details.revert_count.to_string(),
            wip_str,
            tier_str,
        ]);
    }

    println!("{table}");

    // ── Co-change coupling ─────────────────────────────────────────────────
    let notable: Vec<_> = report.couplings.iter().filter(|c| c.co_changes >= 5).take(5).collect();
    if !notable.is_empty() {
        println!();
        println!("{}", "⚠️  Co-change coupling detected:".yellow());
        for c in &notable {
            println!(
                "    {} ↔ {} {}",
                c.file_a.cyan(),
                c.file_b.cyan(),
                format!("(changed together {}x, strength {}%)", c.co_changes, c.strength.round()).bright_black(),
            );
        }
    }

    // ── Recommendations ────────────────────────────────────────────────────
    let recs = build_recommendations(&report.results);
    if !recs.is_empty() {
        println!();
        println!("{}", "💡 Recommendations:".cyan());
        for rec in &recs {
            println!("    {} {}", "•".white(), rec);
        }
    }

    println!();
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_bar(score: f64) -> String {
    let s = score.round() as usize;
    let parts = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
    let filled  = s / 20;
    let rem     = s % 20;
    let partial = parts[(rem * 8 / 20).min(8)];
    let bar = "█".repeat(filled) + partial;
    format!("{bar:<5}").red().to_string()
}

fn color_score(score: u64, text: &str) -> String {
    match score {
        75..=100 => text.red().bold().to_string(),
        50..=74  => text.yellow().bold().to_string(),
        25..=49  => text.white().to_string(),
        _        => text.green().to_string(),
    }
}

fn tier_label(tier: &Tier) -> String {
    match tier {
        Tier::Critical => "🔴 CRITICAL".red().to_string(),
        Tier::High     => "🟠 HIGH".yellow().to_string(),
        Tier::Medium   => "🟡 MEDIUM".white().to_string(),
        Tier::Low      => "🟢 LOW".green().to_string(),
    }
}

fn truncate_path(s: &str, max: usize) -> String {
    if s.len() <= max { return s.to_string(); }
    format!("…{}", &s[s.len().saturating_sub(max - 1)..])
}

fn build_recommendations(results: &[HotspotResult]) -> Vec<String> {
    let mut recs = Vec::new();
    for r in results.iter().take(10) {
        let name = r.file.split('/').next_back().unwrap_or(&r.file);
        if r.details.top_author_percent >= 80.0 && r.details.author_count <= 2 {
            recs.push(format!(
                "{} has {}% single-author commits — consider a knowledge-transfer session",
                name.yellow(), r.details.top_author_percent.round()
            ));
        }
        if r.details.burst_incidents >= 3 {
            recs.push(format!(
                "{} shows burst patterns: {} rapid-commit windows detected",
                name.yellow(), r.details.burst_incidents
            ));
        }
        if r.details.revert_count >= 2 {
            recs.push(format!(
                "{} has been reverted {} times — consider adding tests or stricter review",
                name.yellow(), r.details.revert_count
            ));
        }
        if r.details.wip_commits >= 3 {
            recs.push(format!(
                "{} appears in {} WIP/low-quality commits — this area needs careful review",
                name.yellow(), r.details.wip_commits
            ));
        }
        if r.details.large_commit_count >= 3 {
            recs.push(format!(
                "{} was swept up in {} large commits — consider smaller, focused PRs",
                name.yellow(), r.details.large_commit_count
            ));
        }
        if recs.len() >= 8 { break; }
    }
    recs
}
