# Build and package Axvaris

## Toolchains

- Rust stable, version 1.87 or later, with the native target: `x86_64-pc-windows-msvc` or `x86_64-unknown-linux-gnu`.
- Node.js 24 and pnpm 11.16.0. The pnpm version is pinned in `package.json`.
- The [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/).

No paid compiler, signing service, or CI service is required. Unsigned local packages are buildable. Optional code signing is a distributor concern.

## Prepare and check the source

```sh
pnpm install --frozen-lockfile
cargo fetch --locked
pnpm check
pnpm lint
cargo fmt --all -- --check
```

`pnpm check` runs repository, frontend, dependency, license, translation, TypeScript, production frontend, and Rust test checks. `pnpm lint` runs TypeScript and Clippy checks. Live capture tests that need elevated access are separate from the ordinary test run.

For the deterministic browser preview:

```sh
pnpm dev
```

For the desktop shell with native capture and persistence:

```sh
pnpm tauri dev
```

## Build Windows packages

On Windows 10/11 x64, install the Visual Studio Build Tools workload **Desktop development with C++** and WebView2. Build the NSIS installer:

```sh
pnpm tauri build --bundles nsis
```

The installer is under `target/release/bundle/nsis/`. The public release names are `Axvaris-Setup.exe` and `Axvaris.exe`. Copy the portable executable from the same Tauri production build as the installer. The portable executable also requires WebView2.

NSIS installs per user and creates Start menu and uninstall entries. MSI is optional and may require the Windows VBSCRIPT feature.

Use Tauri to produce distributable executables. A plain `cargo build --release` is useful for native diagnostics but may set Tauri's `cfg(dev)` boundary and omit `frontendDist`. Do not use its executable as the release artifact or the packaging-size baseline. Verify that the portable executable contains the production asset keys from `apps/desktop/dist` and that NSIS uses that same executable.

## Build Linux packages

Build on Linux x64. Tauri desktop packages are native builds and are not cross-compiled from Windows. See [linux.md](linux.md) for the complete Linux guide and optional helper setup.

On Debian or Ubuntu with WebKitGTK 4.1 available:

```sh
sudo apt update
sudo apt install -y build-essential pkg-config clang libbpf-dev iproute2 \
	libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
	libayatana-appindicator3-dev librsvg2-dev patchelf file
pnpm install --frozen-lockfile
pnpm tauri build --bundles appimage,deb
```

Outputs are under `target/release/bundle/appimage/` and `target/release/bundle/deb/`. The public release names are `Axvaris.AppImage` and `Axvaris.deb`. Other distributions need the corresponding Tauri prerequisites.

INET_DIAG is the default capture provider and runs without elevating the desktop. The optional Aya helper is built and installed separately at `/usr/libexec/axvaris-capture-helper`; desktop packages do not bundle it. The [Linux guide](linux.md) describes kernel requirements, capabilities, fallback behavior, and attribution limits. Passing helper fixtures does not establish package installation or production-host security acceptance.

Socket observation does not require dumpcap. For explicit packet recording, install the distribution's Wireshark or dumpcap package and configure its capture group or minimal capabilities according to distribution policy. Do not run the interface as root.

## GeoIP data

Axvaris runs without a GeoIP dataset and reports when the City database is missing. Use the map, connection inspector, or Settings to download DB-IP City Lite. ASN data is optional. Lookups use local databases. Private and loopback addresses have no public geographic location.

If a release includes a reviewed database snapshot, update its CC BY attribution and date. Never bundle Npcap.

## Audit licenses and generate the SBOM

Before a release, run:

```sh
cargo metadata --locked --format-version 1
pnpm licenses list --prod --long
pnpm licenses list --dev --long
node scripts/audit-licenses.mjs
node scripts/generate-sbom.mjs 0.4.3 docs/sbom-0.4.3.spdx.json
```

Reject paid, commercial-only, and network-service dependencies. Update `DEPENDENCIES.md` and required license notices. The license audit reads locked local Cargo metadata and the pnpm store without registry or API calls.

The SBOM generator reads locked offline Cargo metadata and installed packages from the frozen pnpm store. It emits deterministic SPDX 2.3 JSON. `SOURCE_DATE_EPOCH` can override the release timestamp. Regenerate the checksum after changing the SBOM.

## Run manual GitHub Actions workflows

Both workflows use `workflow_dispatch` and run only when manually requested:

- `.github/workflows/ci.yml` validates Rust and TypeScript on Windows and Ubuntu.
- `.github/workflows/packages.yml` builds NSIS on Windows and AppImage and Debian packages on Ubuntu. It normalizes public asset names and uploads workflow artifacts.

Workflow artifacts are build outputs. Publishing a GitHub release is a separate step. Standard GitHub-hosted runners are free for public repositories; artifact storage has separate limits. This package workflow retains transient build artifacts for one day. See [GitHub Actions billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions). Local builds do not require Actions.

## Verify and publish v0.4.3

Keep generated bundles under ignored `target/` paths. Tauri schemas under `apps/desktop/src-tauri/gen/` are also generated and must not be committed.

Publish these assets together:

- `Axvaris-Setup.exe`
- `Axvaris.exe`
- `Axvaris.AppImage`
- `Axvaris.deb`
- `sbom-0.4.3.spdx.json`
- `SHA256SUMS.txt`
- Release notes that identify the source revision and validation limits.

Record artifact sizes and SHA-256 checksums after the final build. Launch each native executable as a normal user, exercise all five workspaces, and verify install, upgrade, uninstall, and the chosen data-retention behavior on the target platform.

Use [docs/AXVARIS_VALIDATION.md](docs/AXVARIS_VALIDATION.md) as the current validation record. Controlled Linux kernel fixtures have passed, but elevated live Windows ETW, Linux desktop package installation, lifecycle acceptance, screen-reader and HiDPI checks, suspension and VPN recovery, and an eight-hour desktop soak still need evidence. Builds, automated tests, and browser screenshots do not replace those checks.
