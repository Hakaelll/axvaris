# Changelog

All notable changes are documented here. Sabicheiro follows semantic versioning for
its local desktop releases.

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
