# Third-party notices

Sabicheiro is MIT-licensed and is assembled exclusively from dependencies that can be
used without a commercial license or paid service. Exact resolved versions are pinned
in `Cargo.lock` and `pnpm-lock.yaml`; the purpose, SPDX expression, cost and
redistribution decision for primary components are in `DEPENDENCIES.md`.

## Application libraries and data

- Tauri and most Rust ecosystem crates: MIT and/or Apache-2.0.
- React, Three.js and their runtime dependencies: MIT.
- TopoJSON Client and `world-atlas`: ISC. `world-atlas` derives its geography from
  Natural Earth, whose source data is public domain.
- SQLite: public domain; `rusqlite` is MIT.
- `maxminddb`: ISC.
- DB-IP City Lite / ASN Lite database, when installed by the user: CC BY 4.0.
  **IP geolocation by DB-IP.com** — <https://db-ip.com/db/lite.php>.

MPL-2.0 packages in the locked dependency graph are used unmodified. GPL system tools
(`iproute2`, dumpcap/Wireshark) are not linked into or redistributed with Sabicheiro.
Windows Pktmon, PowerShell/NetTCPIP and WebView2 are operating-system/runtime
components used under their platform terms and are not redistributed as Sabicheiro
source dependencies.

## Audit snapshot

On 2026-08-09, local package manifests reported 493 Cargo packages and 43 pnpm
packages, with no missing license declaration and no commercial-only dependency. See
`DEPENDENCIES.md` for the category totals and source references.

This summary accompanies the application for attribution and review. Upstream source
distributions retain their complete copyright/license files; distributors should run
the locked license audit in `BUILD.md` and include any additional notice required after
changing a dependency or dataset snapshot.
