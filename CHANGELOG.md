# Changelog

All notable changes are documented here. Axvaris follows semantic versioning for
its local desktop releases.

## 0.4.3 — 2026-11-05 — Axvaris

### Added

- Axvaris identity, local Geist fonts, five investigation workspaces and external DNS imports with explicit provenance.
- Optional separately installed Aya helper for Linux TCP lifecycle events and UDP datagrams, with INET_DIAG fallback.
- Bounded Today bootstrap, session comparison, presentation pseudonyms and explicit unlocated-destination reasons.

### Fixed

- Saved investigation intervals, frozen autosaves, unloaded pin scopes and stale save responses.
- Complete paginated CSV/JSON exports beyond 100,000 records, per-page redaction and publication only after success.
- Simultaneous notification identities, snooze grouping and frontend quiet hours.
- History/export work sharing the capture mutex, focused-control keyboard shortcuts and effective text scaling.
- Today evidence labels, compact Live rows, folded investigation editing and Data & Health navigation order.
- Repository asset validation now recognizes generated ICO input images.

### Validation

- Local frontend/build/i18n, dependency/license audits, Rust format and Clippy passed; 164 Rust tests passed with one elevated ETW test ignored.
- Native Windows/Linux packaging and CI run manually for this tagged release. Package generation remains separate from installer, elevated capture, accessibility, visual and long-soak acceptance.

## 0.4.2 — 2026-08-16 — Lightweight runtime

### Added

- Reproducible artifact, workspace-space, frontend reachability, IPC parity and direct
  Rust dependency audits, plus a release-mode 100.000-row SQLite pagination harness.
- Dashboard IPC counters for requests, serialized bytes, empty deltas and overlaps in
  Data & Health.
- SQLite v6 traffic rollups with interval provenance, deterministic migration backups
  and stable `(last_seen_ms, id)` history cursors.

### Changed

- Today now opens directly without `FirstRun`; secondary views and dialogs load lazily,
  and the production Tauri adapter no longer contains the deterministic preview data.
- Dashboard polling is sequential and visibility/capture-aware, stable state returns a
  bounded empty delta, Globe rendering stops while hidden and the native engine waits
  for events/deadlines instead of waking every 200 ms while idle.
- Demo is limited to four documentation destinations and one event per second, remains
  entirely ephemeral, and never writes sessions, connections, samples or product state
  to the real SQLite database.
- Persistence upserts only changed connections, deduplicates stable samples, forces
  lifecycle flushes, and uses 10 s/30 s normal/background sampling while preserving
  genuine 1 s high-fidelity history.
- The public frontend excludes source maps, the 2,4 MB logo and preview adapter; Map
  uses shared 110 m geometry and Globe uses lighter controls/routes with full cleanup.
- Release builds use size optimization, fat LTO, one codegen unit, aborting panics and
  stripped symbols. Six additional unused IPC operations and one unused direct Rust
  dependency were removed after automated consumer audits.

### Validation status

- Strict TypeScript/i18n, production/reachability audits, format, Clippy, locked license
  and dependency audits, Linux capture source-check, 133 Rust tests, release build,
  NSIS bundle, SPDX SBOM and artifact hashes pass.
- Frontend is 1.163.691 B with no source maps; initial resources are 398.749 B. The
  portable is 6.528.512 B and NSIS installer 2.851.022 B, reductions of 68,29 % and
  67,19 % from 0.4.1.
- The 100.000-row first page is 716.705 B and 5,77 ms p95. The native GUI launch/soak,
  approved ETW, Linux packages and installer lifecycle remain explicitly unclaimed
  because their required execution environments were unavailable.

## 0.4.1 — 2026-08-14 — Spain Focus

### Added

- A standalone, responsive `docs.html` manual that follows the desktop design system
  and documents Windows/Linux installation, platform permissions, every primary area,
  privacy, exports, recording, known limitations, troubleshooting and release checks.
- Runtime validation and deterministic normalization for listener snapshots received
  by the Exposure view, including malformed-record reporting and IPv6 endpoint display.
- A view-level recovery boundary so an unexpected Exposure render failure cannot take
  down the rest of the desktop shell.

### Changed

- The 3D globe and 2D map now open and reset with Spain centered while retaining the
  existing approximate-home origin marker and route evidence.
- Exposure refreshes ignore stale asynchronous responses, remain safe under React
  Strict Mode and build listener comparisons in linear rather than quadratic time.

### Validation status

- Strict TypeScript, production/i18n, Rust formatting, Clippy, workspace tests,
  repository policy, license audit, SPDX SBOM, Windows NSIS build and archive checksum
  verification pass for the 0.4.1 candidate.
