# Node.js Implementation — Architecture

The Node.js implementation lives in `node/`. It produces the same output as the Rust binary and shares the same CLI flags. Analyzers run sequentially (no worker threads), and git history is fetched in two separate subprocess calls.

## Module structure

```
node/
├── bin/
│   └── git-scanline.ts   CLI entry, main(), runAnalysis(), runInteractive()
├── src/
│   ├── types.ts           All shared TypeScript interfaces
│   ├── git/
│   │   ├── log-parser.ts      git log --name-only → Commit[]
│   │   ├── diff-parser.ts     git log --numstat  → DiffStatsMap
│   │   └── blame-analyzer.ts  Author concentration per file
│   ├── analyzers/
│   │   ├── churn.ts           Commit frequency + recency weighting
│   │   ├── bug-correlation.ts Subject keyword match
│   │   ├── revert-tracker.ts  Revert commit detection
│   │   ├── burst-detector.ts  Rapid-commit window detection
│   │   ├── coupling.ts        Co-change file coupling
│   │   ├── commit-quality.ts  WIP + oversized commits
│   │   └── security-check.ts  Sensitive filename detection
│   ├── scoring/
│   │   └── hotspot-scorer.ts  Weighted aggregation → HotspotResult[]
│   ├── filters/
│   │   └── file-filter.ts     Noise file exclusion
│   └── reporters/
│       ├── terminal.ts        Colored table output
│       ├── json.ts            JSON serialization
│       ├── html.ts            Self-contained HTML report
│       └── zorp-animation.ts  ZORP surfing mascot
```

## Analysis pipeline

The Node.js pipeline runs 9 sequential steps inside `runAnalysis()`.

```mermaid
flowchart TD
    A([CLI args / interactive setup]) --> B[Discover git repos\nfindGitRepos]
    B --> C[ZORP animation\nstartZorpAnimation freeze]
    C --> D

    subgraph pipeline ["runAnalysis() — per repo"]
        D["[1/9] git log --name-only\nparseLog → Commit[]"]
        D --> E["[2/9] Security scan\nanalyzeSecurityRisks → SecurityRisk[]"]
        E --> F["[3/9] git log --numstat\nparseDiff → DiffStatsMap"]
        F --> G["[4/9] Filter files\nfilterFiles → string[]"]
        G --> H["[5/9] Churn analysis\nanalyzeChurn"]
        H --> I["[6/9] Bug correlation + reverts\nanalyzeBugCorrelation\nanalyzeReverts"]
        I --> J["[7/9] Bursts + coupling\nanalyzeBursts\nanalyzeCoupling"]
        J --> K["[8/9] Silo + commit quality\nanalyzeAuthors\nanalyzeCommitQuality"]
        K --> L["[9/9] Score hotspots\nscoreHotspots → HotspotResult[]"]
        L --> M[Build report object]
    end

    M --> N{format}
    N -->|terminal| O[reportTerminal\nchalk-colored table]
    N -->|json| P[reportJson]
    N -->|html| Q[reportHtml]
    O & P & Q --> R[printZorpFooter]
```

> **Note:** Steps 6, 7, and 8 each call two analyzers back-to-back. Unlike the Rust version, these run sequentially — there is no parallel execution.

## Data types

All types in `node/src/types.ts` are camelCase mirrors of the Rust snake_case types.

```mermaid
classDiagram
    class Commit {
        +string hash
        +string author
        +number timestamp
        +string subject
        +string[] files
    }

    class ChurnData {
        +number commitCount
        +number weightedScore
        +number rawScore
    }

    class BugData {
        +number bugCommits
        +number bugScore
    }

    class RevertData {
        +number revertCount
        +number revertScore
    }

    class BurstData {
        +number burstIncidents
        +number burstScore
    }

    class SiloData {
        +string topAuthor
        +number topAuthorPercent
        +number authorCount
    }

    class CommitQualityData {
        +number wipCommits
        +number largeCommitCount
        +number commitQualityScore
    }

    class CouplingEntry {
        +string fileA
        +string fileB
        +number coChanges
        +number strength
    }

    class SecurityRisk {
        +string file
        +string riskType
        +number commitCount
        +string firstSeen
        +string lastSeen
    }

    class Weights {
        +number churn = 0.27
        +number bugs = 0.27
        +number reverts = 0.14
        +number bursts = 0.09
        +number coupling = 0.09
        +number silo = 0.05
        +number commitQuality = 0.09
    }

    class ScoringInput {
        +Map churnData
        +Map bugData
        +Map revertData
        +Map burstData
        +CouplingEntry[] couplingData
        +Map siloData
        +Map commitQualityData
        +DiffStatsMap diffStats
        +Partial~Weights~ weights
    }

    class HotspotDetails {
        +number commitCount
        +number bugCommits
        +number revertCount
        +number burstIncidents
        +number wipCommits
        +number largeCommitCount
        +string topAuthor
        +number topAuthorPercent
        +number authorCount
        +number additions
        +number deletions
    }

    class HotspotResult {
        +string file
        +number hotspotScore
        +number churnScore
        +number bugFixScore
        +number revertScore
        +number burstScore
        +number couplingScore
        +number siloScore
        +number commitQualityScore
        +Tier tier
        +HotspotDetails details
    }

    class Report {
        +ReportMeta meta
        +HotspotResult[] results
        +CouplingEntry[] couplings
        +SecurityRisk[] securityRisks
    }

    class ReportMeta {
        +string since
        +number commitCount
        +number fileCount
        +string analyzedAt
    }

    HotspotResult --> HotspotDetails
    Report --> ReportMeta
    Report "1" --> "*" HotspotResult
    Report "1" --> "*" CouplingEntry
    Report "1" --> "*" SecurityRisk
    ScoringInput --> Weights
```

