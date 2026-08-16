# Sabicheiro Architecture

## Goals and boundaries

Sabicheiro is a local-first, single-host observability tool. The product model is
`process → connection → destination → location → traffic → behaviour → history`.
Platform-specific records are normalized immediately; neither the core nor the UI
contains Windows/Linux branches.

The 0.4 evidence pipeline makes observation and projection separate boundaries:

```text
provider -> sequencer/clock -> immutable evidence journal -> deterministic fusion
              |                         |                         |
              v                         v                         v
       capabilities/gaps          SQLite v5 / replay       connection projection
                                                                |
                              +---------------------------------+------------------+
                              v                                 v                  v
                       DNS/route/process                  rules/Today       bounded delta IPC
                       causal context                    notifications             |
                                                                                  v
                                                    Today / Live / Investigate /
                                                    Exposure / Data & Health
```

Every provider has a versioned descriptor, monotonic/UTC clock anchor, generation,
sequence, watermark and drop count. A gap invalidates only provenance fields that
depend on the lost capability. Fusion is idempotent and keeps ambiguous candidates;
it never chooses a process, DNS name, route or peer merely to fill an unknown field.

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

- `crates/sabicheiro-core`: evidence/provenance models, context, typed rules,
  notifications, boolean filters, aggregation, offline catalogs and demo fixtures.
- `crates/sabicheiro-capture`: capture providers, truth harness, sequencing/fusion,
  bounded queues, supervision, process/executable/route context, optional helper and
  recording boundaries.
- `crates/sabicheiro-geo`: bounded local MMDB/DNS caches and private/LAN handling.
- `crates/sabicheiro-storage`: SQLite v5 journal/projections, replay hashes, product
  state, tiered retention/FTS, recovery, `.sabi` data and CSV/JSON export.
- `apps/desktop/src-tauri`: narrow Tauri commands and runtime orchestration.
- `apps/desktop/src`: React presentation, Three.js scene and local UI state.

## Core event pipeline

Snapshot providers produce immutable `ObservedSocket` values and `SnapshotDiffer`
emits `Opened`, `Updated` or `Closed` normalized events. Canonical addresses retain
observed family and zone separately; every endpoint includes role, reason and source,
and every flow includes process instance, interface and capture generation. On Windows,
a bounded correlator joins IP Helper snapshots with ETW TCP/UDP/DNS observations using
PID, canonical tuple, interface, direction and a finite time window; every traffic counter
carries `CounterSource` provenance (`interface_aggregate`, `etw_flow`, `demo` or
`unavailable`). The aggregator keys flows by process, local/remote endpoint and
protocol, coalesces repeated events, retains closed flows for an eight-second visual
TTL and maintains cumulative counters independently of render cadence.

The capture loop and UI cadence are separate:

- OS socket poll: normally 750 ms.
- SQLite flush: batched, at most once per second.
- frontend snapshot: requested/emitted every 200–500 ms.

The opt-in reverse-DNS queue has an explicit capacity of 1,024 jobs; the Windows ETW
network queue is independently bounded at 8,192. Diagnostics expose depths,
capacities, capture/UI drops, event/DNS correlation counts, active counter source and
local process resources. PCAPNG recording is an explicit independent path.

## Tauri IPC

The frontend receives serializable view models, not database handles or filesystem
access. High-frequency updates are revisioned deltas with a monotonic sequence; a
client gap forces a complete resynchronization. Full statistics are computed before
the render payload is capped at 2,000 connections, 1,000 processes and 2,000
destinations. Commands are purpose-specific: capture/snapshot/self-test, history/timeline,
session comparison, metadata export, recording, focus rules, reconstruction, GeoIP
update/offline import, redacted support bundle and settings. Inputs use typed enums,
bounded values and validated/scoped paths. There is no arbitrary shell command or
general filesystem capability exposed to the webview. Product APIs cover rule
versions/dry-runs, Today state, notification state, network profiles, annotations,
listener snapshots, connection context, executable inspection jobs, handshake audit
and verified `.sabi` import/export.

The Tauri capability file grants only core window/event functionality required by
the webview. Privileged capture/recording remains behind the Rust boundary.

## Platform providers

`NetworkCaptureProvider` exposes `snapshot()` and `interfaces()`. Implementations:

