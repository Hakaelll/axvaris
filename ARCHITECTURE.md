# Sabicheiro Architecture

## Goals and boundaries

Sabicheiro is a local-first, single-host observability tool. The product model is
`process → connection → destination → location → traffic → behaviour → history`.
Platform-specific records are normalized immediately; neither the core nor the UI
contains Windows/Linux branches.

```text
OS socket/packet source
        │
        ▼
NetworkCaptureProvider ──► NormalizedNetworkEvent
        │                         │
        │                         ▼
        │                  ConnectionAggregator
        │                         │
        ├──────── attribution ────┤
        ├──────── DNS/GeoIP ──────┤
        │                         ▼
        │                  bounded event batch
        │                    ┌────┴────┐
        │                    ▼         ▼
        │                 SQLite   Tauri IPC
        │                              │
        └──────────────────────────────▼
                                React view model
                              globe/map/feed/inspector
```

## Workspace

- `crates/sabicheiro-core`: typed domain models, aggregation, filters, session
  reconstruction, baselines and deterministic demo generator.
- `crates/sabicheiro-capture`: capture-provider trait implementations, OS socket
  snapshots, interface discovery, process attribution and recording boundaries.
- `crates/sabicheiro-geo`: bounded local MMDB/DNS caches and private/LAN handling.
- `crates/sabicheiro-storage`: versioned SQLite migrations, retention, history and
  CSV/JSON export.
- `apps/desktop/src-tauri`: narrow Tauri commands and runtime orchestration.
- `apps/desktop/src`: React presentation, Three.js scene and local UI state.

## Core event pipeline

Capture providers produce immutable `ObservedSocket` values. `SnapshotDiffer`
compares bounded snapshots and emits `Opened`, `Updated` or `Closed` normalized
events. The aggregator keys flows by process, local/remote endpoint and protocol,
coalesces repeated events, retains closed flows for an eight-second visual TTL and
maintains cumulative counters independently of render cadence.

The capture loop and UI cadence are separate:

- OS socket poll: normally 750 ms.
- SQLite flush: batched, at most once per second.
- frontend snapshot: requested/emitted every 200–500 ms.

The asynchronous DNS queue has an explicit capacity of 1,024 jobs. Diagnostics expose
its depth/saturation, capture/UI drop counters, event rate and local process resources.
The current live socket providers do not merge packet counters into flows; PCAPNG
recording is an explicit independent path.

## Tauri IPC

The frontend receives serializable view models, not database handles or filesystem
access. Commands are purpose-specific: `get_snapshot`, `set_capture_state`,
`list_interfaces`, `query_history`, `export_view`, `start_recording`,
`stop_recording`, focus-rule commands, history reconstruction, GeoIP update,
`get_settings` and `update_settings`. Inputs use typed enums, bounded values and
validated/scoped paths. There is no arbitrary shell command or general filesystem
capability exposed to the webview.

The Tauri capability file grants only core window/event functionality required by
the webview. Privileged capture/recording remains behind the Rust boundary.

## Platform providers

`NetworkCaptureProvider` exposes `snapshot()` and `interfaces()`. Implementations:

- Windows: IP Helper TCP/UDP tables with owner PID, enriched from process APIs.
- Linux: `/proc/net/{tcp,tcp6,udp,udp6}` plus socket-inode ownership from
  `/proc/<pid>/fd`; a Netlink `sock_diag` provider can replace it without changing
  the domain model.
- Demo: seeded event generator used only when explicitly selected or in dev preview.
- Mock: deterministic provider for core/platform tests; never selected in production.

Pktmon/dumpcap record packets only after an explicit command. They do not replace the
socket provider or feed packet payloads into the ordinary metadata/history pipeline.

## Process attribution

PID is read alongside socket rows when supported. A long-lived process cache resolves
name and executable path and invalidates entries when the process start time changes,
preventing PID reuse from attributing a connection to the wrong executable. Icons are
loaded lazily by the desktop layer and are never required for an event to survive.

## DNS and GeoIP

Enrichment runs off the capture critical path. The feed displays the IP immediately.
GeoIP reads a local DB-IP Lite-compatible MMDB and uses a bounded cache. Private,
loopback, multicast and unspecified addresses are classified locally. Reverse DNS is
disabled by default; when the user opts in, public misses are queued to the configured
system resolver with positive/negative TTLs. Unknown public IPs remain unknown.

## Storage

SQLite uses WAL mode, foreign keys, sequential versioned migrations and indexes on
timestamps, session, process, remote IP, domain and country. It stores connection,
session, bookmark, ignore-rule, statistical-baseline and recording metadata. Normal
observation persists metadata only. Retention cleanup runs at startup, after a
retention change and hourly while the app is active. Paths come from Tauri's platform
app-data directory (`AppData` on Windows, XDG data on Linux).

## Frontend and visualization

React owns selection, navigation, filters and accessibility. `GlobeScene` owns one
persistent renderer/scene/camera; traffic geometry is capped and rebuilt from the
current filtered snapshot. Both 3D and 2D views consume the same connection array.
Routes group by country, region/city or destination according to zoom/density, with
marker size communicating grouped count.

All critical globe facts also exist in the semantic feed/inspector. The SVG map is the
non-WebGL fallback. Core navigation strings route through a small English/Spanish
message catalog; full localization remains a post-0.1 task.

## Security boundaries

1. The webview is unprivileged and has no general filesystem/shell capability.
2. The desktop Rust process validates capture settings, query grammar and export path.
3. Any future privileged helper must accept a fixed versioned protocol, operate on a
   pre-opened/validated destination and expose no command execution.
4. Packet payload is discarded unless the user explicitly starts a recording.
5. Logs contain metadata and errors, never payloads, credentials or DNS contents.

## Failure model

Missing permissions, process attribution, DNS, GeoIP or packet counters degrade
independently. Capture errors are surfaced with retry/setup/demo actions. Database
failure keeps live observation in a bounded in-memory mode. GeoIP update failure keeps
the previous database. No fallback invents a domain, process, coordinate or risk score.
