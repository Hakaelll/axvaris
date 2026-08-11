# Build and Packaging

## Toolchains

- Rust stable with the native target (`x86_64-pc-windows-msvc` or
  `x86_64-unknown-linux-gnu`).
- Node.js 22+ and pnpm 11.
- Tauri 2 system prerequisites.

No paid compiler, signing service or CI service is required. Unsigned local packages
are fully buildable; optional code signing is a distributor concern and not required to
run Sabicheiro.

## Bootstrap

```text
pnpm install --frozen-lockfile
cargo fetch --locked
pnpm check
```

Development:

```text
pnpm dev                 # deterministic browser preview
pnpm tauri dev           # real desktop shell/capture
```

Quality gates:

```text
pnpm build
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
rustup target add x86_64-unknown-linux-gnu
cargo check -p sabicheiro-capture --target x86_64-unknown-linux-gnu --locked
```

## Windows 10/11 x86_64

Install the free Visual Studio Build Tools workload **Desktop development with C++**;
WebView2 is normally already present. Build NSIS only:

```text
pnpm tauri build --bundles nsis
```

The output is under `target/release/bundle/nsis/`. Tauri names the setup executable
from the configured product; the release workflow normalizes it to
`Sabicheiro-Setup.exe`. MSI is optional and may require the Windows VBSCRIPT feature.
The NSIS configuration installs per-user and supplies Start-menu/uninstall entries.
The local `0.2.0` handoff also copies the installer, portable executable, checksums and
release notes into `release/Sabicheiro-0.2.0-windows-x64/`.

## Debian/Ubuntu x86_64

Install Tauri's system dependencies:

```text
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
pnpm install --frozen-lockfile
pnpm tauri build --bundles appimage,deb
```

Outputs are under `target/release/bundle/appimage/` and `target/release/bundle/deb/`.
Fedora/Arch package prerequisites follow the official Tauri prerequisites page.
Socket observation does not require dumpcap. To enable explicit packet recording,
install the distro Wireshark/dumpcap package and configure its capture group or minimal
capabilities according to distro policy; do not run the whole UI as root.

Tauri desktop bundles are native builds and are not cross-compiled from Windows. The
included GitHub Actions matrix performs the Linux package build on a Linux runner;
the same commands remain runnable locally without CI.

## GeoIP data

The binary runs without a dataset and displays `Unknown location`. For complete offline
geo rendering, install the current DB-IP City Lite MMDB through Settings or place the
licensed database in the platform data directory. Release bundles may include a reviewed
snapshot provided its CC BY attribution/date is updated.

## License audit

Before a release:

```text
cargo metadata --locked --format-version 1
pnpm licenses list --prod --long
pnpm licenses list --dev --long
```

Reject paid/commercial-only/network-service dependencies, update `DEPENDENCIES.md`, and
ship required MIT/Apache/ISC/BSD/CC notices. Never bundle Npcap.

## CI

`.github/workflows/ci.yml` compiles/tests Rust and TypeScript on Windows and Ubuntu.
`.github/workflows/packages.yml` builds NSIS, AppImage and deb artifacts. Both workflows
use **only** `workflow_dispatch`: nothing consumes hosted-runner quota automatically.

GitHub-hosted Actions can become billable after an account/repository quota is
exhausted, so it is never a zero-cost runtime/build requirement. For a guaranteed €0
workflow, run these commands locally or dispatch them on a compatible self-hosted
runner. No workflow was triggered as part of the 2026-08-09 local validation.

## Release verification

After packaging, record artifact size and SHA-256, launch the native executable as a
normal user, exercise Demo and Live modes, and verify install/uninstall. Linux artifacts
must be tested on Linux; Tauri bundles are not meaningfully validated by merely
cross-compiling them from Windows. The current evidence is maintained in
`docs/VALIDATION.md`.
