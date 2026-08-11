# Sabicheiro 0.1.1 validation record

Validation date: 2026-08-10. This record separates evidence produced on the available
Windows host from checks that require a different operating system, elevated packet
capture or an interactive visual session. `PASS` means the listed command or check
completed successfully here. `REVIEW` means the implementation exists but that
specific runtime evidence was not produced.

## Environment

| Item | Value |
| --- | --- |
| Host | Windows NT 10.0.26200.0, x86_64 |
| Rust | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Cargo | `cargo 1.97.1 (c980f4866 2026-06-30)` |
| Node.js | `v24.14.0` |
| pnpm | `11.16.0` |
| Tauri CLI | `2.11.4` (locked application development dependency) |
| Validation constraints | No installed WSL distribution or container runtime; no hosted workflow was triggered; final GUI launch approval was rejected by the local tool usage limit |

No paid API, hosted database, cloud backend, analytics service, commercial map,
commercial capture driver, paid runner or paid plugin was used. All commands ran
locally with locked dependencies. The checked-in GitHub Actions workflows are
`workflow_dispatch` only and were not executed during this validation.

## Automated gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Rust formatting | PASS | `cargo fmt --all -- --check` |
| Rust lints | PASS | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Rust tests | PASS | `cargo test --workspace --all-targets --locked`: 41 passed, 0 failed |
| Frontend type/build check | PASS | `pnpm --dir apps/desktop check`; TypeScript and Vite production build completed for the final frontend source |
| Windows release executable | PASS | `cargo build --release -p sabicheiro-desktop --locked`; optimized 0.1.1 executable produced offline |
| Windows NSIS bundle | PASS | Previously generated Tauri NSIS script updated to 0.1.1 and compiled with the cached local `makensis`; no download used |
| Cargo license declarations | PASS | 493 locked packages inspected; 0 missing declarations |
| pnpm license declarations | PASS | 43 unique locked packages inspected; 0 missing declarations |
| SQLite runtime store | PASS | read-only `PRAGMA integrity_check` returned `ok`; two schema migrations recorded |
| Unicode adapter discovery | PASS | native Windows provider returned interface addresses and no Unicode replacement characters |
| Established-peer capture | PASS | native provider and full live engine both ingested a real established TCP pair with matching PID/state/endpoints |
| Listener classification | PASS | wildcard TCP/bound UDP sockets are hidden by default and excluded from remote baselines |
| Aggregate interface rates | PASS | deterministic delta/rate test plus live `sysinfo` counter integration; totals remain separate from flows |
| Exact 0.1.1 GUI smoke | REVIEW | launch escalation was requested but rejected by the local tool usage limit; no alternate UI automation was attempted |

The 41 Rust tests are distributed as follows: capture 10, core 13, desktop library 6,
geo 4 and storage 8. The desktop binary target contains no separate unit tests.

The frontend production output contained 61 transformed modules. The detailed map and
globe are independently lazy-loaded so their geometry does not increase the initial
JavaScript payload:

| Asset | Minified | gzip |
| --- | ---: | ---: |
| CSS | 38.64 kB | 8.47 kB |
| Shared mesh JavaScript | 3.79 kB | 1.61 kB |
| Base JavaScript | 295.51 kB | 89.97 kB |
| Lazy `GlobeScene` JavaScript | 662.19 kB | 179.70 kB |
| Lazy detailed `MapView` JavaScript | 759.24 kB | 244.56 kB |

Vite emitted a non-fatal warning because the lazy Three.js globe and detailed map
chunks exceed 500 kB. Both are dynamically imported; the base application payload is
smaller than before this split, so this is not a failed build gate.

## Windows artifacts

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/release/Sabicheiro.exe` | 13,628,928 | `D1CAF2B7F9F143A3F5120F5F6094C5CFA90434AB57E37769F8541B21FA5427E0` |
| `target/release/bundle/nsis/Sabicheiro_0.1.1_x64-setup.exe` | 3,786,610 | `F14D2725ADC1512D3FE509D9156B58FB7C9A7344C9D0C87152CF08219CA905B4` |
| `target/release/bundle/nsis/Sabicheiro-Setup.exe` | 3,786,610 | `F14D2725ADC1512D3FE509D9156B58FB7C9A7344C9D0C87152CF08219CA905B4` |

The stable installer filename is a byte-identical copy of the versioned NSIS bundle.
Both executable and installer Windows resources report file/product version `0.1.1`.
NSIS compilation completed with only its generated unused-function optimization
warning. The exact 0.1.1 GUI process could not be launched after the approval request
was rejected by the tool usage limit; the earlier 0.1.0 lifecycle smoke remains valid
for the unchanged shell/logging path, while new 0.1.1 tests exercise the complete live
capture pipeline. A full interactive install/uninstall cycle remains pending.

The native run created/opened the local application database at
`%APPDATA%/io.sabicheiro.desktop/sabicheiro.sqlite3`. A read-only audit found seven
application tables and an intact database. Empty connection/session tables are
expected because the smoke run did not enable capture or demo generation.

## Locked dependency audit

The Cargo graph contains 493 packages and no missing license declaration. Resolved
expressions are free/open-source terms such as MIT, Apache-2.0, ISC, BSD, Zlib,
Unicode-3.0, MPL-2.0 and CDLA-Permissive-2.0. The pnpm graph contains 43 unique
packages: 30 MIT, 4 Apache-2.0, 3 ISC, 3 Apache-2.0 OR MIT, 2 MPL-2.0 and 1
BSD-3-Clause. See `DEPENDENCIES.md` for purpose, cost and redistribution decisions.

## Checks still requiring their target environment

| Check | Result | Reason / next evidence |
| --- | --- | --- |
| Debian/Ubuntu AppImage and deb | REVIEW | Linux source and bundle configuration exist, but this host has no usable WSL distribution or container runtime. Build, launch and uninstall on Linux before release. |
| Linux `/proc/net` live provider | REVIEW | Parser/provider tests pass on Windows; exercise PID attribution and permissions on a real Linux host. |
| Linux dumpcap recording | REVIEW | Requires Linux plus a locally installed/configured dumpcap. Open every generated part in Wireshark. |
| Windows Pktmon recording | REVIEW | Inbox `PktMon.exe` is present, but `pktmon status` returned access denied as a normal user. No elevated network recording was started. |
| Visual and accessibility QA | REVIEW | The in-app browser navigation failed before the preview page loaded, and its security policy then prohibited a retry or alternate browser surface. No screenshot was fabricated or labelled as verified. Run target-size, keyboard and screen-reader checks interactively. |
| Exact 0.1.1 native window lifecycle | REVIEW | GUI launch approval was rejected because the local tool usage limit had been reached. Launch the delivered installer/executable interactively and confirm start/close. |
| NSIS install/uninstall UX | REVIEW | Bundle generation passed; exercise the per-user Start-menu entry, launch and uninstaller in a disposable Windows account/VM. |

These `REVIEW` items are also tracked in `NEXT_TODOS.md`. They are deliberately not
presented as passes and do not weaken the local/private/zero-cost guarantees of the
implemented application.
