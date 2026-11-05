# Axvaris 0.4.3

Axvaris 0.4.3 brings the Axvaris identity and investigation workspaces to a packaged release. It also corrects investigation persistence, complete exports and local notifications.

## Changes

- Today distinguishes observations with secondary evidence and observation times. Generic explanations appear once per category.
- Live has compact connection rows and expandable technical details. Investigate folds its editor while preserving drafts. Data & Health uses matching navigation and content order.
- Saved investigations retain the requested interval beyond the first 500 loaded records. Metadata autosaves preserve the frozen end and unloaded pin scopes. Stale save responses cannot reactivate another workspace.
- CSV and JSON exports cover the complete matching selection, including processes beyond the former 100,000-record boundary. Exports apply redaction per page and publish the destination only after success.
- File-backed history reads and metadata exports run outside the capture-state mutex on a read-only SQLite connection.
- Simultaneous notification identities distinguish connections and profiles. Snoozes retain their grouped record, and frontend alerts respect local quiet hours.
- Global keyboard shortcuts respect focused controls and dialogs. Text scaling applies to effective font sizes.
- External DNS imports, local evidence provenance, approximate GeoIP context and read-only `.sabi` investigations remain available.

## Downloads and compatibility

Windows x64 packages include a per-user NSIS installer, `Axvaris-Setup.exe`, and the portable `Axvaris.exe`. Windows requires WebView2. Linux x64 packages include `Axvaris.AppImage` and `Axvaris.deb`.

The technical identifier remains `io.sabicheiro.desktop`. Existing data locations and `.sabi` files remain compatible. SQLite upgrades retain recoverable backups. The packages do not require an account, subscription, telemetry endpoint or paid API.

The optional Linux Aya helper is installed separately as described in [linux.md](../linux.md). Desktop packages do not grant capture permissions or enable it. INET_DIAG remains the normal fallback. Elevated Windows ETW also requires separate permission.

Check downloads against the attached `SHA256SUMS.txt`. The SPDX inventory is `sbom-0.4.3.spdx.json`. Packages are unsigned.

## Validation boundaries

The implementation passed local frontend, translation, dependency/license, format and Clippy checks, with 164 Rust tests passing and one elevated ETW test explicitly ignored. Release packaging and validation run separately on native Windows and Linux GitHub runners for the tagged source.

Package generation does not establish installation, upgrade/uninstall behavior, elevated capture accuracy, screen-reader/HiDPI support, fresh visual layout checks, suspension/VPN recovery or an eight-hour soak. Those acceptance checks remain open in the [validation record](AXVARIS_VALIDATION.md).
