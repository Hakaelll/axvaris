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

Elevated live ETW, native Linux event capture and packaging, screen-reader/HiDPI
checks, and an eight-hour desktop soak require execution in their respective
environments. Compilation or browser checks do not substitute for that evidence.
