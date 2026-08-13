# Sabicheiro 0.3.0 validation record

Validation date: 2026-08-12. This is the evidence for the **Verified Destinations**
local test release. `PASS` means the listed check ran successfully on the available
Windows host. `REVIEW` is deliberately retained for proof that requires Linux,
elevated Windows tracing/recording, an OS screen reader or an installer sandbox; it is
not a synonym for pass and does not block generating an explicitly labelled test build.

## 0.3 environment and zero-cost boundary

| Item | Value |
| --- | --- |
| Host | Windows NT 10.0.26200.0, x86_64 |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Node.js / pnpm | `v24.14.0` / `11.16.0` |
| Product version | `0.3.0` in Cargo, desktop package, Tauri and PE resources |
| Native target | `x86_64-pc-windows-msvc` |
| Linux source target | `x86_64-unknown-linux-gnu` |
| Host constraints | no WSL/container/Linux runtime; normal-user ETW access denied |

No paid API, plugin, service, hosted runner, commercial map, account, telemetry,
analytics SDK or cloud backend was added or used. The only product download path is a
user-triggered complete free DB-IP Lite dataset over HTTPS; observed IPs are never
submitted to a lookup API. Reverse DNS remains opt-in and off by default. The manual
GitHub workflows were not dispatched.

## 0.3 automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Repository policy | PASS | Desktop-only NSIS/AppImage/deb configuration, synchronized 0.3.0 versions, five referenced assets, generated/mobile/binary path bans and prohibited analytics/service dependency scan |
| Spanish catalog | PASS | `audit-i18n.mjs`: 596 literal UI keys covered; dynamic diagnostic/filter text routes through the catalog |
| Frontend types/build | PASS | TypeScript project build and Vite 8 production build; 63 modules transformed |
| Rust formatting | PASS | `cargo fmt --all -- --check` |
| Rust lints | PASS | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Rust tests | PASS | `cargo test --workspace --all-targets --locked`: 68 passed, 0 failed, 1 privilege-sensitive ETW test ignored by default |
| Windows native capture | PASS | Native IP Helper tables, Unicode adapters, process attribution and established TCP peer/live-engine ingestion passed |
| Linux capture source | PASS | Core/capture all-target cross-check passed; native TCP/UDP IPv4/IPv6 fixture tests compile and source contains `INET_DIAG`, not `/proc/net` socket parsing |
| Offline GeoIP | PASS | Synthetic City/ASN MMDB covers exact/longest-prefix, IPv4/v6, partial/missing/invalid data and independent generations; lifecycle rollback tests pass |
| SQLite v4 | PASS | v1–v3 file migrations, readable backup, rollback on failure, provenance, investigation, derived-view atomicity, retention and export tests pass |
| IPC/reliability | PASS | Delta sequence/resync, full-statistics/bounded-payload, suspension/network-generation reconciliation and cancelled re-enrichment tests pass |
| Locked licenses | PASS | Offline local audit: 519 Cargo and 42 pnpm packages; zero missing, proprietary, commercial-only or unlicensed declarations |

The workspace test distribution is capture 16 passed plus one ignored ETW proof, core
20, desktop library 11, GeoIP/DNS 7 and storage 14. The Linux core/capture command was:

```text
cargo check -p sabicheiro-core -p sabicheiro-capture --all-targets \
  --target x86_64-unknown-linux-gnu --locked
```

Extending that Windows cross-check through Geo/storage reaches the native `ring` build
and correctly requires an `x86_64-linux-gnu-gcc` cross toolchain, which is not installed.
No system component was installed merely to turn that host limitation into a checkmark.

## Performance and boundedness

The release-mode harness is versioned at `docs/performance-0.3.0.json`. All cases use
nine measured iterations and explicit limits of 100 ms filter p95, 1,500 ms
serialization p95 and 2,500 serialized bytes per connection.

| Connections | Filter p95 | Serialization p95 | JSON bytes | Result |
| ---: | ---: | ---: | ---: | --- |
| 1,000 | 0.1556 ms | 2.4234 ms | 1,204,501 | PASS |
| 10,000 | 1.8011 ms | 23.2542 ms | 12,147,769 | PASS |
| 50,000 | 13.5622 ms | 157.8122 ms | 61,113,954 | PASS |

The required 30-minute soak completed 4,460 samples at a 400 ms interval. Latency p95
was 19.4723 ms; memory stayed between 10,108,928 and 11,403,264 bytes, a growth of
1,294,336 bytes against the 32 MiB limit. Queue/payload bounds remained enabled and the
soak result is `PASS`.

## Visual, interaction and accessibility evidence

The local deterministic preview was inspected without external navigation at
1280×720 and 1920×1080. The exercised matrix included first run/capture-off, dense Demo
data, 3D globe, 2D map, normal and expanded map, selection/inspector, zoom controls,
selection framing, Escape restoration, arrow-key pan, keyboard zoom, comfortable and
compact density, and English/Castellano. State survived map expansion/restoration.

