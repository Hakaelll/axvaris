# Third-party notices

Axvaris is MIT-licensed and is assembled exclusively from dependencies that can be
used without a commercial license or paid service. Exact resolved versions are pinned
in `Cargo.lock` and `pnpm-lock.yaml`; the purpose, SPDX expression, cost and
redistribution decision for primary components are in `DEPENDENCIES.md`.

## Application libraries and data

- Geist and Geist Mono: SIL Open Font License 1.1. Local variable WOFF2 files from [Vercel's Geist repository](https://github.com/vercel/geist-font); the license is packaged in `apps/desktop/public/fonts/OFL.txt`.

- Tauri and most Rust ecosystem crates: MIT and/or Apache-2.0.
- React, Three.js and their runtime dependencies: MIT.
- TopoJSON Client and `world-atlas`: ISC. `world-atlas` derives its geography from
  Natural Earth, whose source data is public domain.
- NASA Earth Observatory's [Blue Marble: Next Generation with Topography and Bathymetry](https://science.nasa.gov/earth/earth-observatory/blue-marble-next-generation/base-topography-bathymetry/), January 2004: packaged raster relief backdrop. NASA is the source; its [media usage guidelines](https://www.nasa.gov/nasa-brand-center/images-and-media/) permit factual use without implying endorsement. Axvaris renders the original raster in neutral tones and keeps connection markers separate. Source and projection details are in `apps/desktop/public/maps/README.md`.
- SQLite: public domain; `rusqlite` is MIT.
- `maxminddb`: ISC.
- `ferrisetw` and `sha2`: MIT OR Apache-2.0.
- `mmdb-writer`: MIT, development-only synthetic MMDB fixture generation.
- DB-IP City Lite / ASN Lite database, when installed by the user: CC BY 4.0.
  **IP geolocation by DB-IP.com** — <https://db-ip.com/db/lite.php>.
- The embedded IANA service/port release subset follows the IANA licensing terms and
  is used only as an offline display hint.
- The embedded Public Suffix List release subset is MPL-2.0 and is used only for local
  registrable-domain grouping.

MPL-2.0 packages in the locked dependency graph are used unmodified. GPL system tools
(`iproute2`, dumpcap/Wireshark) are not linked into or redistributed with Sabicheiro.
Windows Pktmon and WebView2 are operating-system/runtime
components used under their platform terms and are not redistributed as Sabicheiro
source dependencies.

## Audit snapshot

On 2026-08-14, local package manifests reported 519 Cargo packages and 42 pnpm
packages, with no missing license declaration and no commercial-only dependency. See
`DEPENDENCIES.md` for the category totals and source references.

This summary accompanies the application for attribution and review. Upstream source
distributions retain their complete copyright/license files; distributors should run
the locked license audit in `BUILD.md` and include any additional notice required after
changing a dependency or dataset snapshot.

The SPDX 2.3 SBOM for Axvaris 0.4.3 is distributed as
`docs/sbom-0.4.3.spdx.json` with its SHA-256 manifest.
