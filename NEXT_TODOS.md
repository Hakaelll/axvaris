# Sabicheiro Operational Plan

Last updated: 2026-08-11. `DONE` means the stated acceptance criterion was compiled
and tested in the available environment. `REVIEW` means the implementation exists but
needs visual, privilege-sensitive or Linux-native evidence; it is never used as a
synonym for done.

| ID | Title | Description | Priority | Status | Dependencies | Acceptance criteria | Related files | Technical notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| P0-01 | Repository inspection | Inspect files, AGENTS, Git, OS and toolchains | P0 | DONE | — | Baseline and available toolchains recorded | `BUILD.md` | Windows 11; Rust/MSVC/WebView2; no WSL/container runtime |
| P1-01 | License/cost research | Verify libraries/datasets and reject paid services | P0 | DONE | P0-01 | Purpose/license/cost/redistribution plus locked audit | `DEPENDENCIES.md` | 515 Cargo + 43 pnpm packages; no undeclared/commercial-only license |
| P2-01 | Architecture | Define platform/core/UI/security boundaries | P0 | DONE | P1-01 | Required boundaries documented against implementation | `ARCHITECTURE.md`, `NETWORK_CAPTURE.md` | Known v0.1 gaps stated explicitly |
| P3-01 | Domain model | Typed connection/process/destination/session/event models | P0 | DONE | P2-01 | Serialization, identity and reconstruction tests pass | `crates/sabicheiro-core` | Shared semantics mirrored strictly in TypeScript |
| P3-02 | Demo pipeline | Seeded realistic event generator | P0 | DONE | P3-01 | Same seed/time produces identical enriched events | `crates/sabicheiro-core/src/demo.rs` | Explicit Demo mode only |
| P4-01 | Design system | Tokens, desktop layout, accessibility and motion rules | P0 | DONE | P2-01 | Production build and target-size inspection pass | `DESIGN.md`, `apps/desktop/src/styles.css` | Verified at 1280×720 and 1920×1080; view-change scroll regression fixed |
| P5-01 | Desktop shell | Tauri commands, runtime and least-privilege capability | P0 | DONE | P3-01 | Typed commands compile; native Windows executable smoke-tested | `apps/desktop/src-tauri` | `core:default` only; no frontend shell/filesystem API |
| P6-01 | Globe/map | Persistent Three.js globe and shared SVG map data | P0 | DONE | P3-02,P4-01 | Local geometry, selection and adaptive grouping compile and render at target sizes | `apps/desktop/src/components/visualization`, `src/lib/routes.ts` | Unknown geo never plotted; routes capped |
| P7-01 | Feed/inspector | Grouped feed synchronized with selection | P0 | DONE | P6-01 | Semantic sections, selection and keyboard shortcuts pass interactive QA | `apps/desktop/src/components` | Configurable conceptual grouping, off-screen layout containment |
| P8-01 | Windows provider | IPv4/v6 TCP/UDP owner-PID socket snapshots | P0 | DONE | P3-01 | Native table, established-peer and Unicode interface tests pass on Windows | `crates/sabicheiro-capture/src/windows.rs`, `windows_events.rs` | IP Helper always available; ETW enhancement falls back cleanly when Windows denies trace access |
| P9-01 | Linux provider | Netlink socket diagnostics and inode→PID mapping | P0 | REVIEW | P3-01 | UAPI parser tests and Linux-target cross-check pass; real Linux runtime still required | `crates/sabicheiro-capture/src/linux.rs`, `inet_diag_wire.rs` | Uses `NETLINK_INET_DIAG`; `/proc/<pid>/fd` remains only for owner attribution |
| P10-01 | Process attribution | PID/name/path/start-time cache | P0 | DONE | P8-01,P9-01 | Current Windows test process is attributed; missing PID stays unknown | `crates/sabicheiro-capture/src/process.rs` | Cache refresh overwrites PID reuse identity |
| P11-01 | GeoIP/DNS | Local MMDB plus bounded async DNS caches | P1 | REVIEW | P3-01 | Scope/cache/fallback tests pass; live free-dataset download not exercised | `crates/sabicheiro-geo` | Reverse DNS opt-in/off by default; DB-IP attribution shown |
| P12-01 | Search/filter | Global search and compact query grammar | P1 | DONE | P3-01 | Valid, combined and invalid parser tests pass | `crates/sabicheiro-core/src/filter.rs`, UI filter | Bounded/typed backend parser |
| P13-01 | SQLite/history | Migrations, sessions, retention and reconstruction | P1 | DONE | P3-01 | v1→v3 migration, timeline provenance, overlap, retention, baseline and recording tests pass | `crates/sabicheiro-storage` | WAL, indexed; startup/change/hourly cleanup |
| P14-01 | Secondary views | Processes, destinations, history and top context | P1 | DONE | P7-01,P13-01 | Shared filters/selection and target-size visual QA pass | `apps/desktop/src/views`, `TopContext.tsx` | Avoids independent duplicate data stores |
| P15-01 | Metadata export | CSV/JSON query/range/session scope | P1 | DONE | P13-01 | Escaping and output-path tests pass | storage/Tauri/dialogs | Relative paths scoped to app data; no overwrite by packet recorder |
| P16-01 | PCAPNG | Explicit bounded recording adapters and session index | P1 | REVIEW | P8-01,P9-01 | Size/duration/rotation logic and tests compile; captured file must open in Wireshark | `crates/sabicheiro-capture/src/recording.rs` | Pktmon max four parts; dumpcap ring four; privilege QA pending |
| P17-01 | Baselines | Explainable persistent unusual-activity heuristics | P2 | DONE | P13-01 | Deterministic/statistical and persistence tests pass | `crates/sabicheiro-core/src/baseline.rs` | Neutral signals, no AI/risk score |
| P18-01 | Diagnostics | CPU/RAM/FPS/rates/DB/queue/drop counters | P1 | DONE | P5-01 | Backend counters, provenance, self-test and settings rendering pass available QA | core/Tauri/Settings | Local only; capture drops remain backend-dependent |
| P19-01 | Packaging | NSIS, AppImage and deb configuration/artifacts | P0 | REVIEW | P5-01 | Windows 0.2.0 portable/NSIS artifacts built and smoke-tested; Linux artifacts need Linux runner | Tauri config/workflows | Workflows manual-only; no paid runner was used |
| P20-01 | Release validation | Tests, a11y, visual, privacy and package audit | P0 | REVIEW | P19-01 | Evidence table records every pass and remaining check | `docs/VALIDATION.md` | Rust/TS/license/Windows/visual pass; Linux, ETW/Pktmon privilege and OS screen reader remain |

