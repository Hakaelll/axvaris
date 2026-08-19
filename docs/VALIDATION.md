# Sabicheiro validation record

`PASS` means the command or bounded native check completed on the stated host. `REVIEW`
means implementation or partial evidence exists but the full acceptance environment
was unavailable. `BLOCKED` identifies an external host/tool prerequisite. No `REVIEW`
or `BLOCKED` row is presented as a pass.

## 0.4.2 lightweight-runtime validation — 2026-08-16

| Item | Value |
| --- | --- |
| Host | Windows NT `10.0.26200`, x86_64, normal-user desktop session |
| CPU / RAM | Intel Core i7-1165G7, 8 logical CPUs / 16.932.671.488 B |
| Rust / Cargo / Node | `1.97.1` / `1.97.1` / `v24.19.0` |
| Product version | `0.4.2` in Cargo workspace, lockfile, desktop package and Tauri config |
| Native target | `x86_64-pc-windows-msvc` |
| Source-check target | `x86_64-unknown-linux-gnu` capture paths |
| Dataset | deterministic capture fixtures, 100.000-row SQLite history, simulated eight-hour samples |

### 0.4.2 automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Rust format and lints | PASS | `cargo fmt --all -- --check`; strict workspace/all-target Clippy with `-D warnings` |
| Rust tests | PASS | 133 passed, 0 failed, 1 explicit elevated live-ETW test ignored |
| Frontend | PASS | strict TypeScript, 926 used Spanish keys/no orphans, 65 production modules transformed |
| Product reachability | PASS | 34 reachable modules, 60 registered/documented/consumed IPC commands, no preview/local paths/maps in `dist` |
| Direct dependencies | PASS | 46 Cargo declarations across five workspace packages have cfg/dev/source consumers; unused desktop `thiserror` removed |
| Licenses / policy | PASS | 519 locked Cargo + 42 pnpm packages; 137 repository files; no prohibited service dependency |
| Linux capture source | PASS | locked all-target `x86_64-unknown-linux-gnu` source-check |
| History 100k | PASS | 500-row response 716.705 B; 5,77 ms first-page p95; 4,26 ms filtered p95; 100.000 unique/exported |
| Demo isolation | PASS | simulated 600 seconds, ≤4 active destinations, ≤1 event/s and zero persisted product/history rows |
| Traffic compaction | PASS | normal eight-hour fixture 2.880 raw rows to ≤24 rows; totals/rates exact; one-second rows retained |
| Engine / IPC | PASS | idle deadline ≥10 s, demo deadline ≤1 s, sequential UI poll, empty delta <512 B, zero overlaps |
| Windows release package | PASS | release executable and unsigned local NSIS bundle generated from production frontend |
| Native GUI launch/profile | BLOCKED | execution approval was rejected after the host allowance was exhausted; no result claimed |
| Linux packages / approved ETW / 8 h desktop soak | BLOCKED | require their native/elevated/long-running environments |

### 0.4.2 artifact and bundle comparison

| Metric | 0.4.1 | 0.4.2 | Result |
| --- | ---: | ---: | --- |
| Frontend | 9.612.676 B | 1.163.691 B | PASS, −87,89 % |
| Public source maps | 5.228.126 B | 0 B | PASS |
| Initial resources | 2.939.648 B | 398.749 B | PASS, −86,44 % |
| Visible logo | 2.405.703 B | 2.503 B | PASS |
| Entry JS | 456.606 B | 328.138 B | PASS, −28,14 % |
| Map + shared world | 766.774 B | 118.215 B | PASS, −84,58 % |
| Globe code chunk | 678.128 B | 539.310 B | PASS, −20,47 % |
| Portable executable | 20.589.568 B | 6.528.512 B | PASS, −68,29 % |
| NSIS installer | 8.689.212 B | 2.851.022 B | PASS, −67,19 % |
| Build/test/bundle `target` | 29.866.606.861 B baseline | 8.086.743.018 B (7,53 GiB) | PASS, below 8 GiB |

`opt-level=z` produced 6.528.000 B before the final QA marker versus 7.347.200 B
for `s`, 819.200 B smaller. Both used fat LTO, one codegen unit, aborting panics and
stripped symbols; `z` was retained. `docs/performance-0.4.2-before.json`,
`docs/performance-0.4.2-after.json`, `docs/performance-0.4.2-core.json`,
`docs/performance-0.4.2-history.json` and `docs/performance-0.4.2-runtime.json` contain
the machine-readable evidence and its exact boundary. The final 0 B target and
340.524.344 B workspace result is recorded in `docs/performance-0.4.2-space.json`.

