# Axvaris

Axvaris is a local desktop application for investigating the network activity of a computer. It helps a person follow connections, inspect process attribution and reconstruct activity from stored evidence. It requires no account, subscription, telemetry service or paid API. Ordinary observation and history use local processing and storage. Optional reverse DNS can contact the configured resolver and is opt-in.

## Audience and purpose

The product is for people who need to understand which applications communicate, where their connections lead and what changed over time. It is an observability and investigation tool. It reports evidence and limits rather than malware verdicts or opaque risk scores.

## Investigation areas

- Today summarizes activity and changes, with actions that open the underlying evidence.
- Live pairs connections, local peers and listeners with a globe or map and a contextual inspector.
- Investigate provides paginated history, process instances, timelines and read-only .sabi investigation bundles.
- Exposure shows listening sockets, interfaces and explainable local rules.
- Data & Health provides capture capabilities, diagnostics, retention, privacy and imported external DNS evidence.

## Evidence and capture

Windows uses IP Helper TCP/UDP socket snapshots with owner PID and optional ETW network and DNS events. Elevated event capture needs permission. Polling can miss short-lived sockets.

Linux currently uses Netlink INET_DIAG TCP/UDP IPv4/IPv6 snapshots and socket-inode ownership through /proc/<pid>/fd. PID attribution depends on process and namespace visibility. An Aya event-capture implementation is not yet complete and is not a shipped capability.

Process attribution includes creation time when available. Unresolved identities, missing locations, provider timestamps and capture loss remain explicit. Local GeoIP databases provide approximate destination context; the application does not invent geographical coordinates.

Imported CSV or JSON resolver logs allow investigation of observed client addresses and queried domains. Imports retain their source and remain separate from local capture. A query does not establish a network connection or identify a process on the external device. This release does not install remote agents. Native capture and persistent DNS import require the desktop application; the browser preview uses deterministic fixtures.

Packet recording begins only after an explicit command through the supported local recording tools. Packet payloads are not part of the ordinary connection metadata and history pipeline.

## Identity and compatibility

Axvaris replaces the public Sabicheiro identity with a serious observability interface. Factory and Gotham are the agreed visual references. The monochrome serif AXVARIS wordmark and matching X, locally packaged Geist fonts and warm dark palette are durable identity choices. The X has no added crossing line. The implemented design system is documented in [DESIGN.md](DESIGN.md).

The technical application identifier remains io.sabicheiro.desktop. Existing data locations and .sabi compatibility remain preserved. English is the default language; Spanish and saved language preferences remain available.

## Quality boundaries

Keep capture independent of rendering, bound live data structures and paginate history. Rendering limits and passing builds do not establish native capture reliability, prolonged resource use or accessibility certification. The [validation record](docs/AXVARIS_VALIDATION.md) identifies completed checks and native acceptance work that still needs evidence.

The application supports keyboard navigation, visible focus, reduced motion and text scaling. Treat these as implemented behaviors rather than a certification claim. Keep provenance and uncertainty readable in every investigation path.
