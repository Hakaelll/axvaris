---
name: Axvaris
description: A desktop instrument for inspecting local network evidence.
colors:
  void: "#101010"
  carbon: "#151413"
  obsidian: "#1d1a18"
  graphite: "#3d3a39"
  smoke: "#4d4947"
  ash: "#a39b97"
  fog: "#b8b3b0"
  mist: "#eeeeee"
  paper: "#fff"
  selection-muted: "rgba(238, 238, 238, 0.08)"
  surface-interactive: "#24211f"
  signal-orange: "#ee6018"
  state-positive: "#a0ca92"
  connection-selected: "#34302c"
  divider: "rgba(238, 238, 238, 0.12)"
  globe-boundary: "#68706e"
typography:
  headline:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "32px"
    fontWeight: 560
    letterSpacing: "-0.025em"
  title:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "16px"
    fontWeight: 500
  body:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "14px"
    lineHeight: 1.6
  row-name:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "16px"
  label:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "12px"
  metric:
    fontFamily: '"Geist Mono", ui-monospace, "Cascadia Code", monospace'
    fontSize: "23px"
  mono:
    fontFamily: '"Geist Mono", ui-monospace, monospace'
rounded:
  badge: "4px"
  control: "10px"
  panel: "16px"
  capsule: "20px"
  capsule-item: "16px"
  protocol-tag: "6px"
  connection-row: "10px"
  globe-caption: "12px"
  query-chip: "14px"
  floating-control: "14px"
  command-palette: "18px"
spacing:
  compact: "6px"
  control: "12px"
  row: "16px"
  observation: "22px"
  section: "20px"
  workspace: "24px"
  page: "28px"
components:
  button-primary:
    backgroundColor: "{colors.mist}"
    textColor: "{colors.void}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "0 12px"
  button-primary-hover:
    backgroundColor: "#d8d5d2"
  button-secondary:
    backgroundColor: "#181a1b"
    textColor: "{colors.mist}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "0 12px"
  search-field:
    backgroundColor: "{colors.carbon}"
    textColor: "{colors.mist}"
    rounded: "{rounded.capsule}"
    height: "36px"
    padding: "0 12px"
  sidebar-item-active:
    backgroundColor: "#292522"
    textColor: "{colors.paper}"
    rounded: "{rounded.control}"
    padding: "10px"
  observation-list:
    backgroundColor: "{colors.carbon}"
    textColor: "{colors.fog}"
    rounded: "{rounded.panel}"
    padding: "0 24px"
  observation-row:
    textColor: "{colors.fog}"
    padding: "22px 0"
  evidence-action:
    textColor: "{colors.paper}"
    rounded: "{rounded.control}"
    padding: "6px 10px"
  session-panel:
    backgroundColor: "{colors.obsidian}"
    textColor: "{colors.mist}"
    rounded: "{rounded.panel}"
    padding: "22px 24px"
  notification-panel:
    backgroundColor: "{colors.carbon}"
    textColor: "{colors.fog}"
    rounded: "{rounded.panel}"
    padding: "18px 20px"
  connection-row-selected:
    backgroundColor: "{colors.connection-selected}"
    textColor: "{colors.mist}"
    rounded: "{rounded.connection-row}"
    padding: "12px 10px"
  live-feed:
    backgroundColor: "{colors.obsidian}"
    rounded: "{rounded.panel}"
  map-controls:
    backgroundColor: "rgba(29, 26, 24, 0.9)"
    rounded: "{rounded.floating-control}"
    padding: "4px"
  protocol-tag:
    backgroundColor: "{colors.selection-muted}"
    textColor: "{colors.mist}"
    typography: "{typography.mono}"
    rounded: "{rounded.protocol-tag}"
    padding: "2px 6px"
  globe-caption:
    backgroundColor: "{colors.obsidian}"
    rounded: "{rounded.globe-caption}"
    padding: "12px 16px"
  query-chip:
    backgroundColor: "#191b1c"
    textColor: "{colors.fog}"
    typography: "{typography.label}"
    rounded: "{rounded.query-chip}"
    padding: "5px 7px"
---

# Design System: Axvaris

## Overview

**Creative North Star: "The Network Observatory"**

