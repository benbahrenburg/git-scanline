# Rust Implementation — Architecture

The Rust binary lives in `rust/`. It is the primary implementation, optimized for speed with a single git subprocess and parallel analyzer execution via `rayon`.

## Module structure

```
rust/src/
├── main.rs          CLI entry, main() loop, run_analysis() pipeline
├── types.rs         All shared data types
├── animation.rs     ZORP surfing mascot (start_zorp / freeze / stop)
├── scoring.rs       Weighted score aggregation → HotspotResult
├── filters/
│   └── mod.rs       File noise filter (globs, path prefixes)
├── git/
│   ├── mod.rs       Re-exports log_parser
│   └── log_parser.rs  Single git log --numstat invocation → (Vec<Commit>, DiffStatsMap)
├── analyzers/
│   ├── mod.rs
│   ├── churn.rs           Commit frequency + recency weighting
│   ├── bug_correlation.rs Commits whose subject matches bug/fix/hotfix keywords
│   ├── revert_tracker.rs  Commits that revert previous commits
│   ├── burst_detector.rs  Rapid-commit windows (many commits in short time)
│   ├── coupling.rs        Files that always change together (co-change analysis)
│   ├── blame.rs           Author concentration (silo risk)
│   ├── commit_quality.rs  WIP commits, oversized commits
│   └── security.rs        Sensitive filenames in git history (.env, keys, certs)
└── reporters/
    ├── terminal.rs  comfy_table UTF8_FULL table + colored output
    ├── json.rs      Serialized Report → stdout or file
    └── html.rs      Self-contained HTML report
```

## Analysis pipeline

The 5-step pipeline runs inside `run_analysis()` for each discovered repo.

```mermaid
flowchart TD
    A([CLI args / interactive setup]) --> B[Discover git repos\nfind_git_repos]
    B --> C[ZORP animation\nstart_zorp freeze]
    C --> D

    subgraph pipeline ["run_analysis() — per repo"]
        D["[1/5] git log --numstat\nparse_log → Vec&lt;Commit&gt; + DiffStatsMap"]
        D --> E["[2/5] Security scan\nanalyze_security → Vec&lt;SecurityRisk&gt;"]
        E --> F["[3/5] Filter files\nfilter_files → Vec&lt;String&gt;"]
        F --> G

        subgraph parallel ["[4/5] rayon::join — parallel"]
            G1["churn::analyze_churn"]
            G2["bug_correlation::analyze_bug_correlation"]
            G3["revert_tracker::analyze_reverts"]
            G4["burst_detector::analyze_bursts"]
            G5["coupling::analyze_coupling"]
            G6["blame::analyze_authors"]
            G7["commit_quality::analyze_commit_quality"]
        end

        G --> H["[5/5] score_hotspots\n→ Vec&lt;HotspotResult&gt; sorted by score"]
        H --> I[Build Report struct]
    end

    I --> J{format}
    J -->|terminal| K[reporters::terminal\ncomfy_table UTF8_FULL]
    J -->|json| L[reporters::json\nserde_json]
    J -->|html| M[reporters::html\nself-contained HTML]
    K & L & M --> N[ZORP footer\nprint_zorp_footer]
    N --> O{interactive?}
    O -->|"yes → y"| P[Offer another repo\nloop back]
    O -->|no / n| Q([Exit])
```

## Parallel analyzer execution

All 7 analyzers read only immutable `&[Commit]` and `&[String]` references, so they satisfy `Send + Sync` without locks. `rayon::join` runs them in a binary tree to maximize CPU utilization.

```mermaid
sequenceDiagram
    participant M as main thread
    participant R as rayon threadpool

    M->>R: rayon::join (left half)
    M->>R: rayon::join (right half)

    Note over R: Left half
    R->>R: analyze_churn
    R->>R: rayon::join
    R->>R:   analyze_bug_correlation
    R->>R:   analyze_reverts

    Note over R: Right half
    R->>R: analyze_bursts
    R->>R: rayon::join
    R->>R:   analyze_coupling
    R->>R:   rayon::join
    R->>R:     analyze_authors (silo)
    R->>R:     analyze_commit_quality

    R-->>M: (churn, (bugs, reverts))
    R-->>M: (bursts, (coupling, (silo, quality)))
    M->>M: score_hotspots aggregates all 7 maps
```

## Data types

