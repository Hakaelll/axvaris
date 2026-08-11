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
