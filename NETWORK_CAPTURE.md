# Network Capture

## Decision

Sabicheiro separates always-available **socket observation** from optional
**packet observation/recording**. This avoids making the application depend on a
commercially restricted driver while still providing real process-aware connections.

| Capability | Windows | Linux |
| --- | --- | --- |
| TCP endpoints + PID | IP Helper `GetExtendedTcpTable` | `/proc/net/tcp*` + inode owner |
| UDP bound endpoints + PID | IP Helper `GetExtendedUdpTable` | `/proc/net/udp*` + inode owner |
| IPv4/IPv6 | native tables | native tables |
| Per-flow bytes/packets | not supplied by v0.1 socket backend | not supplied by v0.1 socket backend |
| PCAPNG recording | inbox Pktmon adapter | locally installed dumpcap |
| Normal UI privilege | current user | current user |

Microsoft documents that `GetExtendedTcpTable` returns TCP endpoints and supports
owner-PID tables for IPv4 and IPv6. `GetExtendedUdpTable` returns UDP endpoints with
owner PID but does not expose a connected remote endpoint. Consequently Windows UDP
destination visibility requires an optional packet/event source; Sabicheiro labels the
baseline limitation instead of inventing a remote destination.

## Windows backend

### Baseline: IP Helper

The provider polls owner-PID TCP and UDP tables, normalizes byte order and joins PIDs
to a guarded process cache. It requires no third-party driver and works as a normal
desktop user, subject to Windows access controls. TCP state and endpoint lifecycle are
diffed between polls. Interface friendly names, unicast addresses and operational state
come from the native `GetAdaptersAddresses` path exposed by `sysinfo`; no redirected
PowerShell text or legacy code page is involved. Choosing one filters snapshots by its
local IPs.

The normal feed keeps only sockets with a real remote peer. Bound UDP sockets and TCP
listeners can be enabled explicitly in Settings, but they never train remote-destination
baselines. IP Helper does not expose reliable per-flow byte or packet counters. The UI
therefore labels rows as endpoint metadata and reports independently measured aggregate
interface throughput in the bottom strip instead of assigning device totals to flows.

### Packet recording: Pktmon

Windows 10 build 19041+ and Windows 11 include Packet Monitor (`pktmon.exe`). Sabicheiro
invokes the executable directly with a fixed argument vector—never through a shell—and
captures full packets to temporary ETL before `etl2pcap` conversion. Recording is an
explicit operation and commonly requires an elevated capture context. v0.1 does not
install an elevation service; if the current process cannot start Pktmon, it reports a
permission error instead of elevating the whole UI silently.

Output extension, directory, existing-file collisions, size (1–4096 MiB) and duration
(up to 24 hours) are validated. Circular mode stops at its configured cap. Rotation
uses Pktmon multi-file mode, but Sabicheiro polls every 200 ms and stops conservatively
when four ETL files exist or the four-file byte ceiling is reached. Each ETL is
converted without overwriting to the requested `.pcapng`, then
`.part-002.pcapng`, etc.; an ETL is deleted only after its conversion succeeds.

Pktmon is preferred to bundling Npcap because Npcap's free edition has redistribution
and deployment restrictions. A user-installed compatible libpcap/Npcap runtime may be
detected as an optional backend, but it is never bundled or required for the core app.

### Alternatives evaluated

- WFP: powerful and process-aware, but a distributable callout driver adds signing,
  privileged-service and maintenance complexity inappropriate for the first release.
- ETW/TCPIP: useful future high-rate provider; event schemas/version permissions need
  more platform validation before it can replace the stable IP Helper baseline.
- WinDivert/Npcap: technically capable but unsuitable as a mandatory dependency after
  redistribution/license review.

## Linux backend

The compatibility provider parses `/proc/net/tcp`, `tcp6`, `udp` and `udp6`, then maps
socket inodes through `/proc/<pid>/fd`. It normally attributes the current user's
processes; hidepid and other-user permissions can produce `Unknown process`. Linux
kernel documentation notes `/proc/net/tcp*` is deprecated in favour of `tcp_diag`, so
the provider boundary is prepared for a Netlink `INET_DIAG` implementation. A fixed
`ip -o address show` command supplies interface/address metadata; the selected local
addresses filter the common socket snapshot.

PCAPNG recording uses a locally installed `dumpcap` process on interface `any`. The
normal setup is the distro's Wireshark capture group/capability configuration so the
Tauri/UI process stays unprivileged. Rotation passes `files:4` and a per-file size;
non-rotating capture uses a size autostop, and both modes accept a duration autostop.
Sabicheiro does not silently install or redistribute dumpcap/libpcap. A separately
packaged minimal helper is still desirable hardening beyond v0.1.

### Alternatives evaluated

- Netlink `sock_diag`: preferred successor for scalable socket enumeration; kept behind
  the same provider contract.
- eBPF: optional future backend, not a baseline because kernel/BTF/privilege and distro
  compatibility costs outweigh the v1 benefit.
- libpcap: mature and free, but a system dependency and not sufficient by itself for
  reliable process attribution.

## Process attribution and races

Socket-to-PID mappings are inherently snapshots. A connection can close between the
socket and process queries, and PID reuse can occur. Cache entries include executable
identity/start time when available. Missing attribution never drops a connection.

## VPN, LAN and loopback

Loopback is captured but hidden from the globe by default. RFC1918, link-local and ULA
destinations are `Local network` and never GeoIP-looked-up. VPN/virtual interfaces are
reported; when only a tunnel endpoint is observable, the UI states that the final
destination is hidden rather than presenting the VPN server as the destination.

## Known limitations

- Socket polling can miss connections shorter than the polling interval; diagnostics
  expose the backend identity, but v0.1 has no packet-to-flow merger to close the gap.
- Baseline Windows UDP tables lack remote endpoints; bound UDP listeners are hidden by
  default instead of being presented as connections to `0.0.0.0:0` or `[::]:0`.
- Real per-flow byte/packet counters require future packet/event correlation; interface
  totals are never misrepresented as exact flow totals.
- Reverse DNS is opt-in and may use the configured resolver. TLS/SNI parsing is not
  implemented; encrypted DNS/TLS remains encrypted and no MITM is attempted.
- Windows recording elevation is not yet isolated in a dedicated minimal helper.
- Linux provider/packaging code is present but has not been executed in this Windows
  validation environment; see `docs/VALIDATION.md`.

## Primary references

- [Microsoft GetExtendedTcpTable](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedtcptable)
- [Microsoft GetExtendedUdpTable](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedudptable)
- [Microsoft Packet Monitor](https://learn.microsoft.com/en-us/windows-server/networking/technologies/pktmon/pktmon)
- [Microsoft `pktmon start`](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/pktmon-start)
- [Linux `/proc/net/tcp` format](https://docs.kernel.org/networking/proc_net_tcp.html)