```mermaid
classDiagram
    class Commit {
        +String hash
        +String author
        +i64 timestamp
        +String subject
        +Vec~String~ files
    }

    class DiffStats {
        +usize additions
        +usize deletions
    }

    class ChurnData {
        +usize commit_count
        +f64 weighted_score
        +f64 raw_score
    }

    class BugData {
        +usize bug_commits
        +f64 bug_score
    }

    class RevertData {
        +usize revert_count
        +f64 revert_score
    }

    class BurstData {
        +usize burst_incidents
        +f64 burst_score
    }

    class SiloData {
        +String top_author
        +f64 top_author_percent
        +usize author_count
    }

    class CommitQualityData {
        +usize wip_commits
        +usize large_commit_count
        +f64 commit_quality_score
    }

    class CouplingEntry {
        +String file_a
        +String file_b
        +usize co_changes
        +f64 strength
    }

    class SecurityRisk {
        +String file
        +String risk_type
        +usize commit_count
        +String first_seen
        +String last_seen
    }

    class Weights {
        +f64 churn = 0.27
        +f64 bugs = 0.27
        +f64 reverts = 0.14
        +f64 bursts = 0.09
        +f64 coupling = 0.09
        +f64 silo = 0.05
        +f64 commit_quality = 0.09
    }

    class HotspotDetails {
        +usize commit_count
        +usize bug_commits
        +usize revert_count
        +usize burst_incidents
        +usize wip_commits
        +usize large_commit_count
        +String top_author
        +f64 top_author_percent
        +usize author_count
        +usize additions
        +usize deletions
    }

    class HotspotResult {
        +String file
        +f64 hotspot_score
        +f64 churn_score
        +f64 bug_fix_score
        +f64 revert_score
        +f64 burst_score
        +f64 coupling_score
        +f64 silo_score
        +f64 commit_quality_score
        +Tier tier
        +HotspotDetails details
    }

    class Tier {
        <<enum>>
        Critical
        High
        Medium
        Low
    }

    class Report {
        +ReportMeta meta
        +Vec~HotspotResult~ results
        +Vec~CouplingEntry~ couplings
        +Vec~SecurityRisk~ security_risks
    }

    class ReportMeta {
        +String since
        +usize commit_count
        +usize file_count
        +String analyzed_at
        +String repo_path
    }

    HotspotResult --> Tier
    HotspotResult --> HotspotDetails
    Report --> ReportMeta
    Report "1" --> "*" HotspotResult
    Report "1" --> "*" CouplingEntry
    Report "1" --> "*" SecurityRisk
```

## Git parser — combined invocation

`log_parser::parse_log()` runs a single `git log` subprocess and returns both the commit list and per-file diff stats in one pass:

```mermaid
flowchart LR
    A["git log\n--format=COMMIT|hash|email|timestamp|subject\n--date=unix\n--numstat\n--diff-filter=ACDMRT"] --> B[stdout stream]

    B --> C{line type}
    C -->|"COMMIT|..."| D[flush previous commit\nstart new Commit struct]
    C -->|"N\\tN\\tfilename"| E[push file to commit.files\naccumulate DiffStats]
    C -->|"-\\t-\\tfilename binary"| F[push file to commit.files\nskip DiffStats]
    C -->|blank| G[ignored]

    D & E & F & G --> H{more lines?}
    H -->|yes| C
    H -->|no| I["return (Vec&lt;Commit&gt;, DiffStatsMap)"]
```

## Scoring formula

```
hotspot_score =
    churn_score          × weight.churn          (default 0.27)
  + bug_fix_score        × weight.bugs           (default 0.27)
  + revert_score         × weight.reverts        (default 0.14)
  + burst_score          × weight.bursts         (default 0.09)
  + coupling_score       × weight.coupling       (default 0.09)
  + silo_score           × weight.silo           (default 0.05)
  + commit_quality_score × weight.commit_quality (default 0.09)
```

All individual scores are normalized to a 0–100 scale before weighting.
Weights are normalized at runtime so that custom `--weight-*` values always sum to 1.

**Tier thresholds** (in `scoring.rs`):

| Tier | Score |
|---|---|
| Critical | ≥ 75 |
| High | ≥ 50 |
| Medium | ≥ 25 |
| Low | < 25 |

## Analyzers reference

| Analyzer | File | Input signal | Output |
|---|---|---|---|
| Churn | `analyzers/churn.rs` | Commit frequency + recency decay | `ChurnData` per file |
| Bug correlation | `analyzers/bug_correlation.rs` | Subject keyword match (fix, bug, hotfix, …) | `BugData` per file |
| Revert tracker | `analyzers/revert_tracker.rs` | Subject starts with "Revert" | `RevertData` per file |
| Burst detector | `analyzers/burst_detector.rs` | Multiple commits in a sliding time window | `BurstData` per file |
| Co-change coupling | `analyzers/coupling.rs` | Files changed in same commit | `Vec<CouplingEntry>` |
| Silo (blame) | `analyzers/blame.rs` | Author distribution across commits | `SiloData` per file |
| Commit quality | `analyzers/commit_quality.rs` | WIP subjects, oversized commit file counts | `CommitQualityData` per file |
| Security | `analyzers/security.rs` | Filename pattern (.env, *.pem, …) | `Vec<SecurityRisk>` |

## Terminal reporter — table rendering

The terminal reporter uses `comfy_table` with the `UTF8_FULL` preset. Cells are constructed with `Cell::new("plain text").fg(Color::X)` rather than pre-colored ANSI strings — this ensures `comfy_table` measures visible character widths correctly.

```mermaid
flowchart LR
    A[HotspotResult] --> B[score_cell\nplain number + Color]
    A --> C[churn_cell\nblock bar ████ + Color::Red]
    A --> D[tier_cell\nplain label + Color]
    A --> E[Cell::new wip_count]
    B & C & D & E --> F[table.add_row\ncomfy_table measures\nplain text widths]
    F --> G[UTF8_FULL grid rendered\nANSI applied at print time]
```

## ZORP animation

`animation::start_zorp()` returns a `ZorpHandle`. The handle enforces a minimum 1 600 ms display time.

| Method | Behaviour |
|---|---|
| `.freeze()` | Leaves ZORP on screen; output appears below it |
| `.stop()` | Clears ZORP from screen (interactive mode welcome) |
| `print_zorp_footer()` | Prints a static ZORP frame as a footer after all output |