The visual development preview was exercised in a browser: direct Today start,
capture-off state, bounded Demo, table, Map, Globe, History, Exposure, Data & Health,
controls and English/Spanish all rendered without console errors. This supports the UI
matrix but is not substituted for native capture, tray, WebView resource, egress or
installer-lifecycle evidence.

The versioned Windows launch probe uses an absolute isolated QA data directory and
waits for the production WebView's second `requestAnimationFrame`. It was not executed
after the GUI permission rejection. The absence of launch, five-minute scenario and
eight-hour numbers is explicit; no value was inferred from the browser preview or the
30-second model soak.

## Historical 0.4.1 validation — 2026-08-14

## 0.4.1 environment and release boundary

| Item | Value |
| --- | --- |
| Host | Windows NT `10.0.26200.0`, x86_64, normal-user desktop session |
| Rust / Cargo | `1.97.1` / `1.97.1` |
| Node.js / pnpm | `v24.19.0` / `11.19.0` |
| Product version | `0.4.1` in Cargo workspace, desktop package, Tauri config and PE resources |
| Native target | `x86_64-pc-windows-msvc` |
| Source-check target | `x86_64-unknown-linux-gnu` |
| Host constraints | no WSL, Docker, Podman or Linux host; ETW returns `ACCESS_DENIED`; no OS screen reader |

No paid API, hosted runner, account, telemetry, analytics SDK, remote map, cloud
database or automatic workflow was used. The new `docs.html` has no external runtime
resource or network dependency. Optional DB-IP whole-dataset download and system
reverse DNS remain disabled by default and require typed broker authorization;
`offline_enforced` rejects both.

## 0.4.1 automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Rust formatting | PASS | `cargo fmt --all -- --check` |
| Rust lints | PASS | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Rust tests | PASS | `cargo test --workspace --all-targets --locked`: 125 passed, 0 failed, 1 explicit live-ETW test ignored |
| Frontend/i18n | PASS | strict TypeScript, 843 literal Spanish keys and Vite production build; 67 modules |
| Exposure resilience fixture | PASS | malformed roots/rows, deduplication, linear snapshot comparison, IPv6 display and invalid IPC payloads exercised against compiled TypeScript |
| Documentation structure | PASS | Spanish language and heading, unique IDs, valid local anchors, no external URLs/resources and valid inline JavaScript syntax |
| Repository policy | PASS | 130 tracked release files scanned; synchronized 0.4.1 versions, three desktop bundle targets, five referenced assets and no paid/analytics dependency |
| Locked licenses | PASS | 519 Cargo plus 42 pnpm packages; zero missing, proprietary, commercial-only or unlicensed declarations |
| SPDX SBOM | PASS | Deterministic SPDX 2.3 JSON with 562 packages including Sabicheiro; generated by `scripts/generate-sbom.mjs` |
| Windows release build | PASS | `pnpm --dir apps/desktop tauri build --bundles nsis` |
| Linux capture source | PASS | `cargo check -p sabicheiro-capture --all-targets --target x86_64-unknown-linux-gnu --locked` |
| Native Linux packages | BLOCKED | AppImage/deb must be built, launched, installed and removed on a real supported Linux host |

Test distribution remains capture library 45 passed plus one ignored ETW proof, helper
binary 1, core 38, desktop 12, Geo/DNS 7 and storage 22. The 0.4.1 UI changes do not
alter capture-provider semantics; the 0.4.0 truth-fixture evidence remains the current
bounded native baseline and is preserved below rather than being represented as a new
capture run.

## 0.4.1 behaviour evidence

- Both the 2D map and 3D globe define Madrid/Spain as their initial camera target and
  reset target. TypeScript builds the same constants used by initial and reset state.
- Exposure validates IPC payloads at runtime before rendering, skips malformed rows
  with a visible warning, rejects malformed roots, deduplicates listener identities and
  compares two snapshots in linear time.
- Refresh requests are sequenced and cancellable. Late results cannot overwrite newer
  data, and unmounted/StrictMode effects cannot write stale state.
- A view-local error boundary supplies recovery UI, so an unexpected Exposure render
  failure does not tear down the application shell.
- Loading, empty, partial-data, native-error and recovery states have translated labels
  and keyboard-focusable actions.
- `docs.html` is self-contained, responsive, printable and keyboard navigable. Its
  content covers Windows/Linux installation, permissions, capabilities, limitations,
  privacy, troubleshooting, build steps and validation boundaries without claiming
  unfinished Linux eBPF or native package evidence.

## 0.4.1 performance and boundedness

`docs/performance-0.4.1.json` was produced in release mode with nine iterations per
case and a 30-second bounded soak smoke:

