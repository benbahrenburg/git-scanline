# git-scanline — Documentation

`git-scanline` ships in two independent implementations that produce identical output.

| | Rust | Node.js |
|---|---|---|
| Entry point | `rust/src/main.rs` | `node/bin/git-scanline.ts` |
| Analyzer execution | Parallel (`rayon`) | Sequential |
| Git invocations | 1 (`--numstat`) | 2 (separate log + diff) |
| Pipeline steps | 5 | 9 |
| Output formats | terminal, json, html | terminal, json, html |

## Documentation

- [Rust implementation](rust/architecture.md) — pipeline, parallel analyzers, types, scoring
- [Node.js implementation](node/architecture.md) — pipeline, module structure, types

## Shared concepts

Both implementations apply the same conceptual pipeline:

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
