# Axvaris implementation evidence

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

Elevated live ETW, native Linux event capture and packaging, screen-reader/HiDPI
checks, and an eight-hour desktop soak require execution in their respective
environments. Compilation or browser checks do not substitute for that evidence.