Axvaris presents network evidence in a restrained desktop workspace. Factory and Gotham are the confirmed references for its serious observability character. The selected Axvaris guide of 2026-10-13 establishes the current composition and materials. A straight sidebar anchors warm opaque evidence panels with quiet enclosing edges. Bordered warm glass separates floating geographic and temporal controls and the command palette from the evidence behind them. The globe shares the workspace with the feed; evidence remains available when a destination cannot be plotted.

The serif AXVARIS wordmark supplies the identity. Interface text uses Geist and technical values use Geist Mono. Motion responds to an action or a data change. Continuous decorative particles and pulsing chrome have no role in this design.

**Key Characteristics:**

- Five persistent investigation areas in a left sidebar.
- Monochrome controls with limited color for evidence states.
- Full-width observations with titles before category and scope, beside snapshot comparisons.
- Distinct geographic, device, listener and retained-evidence workspaces.
- Larger values and row names above quieter metadata.
- Local brand and font assets, with no remote font dependency.

## Colors

Warm near-black neutrals carry the interface. The frontmatter records effective CSS tokens, including later stylesheet overrides.

### Primary

- Mist supplies primary text, selected controls and primary action fills.
- Signal orange identifies selected connection evidence, recording and warning or error states. It is not a general brand accent.
- Positive green marks live capture, positive evidence states and the home marker.

### Neutral

- Void is the application background and main canvas.
- Carbon separates the sidebar and continuous observation list from the canvas. The metric strip uses the void background.
- Obsidian is the raised panel tone. The CSS surface-raised token aliases this value.
- Graphite draws stronger edges. The CSS border-subtle token aliases this value. Divider supplies quieter separators inside observation and feed lists.
- Smoke provides stronger secondary edges.
- Fog is secondary copy. Ash is quieter supplementary text.
- Paper provides maximum contrast for short headings and active navigation.
- Active navigation uses a warm selected fill without a border. Connection selected fills the selected feed row.
- Globe boundary gives country outlines enough contrast against the dark globe.

**The Evidence Color Rule.** Use color to identify a capture or evidence state. Keep general navigation and actions monochrome.

## Typography

Geist and Geist Mono load from packaged variable WOFF2 files. Preserve the system fallbacks in the frontmatter. Technical values use tabular numerals. The serif wordmark and X are supplied SVG paths, not a third interface font.

Page headings use the headline token. Observation titles and connection row names use the title size, while supporting body copy uses the body token. Session names use 17px and session metrics use the metric token in Geist Mono. Comparison section headings use 22px at weight 550. Labels and compact controls use the label token. Globe caption labels use 12px and their values use 13px. Keep addresses, timestamps, ports and rates in mono without shrinking the primary process name. Do not promote hidden eyebrows or incidental uppercase export legends into a new heading role.

**The Identity Rule.** Use the supplied serif wordmark and its matching X. Do not add a crossing line or reconstruct the logo with a system font.

## Layout

Type sizes in this document are the 100% baseline. The text-scale preference multiplies effective typography through the inherited `--text-scale` factor, including headings, controls and expanded evidence. Compact Live rows have an 80px minimum height and 8px by 10px padding, while growing to fit enlarged text. Process, destination, protocol, state and rate remain visible; endpoint, location, duration, transfer totals and grouping window appear in expanded detail.

Today rows include their supplied secondary evidence and observation time. A generic category explanation appears once among visible rows. Investigate starts its editor folded under New investigation or Edit investigation and retains drafts while folded. Data & Health uses its global navigation title; its index and content share an order starting with Capture and ending with External DNS, preserving direct section navigation.

The desktop shell has a 208px sidebar, 64px top bar and 44px metric strip. The straight sidebar spans the full shell height, with square outer corners, no inset and a quiet right divider. Today, Live, Investigate, Exposure and Data & Health remain in that order. Search, capture state and actions sit in the top bar. The metric strip has a quiet top separator and no vertical cell rules. Contextual inspection does not require another permanent global navigation column.

The native window has a minimum size of 720 by 600. At 1180px and below, the sidebar becomes a 72px icon rail and the wordmark yields to the X. Names remain available through button labels and tooltips. The Today session strip reduces six metric columns to three; its sparkline is hidden at every width. At 900px and below, the session strip and paired evidence panels stack, observation actions move below their content, and the Live canvas stacks above the feed with a canvas row of at least 250px. Settings navigation wraps above its content. Narrow capture actions move into a compact menu. The 650px breakpoint supports browser fallback, verified at 390px; it does not change the native window target.