## Git parsing — two invocations

Unlike the Rust version, Node.js makes two separate `git log` calls:

```mermaid
flowchart LR
    A["git log\n--name-only\n--format=COMMIT|hash|email|ts|subject"]
    A --> B["parseLog()\n→ Commit[]"]

    C["git log\n--numstat\n--format=COMMIT|hash|email|ts|subject"]
    C --> D["parseDiff()\n→ DiffStatsMap\nMap&lt;file, {additions, deletions}&gt;"]

    B --> E[runAnalysis]
    D --> E
```

Both functions parse the same `COMMIT|hash|email|timestamp|subject` header format, switching between commits on that prefix and accumulating file data from the lines that follow.

## Analyzer outputs — data flow into scoring

```mermaid
flowchart LR
    commits(["Commit[]"])
    files(["string[] filteredFiles"])

    commits & files --> churn["analyzeChurn\n→ Map&lt;file, ChurnData&gt;"]
    commits & files --> bug["analyzeBugCorrelation\n→ Map&lt;file, BugData&gt;"]
    commits & files --> revert["analyzeReverts\n→ Map&lt;file, RevertData&gt;"]
    commits & files --> burst["analyzeBursts\n→ Map&lt;file, BurstData&gt;"]
    commits & files --> coupling["analyzeCoupling\n→ CouplingEntry[]"]
    commits & files --> silo["analyzeAuthors\n→ Map&lt;file, SiloData&gt;"]
    commits & files --> quality["analyzeCommitQuality\n→ Map&lt;file, CommitQualityData&gt;"]

    churn & bug & revert & burst & coupling & silo & quality --> scorer["scoreHotspots(ScoringInput)\n→ HotspotResult[]"]
```

## Scoring formula

```
hotspotScore =
    churnScore        × weights.churn         (default 0.27)
  + bugFixScore       × weights.bugs          (default 0.27)
  + revertScore       × weights.reverts       (default 0.14)
  + burstScore        × weights.bursts        (default 0.09)
  + couplingScore     × weights.coupling      (default 0.09)
  + siloScore         × weights.silo          (default 0.05)
  + commitQualityScore× weights.commitQuality (default 0.09)
```

**Tier thresholds** (same as Rust):

| Tier | Score |
|---|---|
| CRITICAL | ≥ 75 |
| HIGH | ≥ 50 |
| MEDIUM | ≥ 25 |
| LOW | < 25 |

## Differences from Rust implementation

| Aspect | Rust | Node.js |
|---|---|---|
| Git subprocesses | 1 (combined `--numstat`) | 2 (separate log + diff) |
| Analyzer execution | Parallel (`rayon::join`) | Sequential |
| Pipeline steps | 5 | 9 |
| Concurrency model | OS threads (rayon threadpool) | Single-threaded async |
| Type system | Structs + enums | TypeScript interfaces |
| Interactive loop | "Analyze another repo?" re-loop | One-shot, no re-loop |
| Animation handle | `ZorpHandle` with `freeze()`/`stop()` | `ZorpHandle` with `freeze()`/`stop()` |

## ZORP animation

`startZorpAnimation()` returns a `ZorpHandle` (same API as Rust):

| Method | Behaviour |
|---|---|
| `await handle.freeze()` | Leaves ZORP on screen; output appears below |
| `await handle.stop()` | Clears ZORP (used in interactive welcome screen) |
| `printZorpFooter()` | Prints a static ZORP frame as a report footer |
