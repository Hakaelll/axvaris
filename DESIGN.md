# Sabicheiro Design System

## Direction: Midnight Precision Instrument

Sabicheiro is quiet, dense and exact. It is not a game HUD, cyberpunk scene or generic
SaaS dashboard. The globe is the primary canvas; controls support the evidence instead
of competing with it.

## Tokens

| Token | Value | Use |
| --- | --- | --- |
| `--void` | `#08090a` | canvas/ocean |
| `--carbon` | `#0f1011` | shell surfaces |
| `--obsidian` | `#161718` | raised panels/land |
| `--graphite` | `#23252a` | primary hairlines |
| `--smoke` | `#383b3f` | secondary borders |
| `--ash` | `#62666d` | subdued labels |
| `--fog` | `#8a8f98` | secondary metadata |
| `--mist` | `#d0d6e0` | primary copy |
| `--paper` | `#ffffff` | exceptional maximum contrast |
| `--acid-lime` | `#e4f222` | selected/active/primary action only |

Spacing uses a 4 px base (`4, 8, 12, 16, 20, 24, 32, 40`). Panel radii are 12 px,
buttons/inputs 6 px, badges 4 px and status pills fully rounded. Borders are 0.5–1 px;
surface value, not shadow, expresses depth.

## Typography

The UI requests Inter Variable and falls back to Inter/system UI. Weight 400 is normal;
510 and 590 carry hierarchy. Weight 700+ is avoided. IPs, ports, PIDs, protocols,
timestamps and rates use `ui-monospace`, with JetBrains Mono/IBM Plex Mono as optional
free local choices. Type remains readable at compact 11–14 px metadata sizes.

## Layout

- Top bar: 52 px, brand/navigation left, search/status/actions right.
- Main globe/map canvas: 65–75% of usable width.
- Live feed: 25–35%, minimum 300 px, a semantic list rather than a table.
- Inspector: contextual overlay/drawer that does not permanently shrink the globe.
- Bottom metrics: 44 px single strip; never giant KPI cards.
- Supported minimum: 1280×720; tuned at 1920×1080 and 2560×1440.

Below 1180 px the feed narrows and metadata wraps; it does not disappear. Below 900 px
the inspector becomes a full-height overlay. This is responsive desktop UI, not mobile.

## Globe and map

Ocean uses `--void`, land `--obsidian`, borders `--smoke`. Unselected traffic uses one
or two desaturated semantic shades. Only the selected arc and focused endpoint use
acid lime. Closed routes fade through lower opacity during the core's eight-second
visual TTL. Route grouping becomes more aggressive with connection count and globe
distance; grouped marker size communicates relative count without labels covering the
map. Decorative route particles remain a post-0.1 refinement.

The same filtered model drives the 2D equirectangular map. Unknown coordinates are not
drawn; their connections remain visible in the feed. The map uses local 50m country
geometry and is loaded only when selected; the globe keeps a lighter local geometry.

## Components

- `TopBar`: product mark, primary views, compact search, capture/record/settings.
- `LiveFeed`: accessible buttons grouped by process + remote endpoint + protocol
  within a configurable time window; `content-visibility` bounds off-screen layout.
- `Inspector`: process, remote, geo, network, traffic, timing, ASN and history sections.
- `MetricStrip`: active/down/up/countries/processes/packets.
- `CommandPalette`: searchable actions opened with Ctrl+K.
- `QueryBar`: search plus typed filter tokens and concise error text.
- `ProcessList` / `DestinationList`: compact ranked rows, not card grids.
- `Timeline`: traffic/new-connection density with draggable time selection.
- `Settings`: a professional section list, no dashboard grid.

## Interaction rules

Feed selection highlights and focuses the arc, opens the inspector and preserves
keyboard focus. Arc/marker selection scrolls the matching feed item into view. Country
selection filters context without forcing navigation. Smart focus lasts at most
220 ms and can be disabled.

Shortcuts: Ctrl+K palette, Ctrl+F search, Space pause view, G globe, M map, R recording,
Esc inspector. Visible focus outlines use acid lime with sufficient offset. Lists carry
screen-reader labels and all icon-only buttons have text alternatives/tooltips.

English and Castellano share the same layout and component hierarchy. The welcome
screen and Display settings expose the language selector, and the saved setting also
updates the document language for assistive technology.

## Motion and states

Default transitions are 150–220 ms, eased without bounce/overshoot. Reduced-motion
disables arc drawing, camera tweening and decorative transitions. Pause View freezes
visual changes only; capture/history continue and the UI reports queued event count.

Errors are explicit: title, reason and recovery actions. Empty capture keeps the globe
visible with “Waiting for network activity”. Neutral anomaly language explains observed
deviation and never claims malware or produces an opaque risk score.