Today uses one content column with 28px page padding. Observations occupy full-width rows inside one continuous rounded list, with actions aligned beside the content. Changes can show previous and current snapshot values beside their title and evidence action. The enclosed notification inbox follows the list and its empty message has an 84px minimum area. Comparison and location coverage panels share a two-column row before stacking in the compact layout. The rule editor remains an explicit enclosed group.

The Live workspace has a 16px inset and gap around the canvas and rounded feed, reduced to 8px at the stacked breakpoint. Connection rows have a 112px minimum height and 12px by 10px padding. Keep primary values visible before reducing spacing or wrapping metadata. Allow long evidence tables and the bottom metric strip to scroll within their bounds.

## Elevation & Depth

Persistent evidence uses opaque warm backgrounds and one-pixel graphite enclosures, with quiet internal separators. Observation rows do not each have an enclosing border. Session strips, notification inboxes, feeds, comparisons, history and settings groups use edges to establish their bounds. Persistent groups have no ambient shadow. Floating geographic and temporal controls use a warm translucent fill, light edge, inset highlight and soft shadow. The command palette uses the same material with stronger blur and lift.

**The Panel Rule.** Keep evidence, tables and fields opaque. Use warm glass and shadows for floating controls and transient overlays, with an opaque fallback under enhanced contrast, reduced transparency or missing backdrop-filter support.

## Shapes

Buttons, ordinary fields and sidebar items use the control radius. Search keeps its capsule shape. Floating geographic control groups use the floating-control radius with control-radius items; the command palette uses its own larger radius. Continuous observation lists, session strips, Live feeds, comparison panels, inspectors and settings sections use the panel radius. The sidebar has square outer corners. Individual observation rows have square corners inside their group. Feed rows use their own modest corner radius and globe captions use the caption radius. Protocol tags use small corners and query chips retain their existing pill shape.

The monochrome X has the wordmark's serif terminals and stroke contrast. Use light assets against the dark shell and dark assets against light backgrounds. Preserve the supplied geometry and keep the mark free of glow or additional crossing strokes.

## Components

### Buttons

General primary actions have a light fill and dark text. Hover uses a quieter warm light fill. Secondary actions have a dark fill and visible border. Base controls use a 32px minimum height, 12px labels, medium weight and the control radius. Observation actions are quiet inline controls with a 34px minimum height. Open evidence has light text, a subtle divider border and a small chevron; Pin and Dismiss have no resting border. Icon buttons have text alternatives and a 32px square area. Disabled buttons reduce opacity and use the unavailable cursor.

Keyboard focus uses the global 2px selection outline with a 2px offset. State transitions last 120ms with cubic-bezier(0.2, 0.8, 0.2, 1). The inspector opacity transition lasts 160ms. Reduced motion removes decorative transitions; do not reintroduce continuous pulses.

### Inputs / Fields

The top-bar search is a dark capsule with a quiet divider border, search icon and shortcut hint. It has 12px horizontal padding. Its width can shrink without forcing the shell wider. Focus strengthens the border and a query error appears next to the field. External DNS fields use 14px input text and 38px height. Keep labels and invalid states visible.

### Chips

Compound query filters use compact rounded chips with values and a removal action. Evidence badges distinguish scope or change state with text. Live protocol tags have a muted fill, mono type and compact padding. Active or closed text accompanies the small state dot, so state does not depend on its color.

### Cards / Containers

The continuous observation list uses the carbon background, panel radius and 24px horizontal inset. Each row has 22px vertical padding and a quiet bottom separator except the last. A title appears before its category and scope. Body copy stays readable at 14px with a 72-character maximum measure. Quiet actions sit beside the content and stack below it in the compact layout. Pinned observations show a small green dot beside the title.

The session strip uses the raised panel tone, enclosing edge and larger mono values without an extra visible category kicker. Its labels can wrap and its six metrics have internal dividers. The notification inbox is an enclosed carbon panel with a centered empty message. Settings sections and evidence tables use opaque rounded groups with enclosing edges. Settings rows and table toolbars use quiet internal separators; controls retain their rounded shape.

### Navigation

