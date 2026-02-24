# git-scanline — Documentation

`git-scanline` ships as a single Rust implementation.

## Documentation

- [Architecture](architecture.md) — pipeline, parallel analyzers, types, scoring

## Core concepts

The Rust implementation applies this pipeline:

1. Parse git commit history
2. Detect security-sensitive files in history
3. Filter out noise files (build artifacts, dependencies, generated files)
4. Run 7 independent analyzers across all commits
5. Score each file with a weighted formula → produce a ranked `HotspotResult` list
6. Render output (terminal table, JSON, or HTML)

### Scoring formula

```
hotspot_score = (churn × 0.27) + (bugs × 0.27) + (reverts × 0.14)
              + (bursts × 0.09) + (coupling × 0.09)
              + (silo × 0.05)  + (commit_quality × 0.09)
```

All weights are normalized at runtime so custom `--weight-*` flags always sum to 1.

### Risk tiers

| Tier | Score range |
|---|---|
| 🔴 CRITICAL | ≥ 75 |
| 🟠 HIGH | ≥ 50 |
| 🟡 MEDIUM | ≥ 25 |
| 🟢 LOW | < 25 |