| Connections | Filter p95 | Serialization p95 | JSON bytes | Result |
| ---: | ---: | ---: | ---: | --- |
| 1,000 | 0.0845 ms | 1.6592 ms | 1,347,501 | PASS |
| 10,000 | 1.5530 ms | 22.8866 ms | 13,577,769 | PASS |
| 50,000 | 11.6702 ms | 235.8731 ms | 68,263,954 | PASS |
| 100,000 | 26.9164 ms | 554.9883 ms | 136,632,519 | PASS |

The 75 soak samples had 12.8264 ms cycle p95 and memory stayed between 10,440,704 and
10,563,584 bytes: 122,880 bytes growth against the 32 MiB smoke limit. The release
interaction target of 100 ms passes. This short run is not substituted for the required
eight-hour desktop soak, sleep/resume/network-change sequence or battery profile.

## 0.4.1 Windows artifacts and installation evidence

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/Sabicheiro.exe` | 20,589,568 | `1608974b36a40428f471e6aeb1d26c0fb026258870b95f61744840244f6a2812` |
| `target/release/bundle/nsis/Sabicheiro_0.4.1_x64-setup.exe` | 8,689,212 | `ab9b084ad86f14575d192d9733f9bc40936a60c2bc5a722a5e123665b2709a19` |
| `docs/sbom-0.4.1.spdx.json` | 381,987 | `a47947b5c494617ad642068736d46a0122c981e8b363ea211fc41e5af3bacf9c` |
| `docs.html` | 81,340 | `c07eb5b05f57d0572a63b6ec97bb21ded6adc7ee36e34a2bdfdb2f803bf93591` |

The portable executable stayed alive for the eight-second normal-user launch smoke and
closed. Before the installer cycle, an orphaned 0.4.0 uninstall record pointed to an
absent program directory; its exact key was backed up and removed. The intentionally
retained NSIS last-install location then placed 0.4.1 in that per-user directory. The
installed executable reported 0.4.1 and stayed alive for eight seconds. Its exact
uninstaller removed the program directory, uninstall registry record, desktop shortcut
and Start-menu shortcut while retaining `%LOCALAPPDATA%/io.sabicheiro.desktop`, as
designed. No Sabicheiro process or active installation remained after the cycle.

## 0.4.1 open target gates

| Gate | State | Exact next evidence |
| --- | --- | --- |
| Windows ETW/helper approved mode | REVIEW | Run the truth fixture in an approved elevated trace/helper context; inject loss/restart and record the capability transition. |
| Linux eBPF + fallback | BLOCKED | Implement and validate the production event provider on Ubuntu/Debian with BTF and without capabilities, then compare against the truth manifest. |
| Linux AppImage/deb | BLOCKED | Native build, launch, install, update from 0.3/0.4, uninstall and data-retention proof. |
| Firewall evidence | REVIEW | Validate read-only native policy adapters; Exposure correctly says `not_queryable` and never equates wildcard binding with Internet reachability. |
| Handshake provider wiring | REVIEW | Run local TLS/QUIC live fixtures with byte/packet/time bounds and persistence scanning. |
| Eight-hour/resource/battery soak | REVIEW | Run full desktop capture with storm, sleep/resume, network change and low-battery policy on recorded hardware. |
| Visual/Narrator/NVDA/HiDPI | REVIEW | Run the native five-flow keyboard and screen-reader matrix at all required sizes/scales. |
| Stable release gate | BLOCKED | Close every P0 row above, regenerate platform artifacts/SBOM/hashes and reconcile release notes. |

`NEXT_TODOS.md` remains the authoritative per-item status ledger. Sabicheiro 0.4.1 is a
complete maintenance release package for the requested fixes, while the broader P0
acceptance gates remain explicitly open.

---

# Sabicheiro 0.4.0 validation record

Validation date: 2026-08-14. This record describes the **Process to Path** validation
candidate. `PASS` means the command or bounded native check completed on the stated
host. `REVIEW` means implementation or partial evidence exists but the full acceptance
environment was unavailable. `BLOCKED` identifies an external host/tool prerequisite.
No `REVIEW` or `BLOCKED` row is presented as a pass, and the candidate is not a stable
cross-platform release while any P0 row remains open.

## Environment and release boundary

| Item | Value |
| --- | --- |
| Host | Windows NT `10.0.26200.0`, x86_64, normal-user desktop session |
| Rust / Cargo | `1.97.1` / `1.97.1` |
| Node.js / pnpm | `v24.19.0` / `11.19.0` |
| Product version | `0.4.0` in Cargo workspace, desktop package and Tauri config |
| Native target | `x86_64-pc-windows-msvc` |
| Source-check target | `x86_64-unknown-linux-gnu` |
| Host constraints | no WSL, Docker, Podman or Linux host; ETW returns `ACCESS_DENIED`; CIM hardware query denied |

No paid API, hosted runner, account, telemetry, analytics SDK, remote map, cloud
database or automatic workflow was used. Optional DB-IP whole-dataset download and
system reverse DNS are disabled by default and require typed broker authorization;
`offline_enforced` rejects both.

## Automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Rust formatting | PASS | `cargo fmt --all -- --check` |
| Rust lints | PASS | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Rust tests | PASS | `cargo test --workspace --all-targets --locked`: 125 passed, 0 failed, 1 explicit live-ETW test ignored |
| Frontend/i18n | PASS | `pnpm --dir apps/desktop check`: 839 literal Spanish keys, strict TypeScript and Vite production build; 66 modules |
| Repository policy | PASS | 124 tracked/untracked release files scanned; synchronized 0.4.0 versions, three desktop bundle targets, five referenced assets, no mobile/generated artifact or hosted analytics dependency |
| Locked licenses | PASS | 519 Cargo plus 42 pnpm packages; zero missing, proprietary, commercial-only or unlicensed declarations |
| SPDX SBOM | PASS | Deterministic SPDX 2.3 JSON, 562 packages including Sabicheiro; generated by `scripts/generate-sbom.mjs` |
| Windows release build | PASS | `pnpm --dir apps/desktop tauri build --bundles nsis` |
| Linux capture source | PASS | `cargo check -p sabicheiro-capture --target x86_64-unknown-linux-gnu --all-targets --locked` |
| Linux full dependency cross-check | BLOCKED | `ring` requires absent `x86_64-linux-gnu-gcc`; native Linux is the required proof regardless |

Test distribution: capture library 45 passed plus one ignored ETW proof, helper binary
1, core 38, desktop 12, Geo/DNS 7 and storage 22. Tests cover provider loss/reorder,
PID reuse, queue storms, helper authorization/bounds/arbitrary bytes, UDP/QUIC identity,
route lookup, rules/notification storms, immutable journal/replay, v1–v5 migration
rollback, tiered retention, salvage and `.sabi` byte alteration.

## Truth fixture and native capture

The loopback-only truth harness emitted TCP/UDP IPv4/IPv6 flows at 5, 20, 50, 100 and
1,000 ms plus wildcard/listener and unconnected UDP negative cases. The normal-user
Windows IP Helper run used a 10 ms fixture poll:

| Metric | Result | Target / interpretation |
| --- | ---: | --- |
| Expected / observed remote flows | 18 / 10 | snapshot limitation visible |
| True / false positive flows | 10 / 0 | zero false destinations: PASS |
| Destination precision | 100% | PASS |
| Process attribution precision | 100% | PASS; 8 unknown, 0 incorrect |
| Recall, 5–50 ms | 33.33% | event-provider target 95%: not met by snapshot |
| Recall, ≥50 ms | 66.67% | event-provider target 99%: not met by snapshot |
| Close latency p95 | 7.4386 ms | within the 1 s bound for detected rows |
| Missing interface fields | 10 | explicit unknown; no inferred interface |

This confirms that IP Helper is useful and honest but cannot substitute for an event
provider. Running `capture-fixture --provider windows-etw` as the current user returns
Windows `-2147024891 / ACCESS_DENIED`; IP Helper remains active. An administrator-
approved trace session is still required to measure ETW recall, loss and restart. No
attempt was made to elevate the webview.

The Linux provider performs bounded native `INET_DIAG` Netlink dumps and does not parse
`/proc/net` socket tables. Source cross-check passes. A production eBPF event stream is
not present in this candidate; native Ubuntu 22.04/24.04 and Debian 12 execution,
namespace/hidepid variants, capability fallback and AppImage/deb checks remain open.

## Performance and boundedness

`docs/performance-0.4.0.json` was produced in release mode with nine iterations per
case and a 30-second bounded soak smoke:

| Connections | Filter p95 | Serialization p95 | JSON bytes | Result |
| ---: | ---: | ---: | ---: | --- |
| 1,000 | 0.2761 ms | 3.4785 ms | 1,347,501 | PASS |
| 10,000 | 2.5885 ms | 52.2801 ms | 13,577,769 | PASS |
| 50,000 | 14.5330 ms | 266.8827 ms | 68,263,954 | PASS |
| 100,000 | 21.3499 ms | 322.0105 ms | 136,632,519 | PASS |

The 75 soak samples had 6.8617 ms cycle p95 and memory stayed between 10,416,128 and
10,727,424 bytes: 311,296 bytes growth against the 32 MiB smoke limit. The release
interaction target of 100 ms passes. This short run is not substituted for the required
eight-hour desktop soak, sleep/resume/network-change sequence or battery profile.

## Frontend, accessibility and visual evidence

The production build contains exactly five primary navigation actions: Today, Live,
Investigate, Exposure and Data & Health. Static/typed checks verify labelled controls,
semantic evidence-table pagination, exact row counts, keyboard-focusable query/rule
editors, reduced-motion CSS, enhanced contrast and text scaling. The 100,000-row data
path uses windowed rows with overscan while the accessible alternative is a real table.

The in-app browser was configured for 1280×720, but its first navigation raced the
local preview startup. After the server returned HTTP 200, the browser remained on its
internal `data:` error page and its URL security policy rejected returning to localhost.
That policy was not bypassed with another browser surface. Consequently the new 0.4
screens do not claim visual certification here. Native 1280×720, 1920×1080 and
2560×1440 at 100/125/150%, Narrator/NVDA, focus, contrast and horizontal-overflow
evidence remain `REVIEW`.

## Storage, privacy and security evidence

- SQLite v5 migration tests cover every file-backed legacy schema, readable backups,
  transactional rollback, immutable observations, deterministic projection hashes,
  tiered deletion and non-overwriting salvage.
- The `.sabi` parser requires an exact bounded entry set, checks ZIP CRC plus manifest
  SHA-256, rejects altered bytes/path traversal/oversized content and opens read-only.
- Helper requests are size/rate/duration/batch bounded, deny unknown JSON fields and
  protocol versions, bind a random nonce to the native client PID, and accept no shell,
  URL or arbitrary path.
- TLS ClientHello buffers are bounded, disabled by default and zeroized on clear/drop;
  handshake mode has visible settings and immutable local start/stop audit. Live
  provider wiring and QUIC handshake fixtures remain review items.
- Default and offline broker policies cannot issue an egress grant. GeoIP is restricted
  to fixed DB-IP monthly URLs and reverse DNS to the configured system resolver. An
  external proxy packet trace was not run, so QA-04 retains review status despite the
  structural/unit proof.

## Windows artifact and installation evidence

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/Sabicheiro.exe` | 20,581,376 | `e41ce828df2057267ee5ad74482409c042e0ea9cd6adf59d3b06812e0aa78e85` |
| `target/release/bundle/nsis/Sabicheiro_0.4.0_x64-setup.exe` | 8,691,035 | `b76314b5eb1ae5633b726b12a17444a5286cfa1bb890ba699ffd1c5ec4a77a26` |
| `docs/sbom-0.4.0.spdx.json` | 381,987 | `b2669959e393124e7b2b629fe0f287732bc19c1378a529fb84a3cda147b9ed21` |

