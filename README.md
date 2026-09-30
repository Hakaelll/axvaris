# Sabicheiro

**Network observability for your computer.** Sabicheiro is a local-first desktop
application that turns the network activity of one computer into an interactive,
searchable world view. It connects processes to endpoints, destinations, local
history and explainable behaviour signals without accounts, telemetry or a paid API.

> Status: `0.4.2` lightweight Windows validation candidate. The strict test
> suite, production frontend, 100,000-row benchmark, SPDX SBOM, release binary and NSIS
> generation are validated locally. The current native launch/install lifecycle,
> Linux/eBPF packaging, elevated Windows ETW, the eight-hour soak and OS
> screen-reader/HiDPI matrix remain
> open, so this is not yet labelled a stable cross-platform release. Exact evidence is in
> [docs/VALIDATION.md](docs/VALIDATION.md).

## Visual evidence

The deterministic demo is the reproducible visual fixture (`pnpm dev`, then **Demo
mode**). The retained 0.3 review covers first-run/offline state, dense 3D and 2D data, map
zoom/pan/frame/expand/restore, inspector evidence, compact/comfortable density,
English/Castellano, keyboard map controls and the semantic tree. The 0.4 areas and
controls also pass browser checks at 720×600, 200% text scale, wide/narrow inspector
layouts, keyboard Escape/focus restoration and 50 Map↔Globe swaps. Their full native
WebView, screen-reader and OS HiDPI matrix remains recorded as `REVIEW`. The markers,
routes and map use only packaged geometry—there are no remote map tiles.

## Highlights

- Five task-oriented areas: Today, Live, Investigate, Exposure and Data & Health.
- Immutable SQLite v7 evidence journal with provider clocks, sequence gaps,
  field-level provenance, deterministic replay and tiered retention/FTS.
- Process-instance lineage, local executable revision/hash/signature evidence, DNS
  causal candidates, native route context and pseudonymous network profiles.
- A single bounded Today bootstrap covering the active-session strip, novelty across
  all retained history, previous-session comparison, listener changes and direct
  evidence drill-downs without loading the latest 500 connections.
- Explicit unlocated-destination reasons and filters: absent GeoIP, local/reserved,
  low precision, missing coordinates or pending enrichment; no invented coordinates.
- In-memory Presentation mode with stable pseudonyms for addresses, domains, paths,
  users, gateways and profiles. It changes only rendered text and never stored data,
  filters or exports.
- Responsive primary navigation that keeps capture state/actions available at narrow
  widths, plus a docked wide inspector and accessible narrow overlay.
- Application/network profiles, typed versioned rules, grouped quiet-hour
  notifications and listener exposure snapshots.
- Virtualized 100,000-row evidence table with a paginated semantic alternative and a
  local boolean query builder supporting grouped AND/OR/NOT expressions.
- Versioned, bounded, hash-verified `.sabi` investigation bundles opened read-only and
  never merged into local history.
- Explicit operation profiles, tray status/actions, per-user opt-in autostart,
  offline-enforced mode, enhanced contrast and text scaling.
- Spain-centered initial 3D/2D map framing and a hardened Exposure view that validates
  historical listener snapshots, ignores stale responses and isolates render failures.
- A self-contained `docs.html` guide covering Windows/Linux installation, features,
  capture boundaries, privacy, exports, troubleshooting and release verification.

- Synchronized local 3D globe, detailed 50m 2D map, grouped live feed and connection inspector.
- Adaptive country/region/destination route grouping to keep dense views readable.
- Native TCP/UDP socket observation and process attribution where the OS exposes it.
- Optional bounded Windows ETW network/DNS events with measured loss and IP Helper fallback.
- Separate remote, LAN/loopback and listener workspaces with independent counts.
- Real aggregate interface throughput plus explicitly sourced ETW per-flow deltas.
- Stable adapter name/LUID/GUID, hot-plug recovery, VPN awareness and safe fallbacks.
- Process, destination and historical views backed by versioned local SQLite storage.
- Time Machine reconstruction, 1 s/1 min/1 h traffic timelines and process sparklines.
- Session comparison by process/destination/country/port with CSV/JSON export.
- Search and negated filters for IP, CIDR, ASN, scope, GeoIP, source, process, domain,
  country, port and state, with localized syntax errors.
- Stable process/flow identities, endpoint-role evidence and IP Helper/ETW correlation
  quality without fabricated UDP peers, locations or traffic counters.
- Independent City/ASN MMDB generations, atomic import/update/rollback/removal and
  bounded asynchronous enrichment entirely off the capture path.
- Saved local investigations, DNS/IP/process evidence timeline, ASN profile and
  non-destructive historical re-enrichment.
- Versioned delta IPC with sequence-gap resynchronization and bounded render payloads.
- Ephemeral capture, granular local deletion, SQLite integrity/compact controls and
  per-subsystem diagnostics.
- Local bookmarks and view-only ignore rules; Sabicheiro never blocks traffic.
- Deterministic demo mode, command palette, keyboard navigation and reduced motion.
- Persistent English/Castellano interface selector on first run and in Settings.
- Verified offline City/ASN MMDB import, opt-in reverse DNS and explainable local baselines.
- Rate-limited local notification rules for new process/country, ASN or port.
- Capture self-test and user-reviewed redacted ZIP support bundle.
- CSV/JSON scoped export, retention cleanup and local resource/drop diagnostics.
- Explicit bounded PCAPNG recording: Pktmon on Windows, dumpcap on Linux.