The straight left sidebar uses 14px text, small outline icons and a warm muted fill for its active item. Its items have a 46px minimum height and no visible active border. Brand and purpose blocks have no dividing rule. aria-current identifies the current area. Compact navigation retains accessible names and titles when the visible text is hidden. The bottom sidebar copy identifies local processing. Settings has a separate in-page section index whose current section follows scrolling in the main content container.

### Connection feed and globe

Process names lead the row, followed by destination, a compact protocol tag, location, timing and rates. Active or closed text sits beside a small state dot. The inset feed has a rounded obsidian background, an 8px list inset and quiet group separators. Selection gives a row a warmer fill without an edge stripe. Closed connections keep readable opacity. Metadata remains readable in fog. Keep unlocated evidence in the feed and expose its missing location in context. Scroll the selected group into view when selection changes; background group refreshes must not pull the reader away from their scroll position.

The globe uses a packaged grayscale NASA relief image as a geographic backdrop. Authoritative local atlas boundaries and known-coordinate markers are separate overlays. The 2D relief backdrop and atlas use a registered equirectangular projection. Destination markers have crisp rings rather than diffuse halos. Selected markers and routes use orange; ordinary destination rings use fog and their cores use mist. The home marker has a green core and white ring above a coincident destination marker. The legend matches these visible states with a green outlined home symbol, quiet destination dot and orange selected-route dot. Generated reference map pixels never supply geographic data.

The globe schedules a frame on changes or interaction, stops rendering while hidden, and caps displayed routes at 120. The cap is a rendering constraint, not a throughput claim. Its readable caption contains only process, country and destination, with filter actions for each value. Hide the caption in the stacked compact layout. Zoom, reset, view and workspace controls use bordered warm glass groups and maintain an opaque fallback.

### Devices and local services

Device tiles group known physical addresses under their interface. Selection opens a device inspector with addresses, interface and observed connections. Neighbor cache evidence stays explicit and does not imply traffic or links between devices. Listener diagrams pair the observed process with its actual local listening endpoint. They do not invent remote peers or reinterpret the destination field as a listener address.

### Retained evidence and snapshots

Investigate separates the query workspace, temporal evidence and sessions. Opaque query and timeline groups lead into selectable retained records; long evidence opens in a contextual inspector. Exposure has a snapshot ledger and previous/current comparison, with selectable snapshot nodes and an evidence table that follows the selected snapshot. Keep timestamp and hash identity visible. Preserve zeros, unavailable counters and empty evidence rather than filling the composition with invented records.

### Settings and diagnostics

External DNS uses visible field labels and opaque input groups beside explanatory context and imported evidence. The in-page index tracks the visible settings section. Diagnostics presents provider and event-engine counters, capabilities, buffering and loss, with the self-test panel after the engine details. Browser fixtures do not establish native capture reliability.

### Command palette

The command palette is a bounded scrollable warm glass overlay with an 18px radius and a maximum width of 650px. The query is 16px, action rows are at least 44px high, and the selected row has a visible fill and inset outline. Keyboard hints sit in its footer. Enhanced contrast and reduced transparency remove background blur and use the opaque raised-panel tone. The palette keeps text readable without introducing a blue or cyan identity.

## Do's and Don'ts

### Do:

- Do use the supplied serif AXVARIS wordmark and matching X.
- Do keep process names, evidence text and metric values stronger than metadata.
- Do place observation titles before category and scope, with quiet inline actions.
- Do preserve the Live reader's scroll position during background group refreshes.
- Do match legend symbols to the actual home, destination and selected-route colors.
- Do preserve the five-area sidebar and stacked compact Live layout.
- Do keep visible focus, text labels for states and reduced motion behavior.
- Do distinguish local capture, imported DNS queries and demo data in the interface.
- Do keep evidence opaque and reserve warm glass for floating controls and the command palette.
- Do retain snapshot identity, zeros, unavailable values and unknown attribution.
- Do group local devices by interface and inspect observed relationships beside their source.

### Don't:

- Don't add a crossing line, glow or neon accent to the brand assets.
- Don't use continuous decorative particles or pulsing chrome.
- Don't draw destinations without known coordinates.
- Don't turn imported DNS queries into confirmed connections or remote process identities.
- Don't describe the UI as certified accessible or resource-efficient without corresponding validation evidence.
- Don't turn neighbor cache entries into proof of observed traffic or draw invented device links.
- Don't use generated map geometry as geographic evidence.