The release executable stayed alive for the eight-second normal-user launch smoke and
closed. The NSIS package then installed per-user, its installed executable stayed alive
for five seconds, and its exact validated uninstaller removed the executable and
Start-menu shortcut. `%LOCALAPPDATA%/io.sabicheiro.desktop` remained present, proving
that uninstall did not silently delete application data. No pre-existing Sabicheiro
installation was present before the test.

## Open target gates

| Gate | State | Exact next evidence |
| --- | --- | --- |
| Windows ETW/helper approved mode | REVIEW | Run truth fixture in an approved elevated trace/helper context; inject loss/restart and record capability transition under two seconds. |
| Linux eBPF + fallback | BLOCKED | Implement/validate event provider on Ubuntu/Debian with BTF and without capabilities, then compare against the same truth manifest. |
| Linux AppImage/deb | BLOCKED | Native build, launch, install, update from 0.3, uninstall and data-retention proof. |
| Firewall evidence | REVIEW | Add and validate read-only native policy adapters; current exposure view says `not_queryable` and never equates wildcard with Internet reachability. |
| Handshake provider wiring | REVIEW | Local TLS/QUIC live fixtures with byte/packet/time bounds and persistence scan. |
| Eight-hour/resource/battery soak | REVIEW | Run full desktop capture with storm, sleep/resume, network change and low-battery policy on recorded hardware. |
| Visual/Narrator/NVDA/HiDPI | REVIEW | Native five-flow keyboard and screen-reader matrix at all required sizes/scales. |
| Stable release gate | BLOCKED | Close every P0 row above, regenerate platform artifacts/SBOM/hashes and reconcile release notes. |

`NEXT_TODOS.md` is the authoritative per-item status ledger. This record deliberately
preserves unresolved gates rather than manufacturing release evidence.

---

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
