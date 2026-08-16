# Sabicheiro 0.4.0 — Process to Path

Validation candidate built on 2026-08-14. This package is suitable for local Windows
testing. It is not labelled a stable cross-platform release while the native Linux,
elevated ETW/eBPF, eight-hour soak and OS screen-reader gates remain open.

## Highlights

- Five task-oriented areas: **Today**, **Live**, **Investigate**, **Exposure** and
  **Data & Health**. The globe and map remain available inside Live.
- Immutable SQLite v5 evidence journal with provider clocks, cursors, gaps,
  provenance, deterministic replay and tiered retention/FTS search.
- Process-instance lineage, lazy cancellable executable SHA-256/signature inspection,
  DNS causal candidates, native route lookup and pseudonymous network profiles.
- Deterministic multi-source fusion, bounded lifecycle-first queues, provider
  supervision and explicit UDP/QUIC conversation semantics.
- Local Today changes, historical application/network profiles, typed versioned rules,
  grouped notifications with quiet hours and exposure snapshots.
- Virtualized 100,000-row evidence table with semantic paginated alternative, stable
  sorting/grouping and local presets.
- Boolean query grammar and editor with AND/OR/NOT groups, autocomplete, history,
  canonical serialization and positional errors.
- Versioned, hash-verified, bounded `.sabi` investigations imported read-only and never
  merged with local history.
- Windows tray, explicit operation profiles, opt-in per-user autostart, offline-enforced
  mode, enhanced contrast and text scaling.
- Embedded reviewed IANA service and Public Suffix release subsets. They are display
  hints only and never replace observed protocol/port evidence.

## Privacy and security

Default operation creates no Sabicheiro-initiated egress. Optional reverse DNS and the
complete DB-IP dataset update require typed broker grants, explicit local consent and
fixed destination classes; **Offline enforced** rejects both. There is no account,
telemetry, hosted map, reputation lookup or paid API.

Normal monitoring never stores packet payload. Handshake metadata is disabled by
default, bounded, visibly indicated, locally audited and uses zeroized ephemeral
buffers; no MITM or decryption exists. Local rules cannot run shell, block traffic or
start packet capture.

The optional Windows helper accepts a bounded versioned protocol over a per-user named
pipe with nonce and native client-PID binding. It exposes only a closed operation set.
The full webview remains unprivileged.

## Windows package evidence

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `Sabicheiro.exe` | 20,581,376 | `e41ce828df2057267ee5ad74482409c042e0ea9cd6adf59d3b06812e0aa78e85` |
| `Sabicheiro_0.4.0_x64-setup.exe` | 8,691,035 | `b76314b5eb1ae5633b726b12a17444a5286cfa1bb890ba699ffd1c5ec4a77a26` |
| `sbom-0.4.0.spdx.json` | 381,987 | `b2669959e393124e7b2b629fe0f287732bc19c1378a529fb84a3cda147b9ed21` |

The release binary launched normally. The NSIS package completed a per-user silent
install, launched the installed executable, removed its executable and Start-menu entry
through its own uninstaller, and retained local application data.

## Known validation boundaries

- Windows IP Helper remains useful without elevation and produced zero false remote
  destinations, but snapshots cannot meet the short-flow recall target: 33.3% for
  5–50 ms and 66.7% for flows of at least 50 ms in the 10 ms fixture.
- Windows ETW is implemented but this normal-user host returns `ACCESS_DENIED`; rerun
  the truth fixture in an administrator-approved trace context.
- Linux `INET_DIAG` source cross-checks successfully. Native Ubuntu/Debian eBPF,
  AppImage/deb, namespaces and permission variants require a real Linux host; the
  eBPF event provider is not claimed as complete in this candidate.
- The 100,000-row benchmark passes at 21.35 ms filter p95. A 30-second soak passes,
  but the mandatory eight-hour desktop soak has not run.
- Automated i18n/DOM/CSS checks pass. Browser visual automation was blocked by the
  browser's local-navigation policy after an initial connection error; Narrator/NVDA
  and the full 100/125/150% matrix remain required.

See `docs/VALIDATION.md` and `NEXT_TODOS.md` for exact commands, state and next actions.
