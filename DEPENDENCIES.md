# Dependency and Cost Review

Review date: 2026-08-09. Every required runtime feature has zero service/API cost.
Permissive-license notices are retained in release materials; DB-IP attribution is
shown in Settings/About and release notices.

| Dependency | Purpose | License | Cost | Redistribution | Notes |
| --- | --- | --- | --- | --- | --- |
| Tauri 2 / `@tauri-apps/*` | desktop shell and typed IPC | MIT OR Apache-2.0 | €0 | Allowed with notice | Uses system WebView2/WebKitGTK; no Tauri cloud required |
| Rust / Cargo | core toolchain | MIT OR Apache-2.0 (components vary) | €0 | Toolchain not bundled | Stable MSVC/GNU toolchains |
| React / React DOM | complex desktop UI composition | MIT | €0 | Allowed with notice | No hosted React service |
| Three.js | local WebGL globe/arcs | MIT | €0 | Allowed with notice | Renderer is entirely local |
| Vite + React plugin | frontend build | MIT | €0 | Development tooling | Not a runtime service |
| TypeScript | strict frontend types | Apache-2.0 | €0 | Development tooling | `strict: true`, no `any` policy |
| TopoJSON Client | decode local world geometry | ISC | €0 | Allowed with notice | No online tiles |
| world-atlas | Natural Earth TopoJSON distribution | ISC; source data public domain | €0 | Allowed | Archived/stable dataset; vendored by package lock |
| serde / serde_json | typed Rust serialization | MIT OR Apache-2.0 | €0 | Allowed with notice | IPC/storage boundaries |
| thiserror | typed Rust errors | MIT OR Apache-2.0 | €0 | Allowed with notice | No service |
| chrono | UTC timestamps | MIT OR Apache-2.0 | €0 | Allowed with notice | Local time formatting remains UI-side |
| rusqlite + bundled SQLite | local history database | MIT / public domain SQLite | €0 | Allowed | Bundled avoids system DB dependency |
| maxminddb | offline MMDB reader | ISC | €0 | Allowed with notice | Reads DB-IP Lite-compatible files |
| sysinfo | bounded process/resource diagnostics | MIT | €0 | Allowed with notice | Refresh only required subsets |
| dns-lookup | opt-in system reverse DNS adapter | MIT OR Apache-2.0 | €0 | Allowed with notice | Off by default; cached and off capture thread; may contact configured resolver |
| reqwest + rustls | explicit DB-IP Lite download | MIT OR Apache-2.0 | €0 | Allowed with notice | Used only when the user requests an update |
| flate2 | unpack the downloaded MMDB archive | MIT OR Apache-2.0 | €0 | Allowed with notice | Local decompression with a size bound |
| windows-sys | Windows IP Helper/process FFI | MIT OR Apache-2.0 | €0 | Allowed with notice | Target-specific; calls OS APIs |
| libc | Linux signal handling for clean dumpcap stop | MIT OR Apache-2.0 | €0 | Allowed with notice | Target-specific; no service |
| tracing | capture debug instrumentation | MIT | €0 | Allowed with notice | No remote collector; lifecycle file logging uses only Rust std/chrono |
| tempfile (dev) | isolated storage tests | MIT OR Apache-2.0 | €0 | Development only | Not shipped as a service |
| DB-IP Lite | offline IP→city/ASN dataset | CC BY 4.0 | €0 | Allowed with attribution | Monthly public download; reduced accuracy; optional update |
| Natural Earth | globe/map geography | Public domain | €0 | Allowed | Fully local; no tile provider |
| WebView2 | Windows webview runtime | Microsoft runtime terms | €0 | Evergreen/bootstrap permitted by Tauri | Included with supported Windows versions |
| WebKitGTK | Linux system webview | LGPL-2.1+ components | €0 | Dynamically linked system dependency | Installed by distro, not modified/bundled by source tree |
| Pktmon | optional Windows packet recording | Windows inbox component | €0 | Not redistributed | Windows 10 2004+/11; elevation may be required |
| PowerShell + NetTCPIP module | fixed Windows interface discovery | Windows inbox component | €0 | Not redistributed | No user input is interpolated into the command |
| iproute2 `ip` | fixed Linux interface discovery | GPL-2.0-or-later | €0 | System dependency, not bundled | Used only for address/interface enumeration |
| dumpcap/Wireshark | explicit Linux PCAPNG recording | GPL-2.0-or-later | €0 | System dependency, not bundled | Optional; distro capture-group/capability setup |
| libpcap/Npcap runtime | dumpcap dependency / possible future source | BSD (libpcap); Npcap custom | €0 baseline | **Not redistributed** | Npcap is never mandatory; its restrictions prevent bundling |
| GitHub Actions workflows | optional build automation | service terms vary by account/repository | Not required | N/A | Workflows are manual-only; local/self-hosted builds guarantee no hosted-runner charge |

## Rejected mandatory dependencies/services

- Google Maps, Mapbox, HERE, Bing tiles and Cesium Ion: paid/hosted dependency.
- GeoIP query APIs: leak observed IPs, impose quotas and may become paid.
- GeoLite2 as the sole dataset: current download requires an account/license key;
  supported manually but DB-IP Lite is the no-account baseline.
- Npcap redistribution: free edition terms are unsuitable for mandatory redistribution.
- Sentry/analytics SDKs, cloud databases, authentication and LLM APIs: violate local,
  private, zero-cost operation.

## Source evidence

- [Tauri licensing and architecture](https://github.com/tauri-apps/tauri/blob/dev/ARCHITECTURE.md)
- [Three.js MIT license](https://github.com/mrdoob/three.js/blob/dev/LICENSE)
- [Natural Earth public-domain terms](https://www.naturalearthdata.com/about/terms-of-use/)
- [DB-IP Lite CC BY 4.0 downloads](https://db-ip.com/db/lite.php)
- [Rusqlite MIT/bundled SQLite guidance](https://github.com/rusqlite/rusqlite)
- [Sysinfo MIT and performance guidance](https://github.com/GuillaumeGomez/sysinfo)

## Locked transitive audit

The 2026-08-09 locked audit inspected every package manifest available locally:

- Cargo metadata: **493 packages**, zero missing license declarations. The set is
  composed of MIT/Apache/ISC/BSD/Zlib/Unicode/MPL-2.0 and other compatible permissive
  expressions (including CDLA-Permissive-2.0 data); no commercial-only crate appears.
- pnpm store: **43 unique packages**, zero missing license declarations: 30 MIT, 4
  Apache-2.0, 3 ISC, 3 Apache-2.0 OR MIT, 2 MPL-2.0 and 1 BSD-3-Clause.

MPL components remain unmodified dependencies, and GPL system tools are invoked only
when already installed; they are not linked into or redistributed with Sabicheiro.

The lockfiles are the release bill of materials. Before release, run the dependency
license audit described in `BUILD.md` and reconcile any new transitive license here.
