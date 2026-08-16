# Sabicheiro 0.4.1 — Spain Focus

Validation release built on 2026-08-14. This maintenance release is ready for local
Windows testing and carries complete offline documentation for Windows and Linux. It
remains a validation candidate, rather than a stable cross-platform release, while the
native Linux packaging, elevated event-capture, eight-hour soak and OS screen-reader
gates listed below remain open.

## Highlights

- The 2D map and 3D globe now open centred on Spain. Reset restores the same Spain
  view instead of returning to a generic world origin.
- Exposure validates every listener snapshot received from the native bridge before it
  reaches React, rejects malformed roots, skips malformed records with a visible
  warning, deduplicates rows and compares snapshots in linear time.
- Exposure refreshes are sequenced and cancellable, so late IPC results cannot replace
  newer state. Loading, empty, partial-data and native-error states are explicit.
- An isolated recovery boundary prevents an unexpected Exposure rendering failure from
  crashing the rest of the application and offers an in-place reload action.
- The new self-contained `docs.html` follows Sabicheiro's visual language, works
  offline, is responsive and printable, and includes keyboard-friendly navigation,
  search and copyable commands.
- Documentation covers Windows and Linux installation, capture capabilities,
  permissions, features, privacy, exports, limitations, troubleshooting, development
  builds and the precise boundary between implemented and still-unverified behaviour.

## Windows package evidence

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `Sabicheiro.exe` | 20,589,568 | `1608974b36a40428f471e6aeb1d26c0fb026258870b95f61744840244f6a2812` |
| `Sabicheiro_0.4.1_x64-setup.exe` | 8,689,212 | `ab9b084ad86f14575d192d9733f9bc40936a60c2bc5a722a5e123665b2709a19` |
| `sbom-0.4.1.spdx.json` | 381,987 | `a47947b5c494617ad642068736d46a0122c981e8b363ea211fc41e5af3bacf9c` |
| `docs.html` | 81,340 | `c07eb5b05f57d0572a63b6ec97bb21ded6adc7ee36e34a2bdfdb2f803bf93591` |

The portable executable stayed alive for the eight-second normal-user smoke. The NSIS
package then completed a silent per-user installation, reported version 0.4.1, launched
for eight seconds and removed its program directory, uninstall registry record, desktop
shortcut and Start-menu shortcut through its own uninstaller. Existing local
application data remained intact, as intended.

## Automated validation

- Strict TypeScript, Spanish translation-key audit and Vite production build pass.
- Rust formatting and strict Clippy pass for the complete workspace.
- Rust tests pass: 125 passed, zero failed and one deliberately ignored live ETW test.
- The Linux capture crate cross-checks for `x86_64-unknown-linux-gnu`.
- Locked dependency licenses pass and the SPDX 2.3 SBOM contains 562 packages.
- The 100,000-row benchmark passes at 26.9164 ms filter p95; the bounded 30-second
  soak passes at 12.8264 ms cycle p95 with 122,880 bytes of memory growth.
- HTML structure, anchors, local-resource policy and inline JavaScript syntax pass.

## Known validation boundaries

- Windows IP Helper is the normal-user baseline. Live ETW evidence still needs an
  administrator-approved trace context; the current host returns `ACCESS_DENIED`.
- Linux `INET_DIAG` source cross-checks, but a production eBPF event stream is not
  shipped in this candidate. Native Ubuntu 22.04/24.04 and Debian 12 AppImage/deb
  build, install, update and removal remain required.
- The 30-second soak is not a substitute for the mandatory eight-hour desktop run with
  capture storms, sleep/resume, network changes and battery profiling.
- Narrator/NVDA, native HiDPI visual coverage and the final accessibility matrix still
  require the corresponding desktop environment and assistive technology.
- Exposure describes observed listeners and firewall-query status; it never claims
  that a wildcard listener is reachable from the Internet without route/firewall proof.

See `docs/VALIDATION.md` and `NEXT_TODOS.md` for the complete evidence ledger and the
remaining acceptance work.
