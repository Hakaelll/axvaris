# Guide implementation evidence — 2026-10-14

Authority: the 13 selected PNGs and implementation criteria in
`design-references/axvaris-2026-10-13/README.md`. Original screenshots establish
product facts; rejected variants do not establish the selected direction.

| Requirement | Current artifact and evidence |
| --- | --- |
| Original wordmark, X, Geist and Geist Mono | Existing packaged brand/font assets retained; visible in all desktop captures. |
| Five navigation areas | App navigation retained; browser verification visits each area. |
| Object-led content and adjacent actions | Today observations/comparisons, device inspector, service diagram, retained history inspector and Exposure selection in captures 01–10. |
| Appropriate geographic/device/snapshot/timeline structure | Captures 03–10; MapView and GlobeScene use packaged relief plus atlas overlays and existing known-coordinate markers; LocalPeers groups interfaces; HistoryView separates workspace, timeline and sessions. |
| Neighbor cache distinguished from traffic | LocalPeers retains evidence labels and selects only attributed connection IDs; component behavior checks cover device grouping. No device-to-device edges are drawn. |
| Zero, absent counters and unknown attribution | Existing typed data retained; unavailable counters remain absent in HistoryInspector, ConnectionInspector and LiveFeed. Behavior tests cover absent counters. No risk/malware scores or invented locations added. |
| Explanations concise or consultable | Today row detail is consultable; long retained evidence is in an inspector; captures 01, 02, 07–10. |
| Warm glass only floating controls/commands, opaque main evidence | Styles and captures 03, 05, 13; enhanced contrast, reduced transparency and unsupported-filter fallbacks preserve opaque surfaces. Browser test asserts no backdrop filter in the opaque palette. |
| Visible labels, keyboard focus, selection and textual states | QueryBuilder, ExternalDns, CommandPalette and selected evidence rows; browser test covers keyboard containment and navigation, device/listener/snapshot selection, current settings sections and direct scrolling. |
| Reference palette without a new blue/cyan/purple identity | Warm source tokens and final screenshots; DESIGN.md and sidecar persist the implemented system. |
| All selected screens represented | verification.json lists 13 matching numbered captures, opaque command palette and six compact captures, 20 total. |
| Layout remains usable within supported window | Native minimum remains 720×600. Browser tests also check 390×844 fallback, full summary labels/values and document overflow; captures are explicit demonstration data. |

Validation: desktop translation audit (1,078 keys, no orphans), UI behavior,
TypeScript and production build passed after the final settings-scroll correction.
Frontend audit passed (46 reachable modules, 63 documented IPC commands).
The final browser verification completed with exit 0 and no runtime errors.
The independent finish reviewer inspected all 13 selected reference images and
all 20 final captures, checked current design persistence and returned `ship`
with no remaining material fixes; see `finish-review.md`.

These checks establish the interface update and browser behavior. Native capture,
installer lifecycle, screen-reader acceptance and long-duration resource behavior
were not tested in this task.
