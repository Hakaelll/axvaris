# Sabicheiro 0.4.2 — Lightweight runtime

Sabicheiro 0.4.2 is a maintenance release focused on a smaller public artifact,
direct startup, bounded background work and lower local database growth. It does not
remove any supported product area and it does not enable capture or network access at
startup.

## User-visible changes

- The application opens directly on Today. The old `FirstRun` panel is gone; Capture
  off, Live capture, Demo data and the interface selector remain in the top bar.
- History loads 500 stable rows at a time and prefetches the adjacent page. Filtering
  invalidates stale requests; native export still covers the complete matching set.
- Data & Health now exposes dashboard request, serialized-byte, empty-delta and overlap
  counters. Stable operation produces small empty deltas and never overlaps polls.
- Demo remains useful for table, map, globe and Investigate, but is capped at four
  documentation-only destinations and one event per second. It is temporary and never
  enters the real history database.
- Normal traffic history samples every 10 seconds while visible and every 30 seconds in
  background, with bounded heartbeats and minute/hour rollups. Investigation High
  Fidelity continues to retain true one-second samples.

## Size and performance

All comparisons use the same Windows 10.0.26200 x86_64 host and the preserved 0.4.1
artifacts.

| Metric | 0.4.1 | 0.4.2 | Change |
| --- | ---: | ---: | ---: |
| Public frontend | 9.612.676 B | 1.163.691 B | −87,89 % |
| Source maps in product | 5.228.126 B | 0 B | −100 % |
| Initial resources | 2.939.648 B | 398.749 B | −86,44 % |
| Entry JavaScript | 456.606 B | 328.138 B | −28,14 % |
| Map code + shared world | 766.774 B | 118.215 B | −84,58 % |
| Globe code chunk | 678.128 B | 539.310 B | −20,47 % |
| Portable Windows executable | 20.589.568 B | 6.528.512 B | −68,29 % |
| Windows NSIS installer | 8.689.212 B | 2.851.022 B | −67,19 % |

The release-mode SQLite benchmark inserts 100.000 deterministic rows, retrieves the
first 500-row page in 5,77 ms p95, returns 716.705 B, traverses all 201 cursor pages
without a duplicate in 800,84 ms and exports all rows locally in 1.365,52 ms. Normal
eight-hour traffic data compacts from 2.880 raw rows to at most 24 raw/rollup rows while
preserving byte totals and rates; one-second high-fidelity rows remain raw.

The `opt-level=z` release is 819.200 B smaller than the otherwise identical `s` build.
Fat LTO, one codegen unit, `panic=abort` and stripped public symbols are retained. The
complete build/test/bundle tree occupied 8.086.743.018 B (7,53 GiB), below the 8 GiB clean-gate
limit, and the final explicit cleanup removes `target` entirely.

## Data compatibility and privacy

- SQLite schema v6 adds `sample_interval_ms`, `traffic_rollups` and cursor indexes.
  Every file-backed v1–v5 migration creates a readable pre-v6 backup; failed migration
  tests prove rollback.
- Stable persistence writes only new/changed/closed connections, skips identical
  samples until a bounded heartbeat and forces a flush on stop. Reopen tests preserve
  connection and session-end state.
- There is still no telemetry, analytics, remote map, cloud account or default egress.
  GeoIP update and reverse DNS remain behind explicit fixed-destination grants and
  `offline_enforced` still rejects optional network actions.

## Validation boundary

Formatting, strict Clippy, 133 Rust tests (plus one deliberately ignored elevated ETW
proof), strict TypeScript, 926-key Spanish coverage, CSS/import/IPC audits, locked
licenses, direct dependency consumers, Linux capture source-check, production Vite,
Windows release/NSIS packaging and SPDX generation pass.

Browser-based visual QA covered Today, Demo, table, 2D map, 3D globe, History,
Exposure, Data & Health and English/Spanish without console errors. It is not presented
as native capture or installer evidence. The environment rejected permission to launch
the local GUI after exhausting its execution allowance, so ten cold/warm native launch
runs, five-minute desktop scenarios, the install/uninstall cycle, approved ETW, native
Linux AppImage/deb and the eight-hour soak remain unclaimed. The exact launch harness
is versioned at `scripts/measure-windows-launch.mjs` for the next permitted Windows run.