The browser semantic tree exposed named banner/navigation/main/footer regions, capture
controls, visualization toggle, map application/toolbar, destination markers, feed,
settings sections, form labels and the self-test capability table. The review found and
fixed two untranslated dynamic strings and removed conflicting `listitem` roles from
interactive process/destination buttons. The offline self-test displayed pass status
for endpoint/process/interface/close across TCP/UDP and IPv4/IPv6 in deterministic
preview mode.

The automated in-app browser later denied one additional local ASN-profile click under
its security policy. That denial was not bypassed through another browser surface. The
ASN profile and traceability path are instead covered by the desktop test
`asn_profile_and_identity_timeline_trace_retained_connections`.

The 2560×1440 / 125% / 150% screenshots and an actual Narrator/NVDA session remain
`REVIEW`; responsive CSS, reduced-motion rules, keyboard behavior and the semantic tree
are present, but this record does not claim an OS screen-reader certification that did
not run.

## Windows 0.3 test artifacts

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/Sabicheiro.exe` | 18,836,480 | `D4710F6A0C9B311D7A56CA9C55635F0A809DD3C5DABFC848E91A85209EDF9048` |
| `target/release/bundle/nsis/Sabicheiro_0.3.0_x64-setup.exe` | 8,232,758 | `A27A0D14CDB9BB3EEC2E6CAD52DD94A2C7AC1A7BCF59DD273CE2E20D8C4E65B2` |
| `release/Sabicheiro-0.3.0-windows-x64.zip` | 17,606,621 | `752FDFEB070CDDF4B185998461A3ED95562BC25EB20E50F1146FAAB59C79BEA1` |

The executable and installer report file/product version `0.3.0`. The optimized
portable executable started as a normal user, stayed alive for six seconds and accepted
a close-window request; because the hidden smoke process did not exit inside the
bounded five-second wait, the harness terminated that exact process. This proves launch
but is not presented as installer/uninstaller UX evidence. The handoff directory
contains portable and NSIS names, checksums, privacy/third-party notices and test steps.

## Target-specific checks still in review

| Check | Result | Exact reason / next evidence |
| --- | --- | --- |
| Windows ETW network/DNS | REVIEW | Explicit ignored proof was run and Windows returned `ERROR_ACCESS_DENIED`; IP Helper remained the documented fallback. Run in an administrator-approved trace context. |
| Windows Pktmon recording | REVIEW | Requires approved elevation and can create sensitive packet payload; no recording was started implicitly. |
| Linux native capture | REVIEW | Native fixture suite exists and cross-compiles, but this host has no Linux runtime. Run ordinary tests on Ubuntu/Debian and record namespace/permission variants. |
| Linux AppImage/deb | REVIEW | Must be built, launched, installed and removed natively on Linux; Windows cross-packaging is not accepted as proof. |
| OS screen reader / full scale matrix | REVIEW | Run Narrator/NVDA plus 2560×1440 at 125% and 150%; semantic/keyboard automation already passes the available subset. |
| Clean pnpm bootstrap | REVIEW | `pnpm install --frozen-lockfile --offline` could not open the pnpm-store SQLite index outside the writable workspace. Existing locked dependencies passed TypeScript/Vite/package builds and were not modified. |
| NSIS install/uninstall UX | REVIEW | Package generation and portable launch pass; installation was not performed because it would change the user's system. Use a disposable account/VM and verify chosen data retention on uninstall. |

These reviews are why 0.3.0 is described as a **local test release**, not a fully signed
cross-platform production publication. No unknown endpoint, process, counter, domain,
ASN or coordinate is fabricated when optional proof is unavailable.

---

# Sabicheiro 0.2.0 validation record

Validation date: 2026-08-11. This record separates evidence produced on the available
Windows host from checks that require Linux, an elevated capture context or an OS
screen reader. `PASS` means the listed command/check completed successfully here.
`REVIEW` means the implementation exists but that target-specific runtime evidence was
not produced; it is never used as a synonym for pass.

## Environment

| Item | Value |
| --- | --- |
| Host | Windows NT 10.0.26200.0, x86_64 |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Node.js | `v24.14.0` |
| pnpm | `11.16.0` |
| Tauri CLI | `2.11.4` (locked development dependency) |
| Native target | `x86_64-pc-windows-msvc` |
| Cross-check target | `x86_64-unknown-linux-gnu` |
| Constraints | No installed WSL/container runtime; no hosted workflow; normal-user ETW/Pktmon access denied |

No paid API, hosted database, cloud backend, analytics service, commercial capture
driver, paid runner or paid plugin was used. Required features remain local-first and
zero-service-cost. GitHub Actions workflows are manual-only and were not dispatched.

## Automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Rust formatting | PASS | `cargo fmt --all -- --check` |
| Rust lints | PASS | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Rust tests | PASS | `cargo test --workspace --all-targets --locked`: 47 passed, 0 failed, 1 privilege-sensitive ETW proof ignored by default |
| Frontend type/build | PASS | TypeScript project build plus Vite production build; 63 modules transformed |
| Linux capture cross-check | PASS | `cargo check -p sabicheiro-capture --target x86_64-unknown-linux-gnu --locked` |
| Windows native tables | PASS | Real established loopback TCP pair observed with matching PID/state/endpoints |
| Adapter discovery | PASS | `GetAdaptersAddresses` returns stable name/LUID/GUID, Unicode names and addresses |
| SQLite schema v3 | PASS | v1→v3 migration plus connection, timeline provenance, retention, recording and baseline round trips |
| Session comparison | PASS | Deterministic process/port additions and removals plus CSV/JSON generation paths |
| Support ZIP | PASS | ZIP32 local/central directory consistency, entry count and allow-listed names tested |
| Windows release build | PASS | `pnpm --dir apps/desktop tauri build --bundles nsis` |
| Native executable smoke | PASS | Optimized `0.2.0` started, stayed alive for six seconds, accepted `CloseMainWindow` and exited cleanly |
| Locked Cargo licenses | PASS | 515 packages, zero missing license declarations |
| Locked pnpm licenses | PASS | 43 unique packages, zero missing declarations; graph unchanged by 0.2 frontend work |

Rust tests are distributed as follows: capture 13 passed plus one ignored live-ETW
proof, core 13, desktop library 8, Geo/DNS 4 and storage 9. The ignored test is run
explicitly with:

```text
cargo test -p sabicheiro-capture live_etw_observes_loopback_flow_counters --locked -- --ignored --nocapture
```

On this normal-user host it correctly failed at ETW session creation with Windows
`ERROR_ACCESS_DENIED`; therefore it is not counted as live ETW evidence.

## Frontend and accessibility evidence

The deterministic local preview was exercised through the in-app browser at 1280×720
and 1920×1080:

- welcome and populated Demo globe/feed;
- Remote/Local/Listeners tabs and counts;
- Settings, notification rules, offline MMDB import, support bundle and diagnostics;
- English/Castellano switching, including new backend/self-test labels;
- History traffic bars, Time Machine and session-comparison controls;
- semantic headings, navigation, tablist, forms, table/image alternatives and live
  regions exposed in the accessibility tree;
- `Ctrl+K` opened the command palette and `Escape` closed it;
- browser console warnings/errors: none.

The first compact History capture exposed that the scroll position from Settings was
being retained across view changes. A `main-stage` ref now resets both axes whenever
the primary view changes; the repeat check reported `scrollTop: 0` and showed the full
History heading. No horizontal overflow was measured at 1920 px.

Vite's final production assets were:

| Asset | Minified | gzip |
| --- | ---: | ---: |
| CSS | 45.23 kB | 9.69 kB |
| Shared mesh JavaScript | 3.79 kB | 1.61 kB |
| Base JavaScript | 330.31 kB | 100.95 kB |
| Lazy `GlobeScene` JavaScript | 662.19 kB | 179.71 kB |
| Lazy detailed `MapView` JavaScript | 759.24 kB | 244.56 kB |

Vite emits a non-fatal >500 kB warning for the independently lazy-loaded Three.js
globe and detailed map. Neither chunk is part of the initial base payload.

## Windows artifacts

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/Sabicheiro.exe` | 18,058,240 | `1D4D89426AE3836EB85F4E4E66549DAD861C2B78919FDB0B1C2097D03C3298D8` |
| `target/release/bundle/nsis/Sabicheiro_0.2.0_x64-setup.exe` | 7,983,832 | `D606E4CF8FC7F28AF63871036227E87869B6A3D695159C3C934AF20492EF47E4` |