- Native Linux/eBPF/packages, approved elevated Windows ETW, the eight-hour soak and
  Narrator/NVDA/HiDPI evidence remain open and are not claimed as complete.

## 0.4.0 — 2026-08-14 — Process to Path

### Added

- Immutable evidence journal, provider capability/clock/cursor/gap contracts,
  deterministic multi-source fusion and replayable SQLite v5 projections.
- Truth-manifest capture harness, hardened optional Windows named-pipe helper,
  event-provider supervisor, lifecycle-first bounded queues and UDP/QUIC state models.
- Process lineage and executable revisions, cancellable local SHA-256/signature
  inspection, DNS causal graph, route evidence and pseudonymous network profiles.
- Today changes, historical application/network profiles, typed local rules, grouped
  notification inbox, exposure snapshots and hash-verified read-only `.sabi` bundles.
- Five-area information architecture, virtualized evidence table with semantic
  alternative, grouped boolean query builder and persistent investigation workspace.
- Tray controls, operation profiles, explicit autostart, quiet hours, offline-enforced
  mode, enhanced contrast and text scaling.
- Versioned local IANA service and Public Suffix release subsets, SPDX 2.3 SBOM
  generation and reproducible release performance evidence for 100,000 rows.

### Changed

- SQLite schema is now v5. Evidence remains immutable; derived projections,
  annotations, rule versions, notification state, profiles and audits are stored
  separately. Failed migrations preserve readable backups and the previous database.
- Query syntax now supports explicit AND/OR/NOT grouping with deterministic
  serialization while preserving implicit AND compatibility.
- Optional network activity is centralized behind typed fixed-destination grants;
  offline-enforced mode rejects every optional grant.
- The globe/map is a Live tool rather than the product home screen.

### Validation status

- Windows formatting, strict Clippy, 125 automated Rust tests, 839-key Spanish audit,
  production frontend build, 100,000-row benchmark, offline license audit, SBOM,
  release build and NSIS install/launch/uninstall pass.
- The available normal-user host denies ETW. Native Linux/eBPF/package checks,
  eight-hour soak and Narrator/NVDA/HiDPI evidence remain open, so this artifact is a
  validation candidate rather than a stable cross-platform publication.

## 0.3.0 — 2026-08-12 — Verified Destinations

### Added

- Canonical IPv4/IPv6 scope classification, endpoint roles, process-instance and flow
  generations, and explicit source/correlation evidence.
- Bounded Windows IP Helper/ETW correlator and native Linux `INET_DIAG` fixture tests.
- Independent City/ASN MMDB generations, synthetic offline fixtures, atomic lifecycle
  controls and non-destructive historical re-enrichment.
- Destination intelligence, local ASN profiles, evidence timelines, advanced filters,
  saved investigations and provenance-aware/redacted exports.
- Revisioned delta IPC with resynchronization, capture-generation recovery, bounded
  queues/payloads, ephemeral sessions, SQLite maintenance and subsystem diagnostics.
- Redesigned Three.js globe routes/markers and an interactive expandable 2D map with
  pan, zoom, framing, clustering and antimeridian-safe arcs.
- Compact/comfortable display density, expanded English/Castellano coverage and a
  guided local socket capability matrix.
- Offline repository, translation and locked-license audits plus a reproducible
  1k/10k/50k benchmark and 30-minute soak harness.

### Changed

- SQLite schema is now v4. Migrations from v1–v3 are transactional, create a readable
  pre-v4 backup and retain unknown legacy provenance without inventing it.
- High-frequency dashboard payloads are capped after complete statistics are computed.
- The supported product/build scope is Windows x86_64 and Linux x86_64 only.

### Privacy and cost

- No account, telemetry, analytics, cloud database, paid API or subscription was added.
- GeoIP remains an optional complete free-dataset download initiated by the user;
  observed IPs are never sent to a lookup API.
- Reverse DNS stays disabled by default and uses only the configured system resolver
  when explicitly enabled.

### Compatibility notes

- Windows IP Helper capture works as a normal user. ETW and Pktmon may require an
  approved elevated context and degrade explicitly when Windows denies access.
- Linux source/tests compile for `x86_64-unknown-linux-gnu`; AppImage/deb and native
  runtime proof must be produced on a Linux host.

## 0.2.0 — 2026-08-11

- Added optional bounded ETW/DNS event ingestion, remote/local/listener workspaces,
  timelines, offline MMDB import, notifications, session comparison and support ZIP.
- See the preserved 0.2 validation record in `docs/VALIDATION.md`.