## Completed reliability improvements in 0.1.1

| ID | Improvement | Status | Verified result |
| --- | --- | --- | --- |
| R-01 | Native Unicode adapter discovery | DONE | Windows friendly names no longer pass through a legacy PowerShell code page; addresses and operational state come from the native adapter API |
| R-02 | Remote-peer feed | DONE | Wildcard TCP listeners and bound UDP sockets are hidden by default and cannot pollute destination baselines; an explicit setting restores them for service auditing |
| R-03 | Established socket proof | DONE | Native provider and full live-engine tests create a real TCP pair and confirm owner-PID/state/remote-peer observation through ingestion |
| R-04 | Honest live traffic totals | DONE | The bottom strip measures aggregate interface byte/packet deltas; rows state when per-flow counters are unavailable |
| R-05 | Stale adapter recovery | DONE | A saved interface that was renamed or previously mojibaked falls back to all interfaces and the corrected setting is persisted locally |

## Functional roadmap implemented for 0.2.0

All items below must remain local-first and usable without subscriptions, commercial
drivers, hosted databases or paid APIs.

| ID | Improvement | Priority | Status | Dependencies | Acceptance criterion / evidence |
| --- | --- | --- | --- | --- | --- |
| F-01 | Windows ETW network event provider | P0 | REVIEW | P8-01 | Bounded no-payload ETW queue, loss diagnostics and polling fallback are implemented; live proof is blocked because this normal-user Windows session returns access denied |
| F-02 | Per-flow byte and packet correlation | P0 | REVIEW | F-01 | PID/5-tuple ETW deltas retain explicit provenance and interface totals are never divided; privileged live ETW reconciliation remains pending |
| F-03 | Stable adapter identity and hot-plug | P1 | DONE | R-01 | Adapter name, permanent name, LUID and GUID are separate; the engine refreshes every five seconds and safely falls back after removal |
| F-04 | Remote, local and listener workspaces | P1 | DONE | R-02 | Separate accessible tabs/counts filter shared evidence without mixing semantics |
| F-05 | DNS Client event correlation | P1 | REVIEW | F-01 | Bounded expiring correlation and parser tests pass; live DNS Client ETW proof shares the Windows permission dependency |
| F-06 | Traffic timeline and process sparklines | P1 | DONE | R-04,F-02 | 1 s/1 min/1 h storage, pause/zoom, process sparklines and source provenance pass storage/UI tests |
| F-07 | GeoIP onboarding and offline import | P1 | DONE | P11-01 | Empty-map guidance plus size/edition/checksum validation and atomic local City/ASN import are implemented |
| F-08 | Capture self-test panel | P1 | DONE | R-01,R-03 | Interface, real loopback table/process, DNS, GeoIP and SQLite checks return local pass/warn/fail diagnostics |
| F-09 | Rule-based local notifications | P2 | DONE | P17-01 | Opt-in new-process/country/ASN/port rules use per-rule cooldowns and never block traffic |
| F-10 | Session comparison | P2 | DONE | P13-01 | Process/destination/country/port deltas and CSV/JSON export have deterministic tests |
| F-11 | Linux Netlink `INET_DIAG` provider | P1 | REVIEW | P9-01 | Deprecated `/proc/net` socket parsing is replaced; UAPI/IPv4/IPv6 tests and Linux cross-compilation pass, real Linux permission/performance proof remains |
| F-12 | Diagnostic support bundle | P2 | DONE | F-08 | User-reviewed ZIP32 contains versions and redacted diagnostics/settings/logs; raw IP history, payloads, recordings and secrets are excluded; ZIP structure is tested |

## Post-0.1 hardening queue

| ID | Title | Priority | Status | Acceptance criterion |
| --- | --- | --- | --- | --- |
| H-01 | Linux-native release proof | P0 | TODO | Run full gates and open AppImage/deb on Debian/Ubuntu x86_64 |
| H-02 | Minimal privileged recording helper | P0 | TODO | UI remains normal user; fixed authenticated IPC; Windows elevation and Linux capabilities audited |
| H-03 | Packet/event-to-flow counters | P1 | REVIEW | Implementation and provenance tests pass; real ETW byte events require an allowed/elevated trace session |
| H-04 | Automated visual/a11y regression | P1 | REVIEW | 1280×720/1920×1080 screenshots, semantic tree and Ctrl+K/Escape paths pass; an OS screen-reader run is still required |
| H-05 | Netlink `INET_DIAG` provider | P2 | REVIEW | No `/proc/net` socket parsing remains and the Linux target compiles; parity/performance must run on Linux |
| H-06 | Backend explanation localization | P2 | DONE | Generic failures and dynamic baseline/self-test explanations are localized while expandable original diagnostic detail remains available |