Both Windows resources report file/product version `0.2.0`. A user-facing copy is in
`release/Sabicheiro-0.2.0-windows-x64/` with portable/installer names, checksums and
Spanish release notes. The release directory is intentionally ignored by Git because
it contains generated binaries.

## Checks still requiring their target environment

| Check | Result | Reason / next evidence |
| --- | --- | --- |
| Windows ETW network/DNS events | REVIEW | Implementation, bounded queues and fallback compile; this host denies trace creation to the normal user. Run the ignored proof under an approved trace policy/elevation and confirm PID/5-tuple counters. |
| Windows Pktmon recording | REVIEW | Requires elevation here. A minimal authenticated privileged helper is not yet implemented; open generated parts in Wireshark when tested. |
| Debian/Ubuntu AppImage and deb | REVIEW | Bundle configuration exists, but native Tauri packages must be built/launched/uninstalled on Linux. |
| Linux `INET_DIAG` runtime | REVIEW | UAPI tests and cross-compilation pass; exercise IPv4/IPv6, inode→PID permission behavior and performance on Linux. |
| Linux dumpcap recording | REVIEW | Requires Linux and a locally configured dumpcap capability/group. |
| OS screen reader | REVIEW | Semantic tree and keyboard shortcuts passed; run NVDA/Narrator and a Linux screen reader before claiming platform screen-reader certification. |
| NSIS install/uninstall UX | REVIEW | Bundle generation and portable executable smoke passed; exercise Start-menu entry, installed launch and uninstaller in a disposable account/VM. |

These `REVIEW` items remain visible in `NEXT_TODOS.md`. The application degrades
explicitly when optional evidence is unavailable: no fallback invents an endpoint,
domain, coordinate, process identity, counter or security verdict.
