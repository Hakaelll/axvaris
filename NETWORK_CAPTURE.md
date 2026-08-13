# Network Capture

## Decision

Sabicheiro separates always-available **socket observation** from optional
**packet observation/recording**. This avoids making the application depend on a
commercially restricted driver while still providing real process-aware connections.

| Capability | Windows | Linux |
| --- | --- | --- |
| TCP endpoints + PID | IP Helper `GetExtendedTcpTable` plus optional ETW events | Netlink `INET_DIAG` plus `/proc/<pid>/fd` owner mapping |
| UDP bound endpoints + PID | IP Helper `GetExtendedUdpTable` plus optional ETW events | Netlink `INET_DIAG` plus `/proc/<pid>/fd` owner mapping |
| IPv4/IPv6 | native tables | native tables |
| Per-flow bytes/packets | ETW event deltas when Windows permits the trace; otherwise explicitly unavailable | not supplied by `INET_DIAG` snapshots |
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
come directly from `GetAdaptersAddresses`; stable adapter name, LUID and Network GUID
are retained separately from the display name. No redirected PowerShell text or legacy
code page is involved. The selected adapter is refreshed every five seconds so rename,
dock, VPN and Wi-Fi changes do not leave an invisible stale filter.

Remote peers, local/LAN traffic and bound TCP/UDP listeners are retained in separate
workspaces and counts. Listeners never train remote-destination baselines. IP Helper
does not expose reliable per-flow byte or packet counters. The UI therefore labels its
rows as endpoint metadata and reports independently measured aggregate interface
throughput in the bottom strip instead of assigning device totals to flows.

### Event enhancement: ETW network and DNS Client providers

Sabicheiro attempts a bounded `Microsoft-Windows-Kernel-Network` and Windows DNS Client
user trace alongside IP Helper. A bounded temporal correlator joins event and snapshot
evidence using canonical endpoints, PID/process instance, direction and interface.
Parsed TCP/UDP send, receive, connect, accept,
disconnect and retransmit records add short-lived endpoints and PID/5-tuple byte and
packet deltas without retaining payloads. Copy events are discarded to avoid double
counting. DNS answers enter a bounded expiring correlation cache; they never trigger an
external lookup. Queue depth, capacity, parsed packet events and drops are visible in
Diagnostics.

ETW access is policy- and privilege-sensitive. On the 2026-08-12 normal-user validation
host Windows returned `ERROR_ACCESS_DENIED`. Sabicheiro recorded the actionable detail,
kept IP Helper active and marked flow counters unavailable; it did not elevate the
whole UI or invent per-flow rates. A controlled live ETW proof therefore remains `REVIEW` in
`docs/VALIDATION.md`.

### Packet recording: Pktmon

Windows 10 build 19041+ and Windows 11 include Packet Monitor (`pktmon.exe`). Sabicheiro
invokes the executable directly with a fixed argument vector—never through a shell—and
captures full packets to temporary ETL before `etl2pcap` conversion. Recording is an
explicit operation and commonly requires an elevated capture context. v0.3 does not
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
- ETW/TCPIP: implemented as an enhancement rather than a replacement because event
  schemas and trace permissions vary; IP Helper remains the reliable fallback.
- WinDivert/Npcap: technically capable but unsuitable as a mandatory dependency after
  redistribution/license review.

## Linux backend

The provider sends bounded `SOCK_DIAG_BY_FAMILY` Netlink dumps for TCP/UDP over IPv4 and
IPv6 and parses `inet_diag_msg` using layouts checked against the Linux UAPI. It no
longer reads the deprecated `/proc/net/{tcp,udp}*` socket tables. Socket inode ownership
is still mapped through `/proc/<pid>/fd`, so `hidepid` and other-user permissions can
produce `Unknown process` without dropping the endpoint. A fixed `ip -o address show`
command supplies interface/address metadata; selected local addresses filter the common
snapshot. The parser and Linux target cross-compilation pass on Windows. Native Linux
fixture tests create TCP/UDP IPv4 peers, verify inode→PID attribution and disappearance
after close, and exercise IPv6 when the local stack supports it; executing those tests
and collecting permission/performance evidence still requires a real Linux host.

PCAPNG recording uses a locally installed `dumpcap` process on interface `any`. The
normal setup is the distro's Wireshark capture group/capability configuration so the
Tauri/UI process stays unprivileged. Rotation passes `files:4` and a per-file size;
non-rotating capture uses a size autostop, and both modes accept a duration autostop.
Sabicheiro does not silently install or redistribute dumpcap/libpcap. A separately
packaged minimal helper remains outside the 0.3 local test release.

### Alternatives evaluated

- `/proc/net` parsing: removed because the kernel documents these socket tables as
  deprecated in favour of `tcp_diag`/`INET_DIAG`.
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

- IP Helper polling can miss connections shorter than the polling interval when ETW is
  denied. Diagnostics expose the active event source and retain the original permission
  detail.
- Baseline Windows UDP tables lack remote endpoints; bound UDP listeners are hidden by
  default instead of being presented as connections to `0.0.0.0:0` or `[::]:0`.
- Per-flow byte/packet counters are available only from accepted ETW events. Aggregate
  interface totals remain independently sourced and are never misrepresented as flow
  totals.
- Reverse DNS is opt-in and may use the configured resolver. TLS/SNI parsing is not
  implemented; encrypted DNS/TLS remains encrypted and no MITM is attempted.
- Windows recording elevation is not yet isolated in a dedicated minimal helper.
- Linux `INET_DIAG` and its native fixture suite compile for
  `x86_64-unknown-linux-gnu`, but provider/runtime and packaging proof have not been
  executed on Linux; see `docs/VALIDATION.md`.

## Primary references

- [Microsoft GetExtendedTcpTable](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedtcptable)
- [Microsoft GetExtendedUdpTable](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedudptable)
- [Microsoft GetAdaptersAddresses](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses)
- [Microsoft TCP/IP ETW events](https://learn.microsoft.com/en-us/windows/win32/etw/tcpip)
- [Microsoft UDP/IP ETW events](https://learn.microsoft.com/en-us/windows/win32/etw/udpip)
- [Microsoft Packet Monitor](https://learn.microsoft.com/en-us/windows-server/networking/technologies/pktmon/pktmon)
- [Microsoft `pktmon start`](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/pktmon-start)
- [Linux `inet_diag` UAPI](https://github.com/torvalds/linux/blob/master/include/uapi/linux/inet_diag.h)
- [Linux `sock_diag` UAPI](https://github.com/torvalds/linux/blob/master/include/uapi/linux/sock_diag.h)
