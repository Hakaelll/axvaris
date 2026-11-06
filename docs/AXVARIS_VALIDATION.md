# Axvaris implementation evidence

## Evidence and workspace corrections, 2026-11-05

- `pnpm check`, `pnpm lint` and `cargo fmt --all -- --check` passed. The complete Rust run passed 164 tests; one elevated ETW test remains explicitly ignored.
- Export regressions cover 105,001 rows in CSV and JSON, an older process beyond the former 100,000-row cap, per-page redaction, and preservation of an existing destination after a later-page error.
- Notification regressions cover simultaneous connections/profiles, legacy record identities, snooze retention, local quiet-hour boundaries and cooldowns.
- Client tests cover requested investigation windows, frozen metadata autosaves, unloaded pin scopes, rejection of stale save responses, and keyboard control guards. They test helpers and server-rendered markup, rather than browser interaction.
- An independent read-only history/export test succeeds while the capture-state mutex is held. The emergency in-memory storage path retains its existing connection. No prolonged capture-performance claim follows from these checks.
- The final independent code review found no remaining blocking issue after investigation persistence corrections. Native Windows checks ran outside the managed sandbox because its OS socket and pipe access was restricted.
- Fresh screenshots, before/after layout comparison, and computed text scaling at 200% were not verified. CUA access to the local Helium browser was not approved. Earlier screenshots do not establish the appearance of this revision.

## Baseline

- Source: `7b0539a`, Windows x86_64, 12 October 2026.
- Existing frontend checks passed, including 1,026 translated keys, UI behavior,
  TypeScript and the production Vite build.
- Existing native capture tests: 49 passed, one live ETW test explicitly ignored.
- Production entry JavaScript: 325.07 kB; deferred globe: 539.93 kB.
- Repository renamed to `Hakaelll/axvaris`; storage identity remains
  `io.sabicheiro.desktop` for upgrade compatibility.

## Capture corrections

- Added native process creation-time checks, immediate refresh for new/reused PIDs,
  cache invalidation, and event-time attribution guards.
- ETW records retain their native timestamps instead of receiving their drain time.
- Unknown process start times no longer produce a supposedly stable instance ID.
- Stable process/flow identities survive the approximate correlation time window.
- Closed tracks are removed; live deduplication caches are bounded to 65,536 records.
  Records beyond that live window are rejected with an explicit gap. The raw journal
  remains the source for historical replay.
- Native capture tests after changes: 54 passed, one live ETW test ignored.

## Outstanding native evidence

## Identity and external DNS stage

- Custom outlined serif wordmark and matched X icon; removed the extra crossing line after visual review. Local Geist fonts and recoverable storage identity are packaged.
- Frontend TypeScript, UI behavior and translation checks passed: 1,056 used translation keys without orphans. Full repository, frontend, dependency and locked license audits passed.
- Native workspace tests passed (152 tests, one explicitly ignored ETW test).
- External DNS tests cover CSV/JSON normalization, invalid fields/timezones, deduplication, cancellation rollback, pagination and readable schema-seven migration backups. Imported queries also follow local history removal and time-based retention.
- Windows executable and NSIS installer generation succeeded. This establishes package generation, not install/upgrade/uninstall acceptance.
- Fresh visual review scored all six requested corrections resolved: metadata contrast, distinct home marker, softer panel geometry, accessible session heading, current design documentation and responsive captures. It did not certify native performance.
- Native IP Helper fixture measured 10/18 controlled flows, zero incorrect process attributions and zero false positives. Requested polling interval: 10 ms. Short-flow recall was 33.3%; longer-flow recall 66.7%. Polling cannot observe remote UDP peers and does not meet event-capture release targets. See `axvaris-windows-polling.json` for the scoped measurements.
- The live ETW fixture was attempted and returned Windows access denied. IP Helper fallback remains available; elevated ETW accuracy is still unverified.

## Linux event capture

- Aya 0.14 loads embedded TCP tracepoint and UDP cgroup programs through a separate
  fixed-path helper. INET_DIAG remains available when the helper cannot attach.
- Docker Desktop's WSL2 kernel detected all 18 controlled flows, including brief
  IPv4/IPv6 TCP and UDP activity. There were no false positives or incorrect process
  attributions. Twelve owners remained unknown for passive TCP and UDP.
- An unprivileged fixture with a capability-enabled helper reproduced the result.
  The helper termination test confirmed zero effective/permitted/inheritable
  capabilities after attach and `no_new_privs=1`. This ran in a disposable privileged
  container, not a production-host security or installation audit.
- TCP close latency p95 was 10.38 ms in that fixture. UDP close uses a 30-second
  inactivity timeout. Receiving UDP conversations were outside the fixture's truth
  set. These results cannot establish general capture completeness or UDP attribution.
- Linux desktop compilation passed with GTK/WebKit dependencies. Linux capture
  tests passed 57 cases with one privileged helper test excluded from the ordinary
  run and then executed separately. Tests ran sequentially because concurrent process spawning can briefly expose
  inherited descriptors and correctly make ownership ambiguous.
- See [event measurements](axvaris-linux-events.json) and
  [polling measurements](axvaris-linux-polling.json). Raw reports remain under
  ignored `target/` paths because they contain local process evidence.
- Shared capture regressions also passed on Windows: 58 capture tests, with the
  live ETW test still explicitly ignored. All 17 desktop command tests passed.
  Repository, dependency, license, translation, TypeScript and frontend checks passed.

## Reference-guided interface update — 2026-10-14

The interface follows the 13 selected screens in
`design-references/axvaris-2026-10-13/`: a straight divided sidebar, warm opaque
evidence panels, local device groups, process-to-listener diagrams, retained
snapshot comparisons and an evidence inspector. Packaged NASA relief supplies
the geographic backdrop; plotted markers still require known coordinates.
Floating map controls and the command palette have an opaque glass fallback.

`pnpm --dir apps/desktop check` passed translation, UI behavior, TypeScript and
production build checks. `node scripts/audit-frontend.mjs` and `git diff --check`
passed. The reusable `scripts/verify-reference-ui.cjs` produced 20 browser
captures covering all selected reference screens, opaque commands and compact
layouts at 720×600 and 390×844, without runtime errors or root overflow.
It also checked keyboard focus, local device selection, distinct local listener
ports, retained snapshot selection, current settings sections and readable
compact summary values. Fixtures are explicitly browser demonstration data.
Native capture and installer acceptance were outside this interface update.
The independent final finish review inspected all 13 selected references and all
20 final captures, confirmed current design persistence and returned `ship`
without remaining material fixes. The historical review packet is available in
Git history. New runs write generated captures and their verification report to
ignored `target/ui-review/` by default.

## Remaining acceptance work

Elevated live ETW, Linux desktop package installation, upgrade/uninstall lifecycle,
screen-reader/HiDPI checks, suspension/VPN recovery and an eight-hour desktop soak
still require execution. Compilation or browser checks do not substitute for that
evidence. The controlled fixtures do not exercise PID reuse under sustained load.

