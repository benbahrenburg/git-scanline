use crate::types::{CouplingEntry, HotspotResult, Report, ReportMeta, SecurityRisk, Tier};
use std::fs;
use std::path::Path;

pub fn report_html(report: &Report, output_file: &Path) -> Result<(), String> {
    let html = build_html(
        &report.meta,
        &report.results,
        &report.couplings,
        &report.security_risks,
    );
    fs::write(output_file, &html)
        .map_err(|e| format!("Failed to write {}: {e}", output_file.display()))?;
    eprintln!("✓ HTML report written to {}", output_file.display());
    Ok(())
}

fn build_html(
    meta: &ReportMeta,
    results: &[HotspotResult],
    couplings: &[CouplingEntry],
    security_risks: &[SecurityRisk],
) -> String {
    let top: Vec<&HotspotResult> = results.iter().take(25).collect();

    let chart_labels = serde_json::to_string(
        &top.iter()
            .map(|r| {
                let parts: Vec<&str> = r.file.split('/').collect();
                parts
                    .iter()
                    .rev()
                    .take(2)
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or_default();
    let chart_scores = serde_json::to_string(
        &top.iter()
            .map(|r| r.hotspot_score.round() as u64)
            .collect::<Vec<_>>(),
    )
    .unwrap_or_default();
    let chart_colors =
        serde_json::to_string(&top.iter().map(|r| tier_color(&r.tier)).collect::<Vec<_>>())
            .unwrap_or_default();
    let full_labels =
        serde_json::to_string(&top.iter().map(|r| r.file.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

    let crit_count: usize = results.iter().filter(|r| r.tier == Tier::Critical).count();
    let high_count: usize = results.iter().filter(|r| r.tier == Tier::High).count();
    let total_bug_commits: usize = results.iter().map(|r| r.details.bug_commits).sum();

    let security_stat = if !security_risks.is_empty() {
        format!(
            r#"<div class="stat" style="border-color:#ef4444"><div class="stat-label" style="color:#f87171">Security Risks</div><div class="stat-value red">{}</div></div>"#,
            security_risks.len()
        )
    } else {
        String::new()
    };

    let security_section = if !security_risks.is_empty() {
        let rows: String = security_risks.iter().map(|r| format!(
            "<tr><td class=\"path\">{}</td><td><span class=\"badge badge-critical\">{}</span></td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
            esc(&r.file), esc(&r.risk_type), r.commit_count, esc(&r.first_seen), esc(&r.last_seen)
        )).collect();
        format!(
            "<div class=\"card security-card\"><h2>🔐 Security Risks</h2>\
             <p class=\"security-note\">Sensitive files found in git history. Even deleted files remain accessible via <code>git log</code>.</p>\
             <table><thead><tr><th>File</th><th>Risk Type</th><th style=\"text-align:right\">Commits</th><th>First Seen</th><th>Last Seen</th></tr></thead>\
             <tbody>{rows}</tbody></table></div>"
        )
    } else {
        String::new()
    };

    let table_rows: String = results.iter().enumerate().map(|(i, r)| {
        let wip_cell = if r.details.wip_commits > 0 {
            format!("<span class=\"warn\">{}</span>", r.details.wip_commits)
        } else { "0".to_string() };
        format!(
            "<tr><td class=\"num\">{}</td><td class=\"path\">{}</td><td class=\"num\"><strong>{}</strong></td>\
             <td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td>\
             <td class=\"num\">{}</td><td class=\"num\"><span class=\"dim\">{}</span></td>\
             <td>{} <span class=\"dim\">({}%)</span></td><td>{}</td></tr>",
            i + 1, esc(&r.file), r.hotspot_score.round() as u64,
            r.details.commit_count, r.details.bug_commits, r.details.revert_count,
            wip_cell, r.details.large_commit_count,
            esc(&r.details.top_author), r.details.top_author_percent.round(),
            tier_badge(&r.tier)
        )
    }).collect();

    let coupling_section = if !couplings.is_empty() {
        let rows: String = couplings.iter().take(15).map(|c| format!(
            "<tr><td class=\"path\">{}</td><td class=\"path\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}%</td></tr>",
            esc(&c.file_a), esc(&c.file_b), c.co_changes, c.strength.round()
        )).collect();
        format!(
            "<div class=\"card\"><h2>⚠️ Co-change Coupling</h2>\
             <p style=\"color:#64748b;font-size:.8rem;margin-bottom:1rem\">Files that frequently change together — hidden dependencies that lead to bugs.</p>\
             <table><thead><tr><th>File A</th><th>File B</th><th style=\"text-align:right\">Co-changes</th><th style=\"text-align:right\">Coupling Strength</th></tr></thead>\
             <tbody>{rows}</tbody></table></div>"
        )
    } else {
        String::new()
    };

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>git-scanline report</title>
  <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.min.js"></script>
  <style>
    *,*::before,*::after{{box-sizing:border-box;margin:0;padding:0}}
    body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#0f172a;color:#e2e8f0;padding:2rem;font-size:14px;line-height:1.6}}
    h1{{font-size:1.75rem;font-weight:800}} h1 .fire{{color:#f97316}}
    h2{{font-size:1rem;font-weight:700;color:#94a3b8;text-transform:uppercase;letter-spacing:.06em;margin-bottom:1rem}}
    .meta{{color:#64748b;margin:.4rem 0 2rem;font-size:.8rem}} .meta span{{margin-right:1.5rem}}
    .stats{{display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:1rem;margin-bottom:1.5rem}}
    .stat{{background:#1e293b;border:1px solid #334155;border-radius:.625rem;padding:1rem 1.25rem}}
    .stat-label{{font-size:.7rem;text-transform:uppercase;letter-spacing:.06em;color:#64748b}}
    .stat-value{{font-size:2rem;font-weight:800;margin-top:.2rem}}
    .stat-value.red{{color:#f87171}} .stat-value.orange{{color:#fb923c}} .stat-value.blue{{color:#60a5fa}} .stat-value.slate{{color:#94a3b8}}
    .card{{background:#1e293b;border:1px solid #334155;border-radius:.75rem;padding:1.5rem;margin-bottom:1.5rem}}
    .security-card{{border-color:#ef4444}}
    .security-note{{color:#fca5a5;font-size:.8rem;margin-bottom:1rem}}
    .security-note code{{background:#1a0e0e;padding:.1rem .3rem;border-radius:.2rem;font-size:.75rem}}
    .chart-wrap{{position:relative;height:280px}}
    table{{width:100%;border-collapse:collapse}}
    th{{text-align:left;padding:.5rem .75rem;border-bottom:2px solid #334155;font-size:.7rem;font-weight:700;text-transform:uppercase;letter-spacing:.06em;color:#64748b}}
    td{{padding:.5rem .75rem;border-bottom:1px solid #1a2744;vertical-align:middle}}
    tr:last-child td{{border-bottom:none}} tr:hover td{{background:#162032}}
    td.path{{font-family:'JetBrains Mono','Fira Code',ui-monospace,monospace;font-size:.78rem;color:#7dd3fc;word-break:break-all}}
    td.num{{text-align:right;color:#94a3b8}}
    .dim{{color:#475569;font-size:.8em}} .warn{{color:#fbbf24;font-weight:600}}
    .badge{{display:inline-block;padding:.15rem .5rem;border-radius:999px;font-size:.7rem;font-weight:700;white-space:nowrap}}
    .badge-critical{{background:rgba(239,68,68,.15);color:#fca5a5}}
    .badge-high{{background:rgba(249,115,22,.15);color:#fdba74}}
    .badge-medium{{background:rgba(234,179,8,.15);color:#fde047}}
    .badge-low{{background:rgba(34,197,94,.15);color:#86efac}}
    .footer{{text-align:center;color:#334155;font-size:.75rem;margin-top:2rem}}
  </style>
</head>
<body>
  <h1><span class="fire">🔥</span> git-scanline</h1>
  <p class="meta">
    <span>Since: <strong>{since}</strong></span>
    <span>Repo: <strong>{repo}</strong></span>
    <span>Generated: <strong>{now}</strong></span>
    <span>Commits: <strong>{commits}</strong></span>
    <span>Files scanned: <strong>{file_count}</strong></span>
  </p>
  <div class="stats">
    <div class="stat"><div class="stat-label">Critical Hotspots</div><div class="stat-value red">{crit}</div></div>
    <div class="stat"><div class="stat-label">High Risk Files</div><div class="stat-value orange">{high}</div></div>
    <div class="stat"><div class="stat-label">Total Commits</div><div class="stat-value blue">{commits}</div></div>
    <div class="stat"><div class="stat-label">Bug-fix Commits</div><div class="stat-value slate">{bug_commits}</div></div>
    {security_stat}
  </div>
  {security_section}
  <div class="card"><h2>Top Hotspot Files — Score (0–100)</h2><div class="chart-wrap"><canvas id="chart"></canvas></div></div>
  <div class="card">
    <h2>Hotspot Details</h2>
    <table>
      <thead><tr><th>#</th><th>File</th><th style="text-align:right">Score</th><th style="text-align:right">Commits</th>
      <th style="text-align:right">Bug Commits</th><th style="text-align:right">Reverts</th>
      <th style="text-align:right">WIP</th><th style="text-align:right">Large</th>
      <th>Top Author</th><th>Risk</th></tr></thead>
      <tbody>{table_rows}</tbody>
    </table>
  </div>
  {coupling_section}
  <p class="footer">Generated by git-scanline on {now}</p>
  <script>
    const ctx = document.getElementById('chart');
    new Chart(ctx, {{
      type: 'bar',
      data: {{ labels: {chart_labels}, datasets: [{{ label: 'Hotspot Score', data: {chart_scores}, backgroundColor: {chart_colors}, borderRadius: 4, borderSkipped: false }}] }},
      options: {{
        responsive: true, maintainAspectRatio: false,
        plugins: {{ legend: {{ display: false }}, tooltip: {{ callbacks: {{ title: items => {full_labels}[items[0].dataIndex], label: items => ' Score: ' + items.raw }} }} }},
        scales: {{
          y: {{ beginAtZero: true, max: 100, ticks: {{ color: '#64748b' }}, grid: {{ color: '#1e2d47' }} }},
          x: {{ ticks: {{ color: '#64748b', maxRotation: 40, font: {{ size: 11 }} }}, grid: {{ display: false }} }}
        }}
      }}
    }});
  </script>
</body>
</html>"#,
        since = esc(&meta.since),
        repo = esc(&meta.repo_path),
        now = now,
        commits = meta.commit_count,
        file_count = meta.file_count,
        crit = crit_count,
        high = high_count,
        bug_commits = total_bug_commits,
        security_stat = security_stat,
        security_section = security_section,
        table_rows = table_rows,
        coupling_section = coupling_section,
        chart_labels = chart_labels,
        chart_scores = chart_scores,
        chart_colors = chart_colors,
        full_labels = full_labels,
    )
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn tier_color(tier: &Tier) -> &'static str {
    match tier {
        Tier::Critical => "rgba(239,68,68,0.75)",
        Tier::High => "rgba(249,115,22,0.75)",
        Tier::Medium => "rgba(234,179,8,0.75)",
        Tier::Low => "rgba(34,197,94,0.75)",
    }
}

fn tier_badge(tier: &Tier) -> &'static str {
    match tier {
        Tier::Critical => "<span class=\"badge badge-critical\">🔴 CRITICAL</span>",
        Tier::High => "<span class=\"badge badge-high\">🟠 HIGH</span>",
        Tier::Medium => "<span class=\"badge badge-medium\">🟡 MEDIUM</span>",
        Tier::Low => "<span class=\"badge badge-low\">🟢 LOW</span>",
    }
}
