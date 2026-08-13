# Sabicheiro 0.3.0 — Verified Destinations

This is a local Windows x86_64 test release focused on trustworthy endpoint identity,
offline GeoIP provenance and a faster investigation workflow.

## What to test

1. Start the portable executable or install the per-user NSIS package.
2. Choose **Demo data** to inspect a reproducible dense globe/feed without network use.
3. Switch between 3D and 2D; in 2D test zoom, pan, frame selection, expand/restore and
   Escape. Selection and filters should remain intact.
4. Inspect a connection and verify Evidence, Identity, Endpoint, Location, Traffic,
   History and Diagnostics sections.
5. Open Destinations and test country/ASN navigation; open History to save an
   investigation and compare observed versus currently derived GeoIP data.
6. In Settings, switch English/Castellano and Comfortable/Compact. Run the capture
   self-test; it uses loopback sockets only and does not contact the Internet.
7. Optionally start live capture. If Windows denies ETW, IP Helper should remain active
   and Diagnostics should explain that per-flow counters are unavailable.

## Privacy and cost

- No API key, account, subscription or paid service is required.
- There is no telemetry, analytics, crash upload, cloud sync or hosted map.
- Demo and self-test are local. Normal capture stores metadata, never payloads.
- GeoIP uses a local MMDB. The optional update action downloads a complete free DB-IP
  Lite dataset over HTTPS only after a click; it never sends observed IPs.
- System reverse DNS is opt-in and off by default.

## Package contents

- `Sabicheiro-0.3.0-windows-x64-portable.exe`
- `Sabicheiro-0.3.0-windows-x64-setup.exe`
- `SHA256SUMS.txt`
- these release notes and third-party notices

The binaries are unsigned local test artifacts. Verify their SHA-256 values before
running them. Windows SmartScreen may identify unsigned software as unrecognized.

## Known host-dependent limitations

- ETW/Pktmon access is controlled by Windows policy/elevation. The application falls
  back without fabricating evidence.
- Linux AppImage/deb artifacts are not included in this Windows handoff; build and test
  them natively using `BUILD.md`.
- Packet recordings can contain sensitive payload and are created only after a separate
  explicit action.