- Windows: IP Helper TCP/UDP tables with owner PID plus optional bounded ETW network
  and DNS events. Native adapters retain permanent name, LUID and GUID.
- Linux: Netlink `INET_DIAG` TCP/UDP IPv4/IPv6 dumps plus socket-inode ownership from
  `/proc/<pid>/fd`; deprecated `/proc/net` socket parsing is not used.
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

Enrichment runs off the capture critical path through bounded worker queues. The feed
displays the IP immediately. GeoIP reads independently replaceable local DB-IP
Lite-compatible City/ASN MMDBs and caches by canonical address plus City/ASN generation.
Offline imports enforce a 512 MiB cap, validate MMDB edition, compute SHA-256 and use
an atomic local replacement. Private, loopback, multicast and unspecified addresses
are classified locally. Reverse DNS is disabled by default; when the user opts in,
public misses are queued to the configured system resolver with positive/negative
TTLs. Windows DNS Client answers can be correlated passively when ETW is available.
Unknown public IPs remain unknown. Historical re-enrichment builds a separate derived
view and replaces it atomically only if the job completes against the same dataset
generations; observed fields are immutable.

## Storage

SQLite uses WAL mode, foreign keys, sequential versioned migrations and indexes on
timestamps, session, process, remote IP, domain and country. The legacy v4 layer stores
connection/session data, one-second traffic samples with source provenance, bookmarks,
ignore rules, statistical baselines, investigations, endpoint/evidence provenance,
derived GeoIP views and recording metadata. Migrations from v1–v4 are transactional,
keep a readable backup and roll back on failure. Queries aggregate traffic
to 1 s/1 min/1 h buckets. Normal observation persists metadata only. Retention cleanup
runs at startup, after a retention change and hourly while the app is active. Paths
come from Tauri's platform app-data directory (`AppData` on Windows, XDG data on Linux).

Schema v5 adds immutable provider observations/cursors/gaps, projection versions and
hashes, rule/version/match records, annotations, grouped notifications, network
profiles, Today cards, exposure snapshots, handshake/network-action audits and
investigation workspace state. Read-only `.sabi` imports are isolated from this
database. Retention estimates and deletes in bounded tiers; salvage always writes a
new destination and never overwrites the original.

## Frontend and visualization

React owns selection, navigation, filters and accessibility. Primary navigation is
Today, Live, Investigate, Exposure and Data & Health. `GlobeScene` owns one
persistent renderer/scene/camera; traffic geometry is capped and rebuilt from the
current filtered snapshot. Both 3D and 2D views consume the same connection array.
Routes group by country, region/city or destination according to zoom/density, with
marker size communicating grouped count.

All critical globe facts also exist in the semantic feed/inspector. The SVG map is the
interactive non-WebGL fallback, with bounded zoom/pan, framing, antimeridian-safe routes
and an expanded layout that preserves selection and viewport. Remote, local and listener
workspaces filter the same evidence
array. Core navigation, failures, self-tests and dynamic baseline explanations route
through the English/Spanish catalog; original low-level diagnostics remain available
in an expandable detail.

## Security boundaries

1. The webview is unprivileged and has no general filesystem/shell capability.
2. The desktop Rust process validates capture settings, query grammar and export path.
3. The optional Windows helper accepts a fixed versioned, size/rate/time-bounded
   operation set over a named pipe with nonce and native client-PID binding. It accepts
   no shell, URL or arbitrary path. Native elevated deployment remains a review gate.
4. Packet payload is discarded unless the user explicitly starts a recording.
5. Logs contain metadata and errors, never payloads, credentials or DNS contents.
6. The support ZIP is assembled from an explicit allow-list and excludes raw IP
   history, payloads, recordings and secrets.
7. Optional egress requires a typed broker grant for either a fixed DB-IP download or
   the configured system resolver. Offline-enforced mode rejects both.
8. `.sabi` entries have exact path/size/count bounds plus CRC and manifest SHA-256 and
   are opened read-only; rule actions cannot block traffic, execute code or start PCAP.

## Failure model

Missing permissions, process attribution, DNS, GeoIP or packet counters degrade
independently. Capture errors are surfaced with retry/setup/demo actions. Database
failure keeps live observation in a bounded in-memory mode. GeoIP update failure keeps
the previous database. No fallback invents a domain, process, coordinate or risk score.
