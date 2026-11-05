<picture>
  <source media="(prefers-color-scheme: dark)" srcset="apps/desktop/public/brand/axvaris-wordmark-light.svg">
  <img alt="Axvaris" src="apps/desktop/public/brand/axvaris-wordmark-dark.svg" width="340">
</picture>

**Network observability. Local evidence.**

Axvaris is a desktop application for investigating the network activity of your computer. Follow connections, inspect the processes behind them, and reconstruct activity from locally stored evidence. No account, subscription, telemetry service or paid API is required.

Ordinary observation, history, and GeoIP lookups run locally. Optional dataset downloads and opt-in reverse DNS use the network.

[Architecture](ARCHITECTURE.md) · [Privacy](PRIVACY.md) · [Validation](docs/AXVARIS_VALIDATION.md) · [License](LICENSE)

## Download v0.4.3

| Platform | Download |
|---|---|
| Windows 10/11 x64 | [Installer](https://github.com/Hakaelll/axvaris/releases/latest/download/Axvaris-Setup.exe) · [Portable executable](https://github.com/Hakaelll/axvaris/releases/latest/download/Axvaris.exe) |
| Linux x64 | [AppImage](https://github.com/Hakaelll/axvaris/releases/latest/download/Axvaris.AppImage) · [Debian package](https://github.com/Hakaelll/axvaris/releases/latest/download/Axvaris.deb) |

See the [release notes](https://github.com/Hakaelll/axvaris/releases/latest), [SHA-256 checksums](https://github.com/Hakaelll/axvaris/releases/latest/download/SHA256SUMS.txt), and [SPDX SBOM](https://github.com/Hakaelll/axvaris/releases/latest/download/sbom-0.4.3.spdx.json). Windows requires WebView2. Linux requires WebKitGTK 4.1 and the runtime libraries described in the [Linux guide](linux.md).

Version 0.4.3 corrects CSV and JSON exports beyond 100,000 rows, preserves requested and frozen investigation windows during autosave, and fixes notification snooze and local quiet-hour handling. It also corrects keyboard controls and text scaling, adds a compact Live layout, and refines the investigation views.

![Axvaris live workspace](docs/images/axvaris-live.png)

*Illustrative screenshot from an earlier revision. It does not show every v0.4.3 change.*

## Investigation workspaces

- **Today:** activity summaries, changes and evidence drill-downs.
- **Live:** connections, local peers and listeners, with a synchronized map and contextual inspector.
- **Investigate:** paginated history, process instances, timelines and read-only `.sabi` investigation bundles.
- **Exposure:** listening sockets, interfaces and explainable local rules.
- **Data & Health:** capture capabilities, diagnostics, retention, privacy and external DNS imports.

Process attribution includes PID and process creation time when available. Unresolved attribution remains explicit. Provider clocks, provenance and loss reports accompany evidence; geographical coordinates are never invented.

Import CSV or JSON resolver logs to follow observed client addresses and queried domains over time. Imports preserve their source and remain distinct from local capture. A DNS query proves neither a connection nor the identity of a remote process. Start with the [CSV](apps/desktop/public/templates/dns-queries.csv) or [JSON](apps/desktop/public/templates/dns-queries.json) template.

## Capture support

| Platform | Baseline | Event capture | Limits |
|---|---|---|---|
| Windows | IP Helper socket snapshots | Optional ETW network and DNS events | Elevated event capture needs permission; polling can miss short-lived sockets. |
| Linux | INET_DIAG TCP/UDP, IPv4/IPv6 snapshots | Optional local Aya helper, installed separately | Snapshot ownership depends on process visibility. Helper events can attribute active TCP opens; passive TCP and UDP owners remain unknown. |

Axvaris observes the local machine. External devices are represented through imported DNS evidence; this release does not install remote agents. Optional local GeoIP databases provide approximate destination context. Reverse DNS is opt-in.

Linux packages use INET_DIAG by default and do not bundle the Aya helper. See the [Linux guide](linux.md) for helper installation, permissions, kernel requirements, and capture limits. Controlled kernel fixtures establish scoped capture behavior, not desktop package installation acceptance.

## Development

Run the reproducible browser preview:

```sh
pnpm install --frozen-lockfile
pnpm dev
```

The preview uses deterministic fixtures. Native capture and persistent DNS import require the desktop application:

```sh
pnpm --dir apps/desktop tauri dev
```

Building requires Rust 1.87 or later, Node.js 24, the pinned pnpm toolchain, and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
pnpm check
pnpm lint
pnpm --dir apps/desktop tauri build
```

See [BUILD.md](BUILD.md) for native package commands, manual GitHub Actions workflows, and release checks.

## Data and upgrades

Axvaris is the new public identity of Sabicheiro. The technical application identifier, existing data directory and `.sabi` format remain compatible. SQLite upgrades are transactional and retain a recoverable pre-migration backup. Review export redaction controls before sharing sensitive evidence.

English is the default; Spanish and saved language preferences remain available. Geist and Geist Mono are packaged locally. The interface supports keyboard navigation, visible focus, reduced motion and text scaling. See the [brand assets](docs/AXVARIS_BRAND.md).

## Engineering and validation

React and TypeScript provide the interface; Tauri and Rust coordinate native capture, evidence correlation and SQLite storage. Capture and rendering run independently. Histories are paginated, dense lists are virtualized, and live correlation caches are bounded.

The [validation record](docs/AXVARIS_VALIDATION.md) distinguishes automated checks from native acceptance work. Export regressions cover 105,001 rows; controlled Linux kernel fixtures have passed in Docker. Elevated Windows capture, installation lifecycle, assistive technologies, current-revision visual checks, and prolonged resource measurements still require their own evidence. A successful build alone does not establish those guarantees.

Contributions should include a reproducible problem, the smallest complete fix and relevant verification. See [DEPENDENCIES.md](DEPENDENCIES.md) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Licensed under [MIT](LICENSE).
