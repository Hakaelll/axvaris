# Sabicheiro

**Network observability for your computer.** Sabicheiro is a local-first desktop
application that turns the network activity of one computer into an interactive,
searchable world view. It connects processes to endpoints, destinations, local
history and explainable behaviour signals without accounts, telemetry or a paid API.

> Status: functional `0.2.0` release candidate. The Windows x86_64 application, tests,
> production frontend and target-size UI checks pass locally. Linux `INET_DIAG` source
> cross-compiles, but AppImage/deb launch validation still requires a Linux machine.
> Windows ETW and packet recording remain privilege-sensitive and fall back safely.
> Exact evidence and remaining platform checks are in
> [docs/VALIDATION.md](docs/VALIDATION.md).

## Visual evidence

The deterministic demo is the reproducible visual fixture (`pnpm dev`, then **Demo
mode**). It was inspected at 1280×720 and 1920×1080 in English and Castellano, including
Globe, Settings, History, semantic roles and keyboard shortcuts. The review caught and
fixed retained scroll position between views; the console remained error-free. This is
recorded in [docs/VALIDATION.md](docs/VALIDATION.md).

## Highlights

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
- Search and filters such as `process:firefox country:DE port:443 state:active`.
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
| Windows 10/11 x86_64 | IP Helper owner-PID tables; optional ETW events; Pktmon recording | application/tests validated; NSIS release built locally |
| Debian/Ubuntu x86_64 | Netlink `INET_DIAG` plus inode→PID; dumpcap recording | source cross-check passes; AppImage/deb runtime validation pending |
| Fedora/Arch | Same source boundary; distro prerequisites differ | source compatibility only, not a verified v1 artifact |
| ARM64 | Domain/provider boundaries are architecture-neutral | future target; no v1 artifact |

## Install

Windows users can run `Sabicheiro-0.2.0-windows-x64-setup.exe`; it installs per-user,
adds a Start-menu entry and an uninstaller. Linux packages can be produced natively
with the commands in [BUILD.md](BUILD.md); when built, mark the AppImage executable or
install the `.deb` using the normal distro tooling.

The interface language can be selected on the welcome screen and changed later under
**Settings -> Display -> Interface language**. The choice is stored locally.

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
update; individual observed IPs are never sent to DB-IP.

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
- **Where are LAN/listener sockets?** Use the **Remote**, **Local** and **Listeners**
  tabs. Their counts and semantics stay separate and listeners never train remote
  destination baselines.
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