## Platforms

| Platform | Observation backend | Package/validation state |
| --- | --- | --- |
| Windows 10/11 x86_64 | IP Helper owner-PID tables; optional ETW/helper events; Pktmon recording | 0.4.2 application/tests and local NSIS package validated; elevated ETW and launch/install cycle review open |
| Debian/Ubuntu x86_64 | Netlink `INET_DIAG` plus inode→PID; dumpcap recording | capture source cross-checks; eBPF and AppImage/deb native validation require Linux |
| Fedora/Arch | Same source boundary; distro prerequisites differ | source compatibility only, not a verified v1 artifact |
| ARM64 | Domain/provider boundaries are architecture-neutral | future target; no v1 artifact |

## Install

Windows users can run `Sabicheiro_0.4.2_x64-setup.exe`; it installs per-user,
adds a Start-menu entry and an uninstaller. For Linux, follow the detailed
[installation guide](linux.md) to download the source, install dependencies,
build and run a native `.deb` or AppImage.

For per-device DNS visibility through a router or DNS server, see the
[preparation guide](dns.md). Importing those external logs into Sabicheiro is
not implemented yet.

The standalone [docs.html](docs.html) guide provides the complete user-facing setup and
feature reference for both platforms, including current validation limits.

The interface language can be changed under **Settings -> Display -> Interface
language**. Settings also has a local English/Castellano search that focuses the
matching control. The choice is stored locally.

No account, subscription, license key or paid service is required. Sabicheiro works
without a GeoIP dataset and displays `Unknown location`; the optional **Settings →
GeoIP** action downloads the current free DB-IP Lite database for later offline use.

## Permissions and capture limits

The UI and normal socket observer run as the current user. Demo mode requires no extra
permission. The OS can hide sockets/processes owned by other users. Windows IP Helper
does not provide remote UDP destinations or per-flow traffic byte counters; ETW can add
both, but Windows may deny the trace to a normal user. In that case Sabicheiro keeps IP
Helper active and labels flow counters unavailable instead of inventing them.

Full packet recording is a separate explicit action. Pktmon may require elevation on
Windows; Linux dumpcap should be configured with the distro's capture group or minimal
capabilities. Sabicheiro never installs or redistributes Npcap. See
[NETWORK_CAPTURE.md](NETWORK_CAPTURE.md) for the full matrix.

## Privacy and cost

Normal operation is local by default. There is no analytics, crash upload, tracking,
login, cloud database, remote backend, AI model or commercial map. Reverse DNS is off
by default because enabling it can reveal queried IPs to the user's configured DNS
resolver. The only built-in download is the user-triggered, free whole-dataset GeoIP
update; individual observed IPs are never sent to DB-IP. No API key, subscription,
account or paid service is used by any feature.

Presentation mode is a visual privacy boundary only. Its stable pseudonyms live in
memory for the current app run; enabling or disabling it does not rewrite SQLite,
alter the active query or redact an export requested by the user.

All required application dependencies have compatible free/open-source terms, and all
primary features have runtime cost **€0**. See [PRIVACY.md](PRIVACY.md) and
[DEPENDENCIES.md](DEPENDENCIES.md).

## Development

Prerequisites and reproducible commands are in [BUILD.md](BUILD.md). The short path is:

```text
pnpm install --frozen-lockfile
pnpm --dir apps/desktop check
cargo test --workspace --all-targets --locked
pnpm tauri dev
```

`pnpm dev` provides the deterministic frontend preview. It is useful for layout and
accessibility work but does not replace native capture/packaging validation.

## License

Sabicheiro source code is licensed under the [MIT License](LICENSE). Dependency and
dataset attribution is documented in [DEPENDENCIES.md](DEPENDENCIES.md) and
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). System tools
such as Pktmon and dumpcap are not redistributed by this source tree.

## Troubleshooting

- **Capture unavailable:** choose **All interfaces**, check OS visibility/permissions,
  or switch to deterministic Demo mode. Adapter names, addresses and state are read
  through native APIs on Windows; stale selections are reset safely.
- **Where are LAN devices and listener sockets?** Use the **Remote**, **Devices**
  and **Listeners** tabs. Devices combines observed local connections with the OS
  neighbor cache. MAC and interface evidence groups addresses; unverified IPs stay
  separate. The cache is passive and cannot enumerate every device on the router.
- **Connection rows show no byte rate:** Windows IP Helper exposes endpoints and PIDs,
  not per-flow byte counters. The bottom strip shows real aggregate device traffic;
  ETW flow correlation appears only when Windows accepts the trace. Diagnostics shows
  the active source and preserves the original permission detail.
- **No city/ASN:** install DB-IP Lite in Settings. Unknown locations stay textual and
  are never plotted at invented coordinates.
- **No hostname:** enable reverse DNS only if its privacy trade-off is acceptable;
  otherwise the IP remains the correct fallback.
- **Unknown process:** the OS did not expose ownership before the socket disappeared;
  the connection is retained rather than discarded.
- **Blank/slow globe:** ensure WebGL is enabled in WebView2/WebKitGTK and use Map mode.
- **Recording fails:** choose a new `.pcapng` filename and configure Pktmon/dumpcap
  permissions as described in [NETWORK_CAPTURE.md](NETWORK_CAPTURE.md).
- **Linux bundle fails:** install the WebKitGTK/AppIndicator prerequisites from
  [BUILD.md](BUILD.md) and build on Linux rather than cross-compiling from Windows.
