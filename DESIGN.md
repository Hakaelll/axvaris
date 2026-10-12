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
  connection-selected: "#292522"
typography:
  headline:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "28px"
    fontWeight: 400
    letterSpacing: "-0.025em"
  title:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "17px"
    fontWeight: 500
  body:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "14px"
    lineHeight: 1.55
  row-name:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "15px"
  label:
    fontFamily: 'Geist, "Segoe UI Variable", "Segoe UI", system-ui, sans-serif'
    fontSize: "12px"
  metric:
    fontFamily: '"Geist Mono", ui-monospace, "Cascadia Code", monospace'
    fontSize: "18px"
  mono:
    fontFamily: '"Geist Mono", ui-monospace, monospace'
rounded:
  badge: "4px"
  control: "6px"
  panel: "12px"
  query-chip: "14px"
spacing:
  compact: "6px"
  control: "12px"
  row: "16px"
  card: "18px"
  section: "20px"
  workspace: "24px"
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
    rounded: "{rounded.control}"
    height: "36px"
    padding: "0 9px"
  sidebar-item-active:
    backgroundColor: "{colors.selection-muted}"
    textColor: "{colors.paper}"
    rounded: "{rounded.control}"
    padding: "10px"
  evidence-card:
    backgroundColor: "{colors.carbon}"
    textColor: "{colors.fog}"
    rounded: "{rounded.panel}"
    padding: "18px"
  session-panel:
    backgroundColor: "{colors.obsidian}"
    textColor: "{colors.mist}"
    rounded: "{rounded.panel}"
    padding: "20px"
  connection-row-selected:
    backgroundColor: "{colors.connection-selected}"
    textColor: "{colors.mist}"
    padding: "14px 16px"
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

Axvaris presents network evidence in a restrained desktop workspace. Factory and Gotham are the confirmed references for its serious observability character. Warm dark panels, quiet dividers and readable text separate controls from connection evidence. The globe shares the workspace with the feed; evidence remains available when a destination cannot be plotted.

The serif AXVARIS wordmark supplies the identity. Interface text uses Geist and technical values use Geist Mono. Motion responds to an action or a data change. Continuous decorative particles and pulsing chrome have no role in this design.

**Key Characteristics:**

- Five persistent investigation areas in a left sidebar.
- Monochrome controls with limited color for evidence states.
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
- Carbon separates the sidebar, metric strip and evidence cards from the canvas.
- Obsidian is the raised panel tone. The CSS surface-raised token aliases this value.
- Graphite draws panel boundaries and row dividers. The CSS border-subtle token aliases this value.
- Smoke provides stronger secondary edges.
- Fog is secondary copy. Ash is quieter supplementary text.
- Paper provides maximum contrast for short headings and active navigation.
- Selection muted fills the active sidebar item. Connection selected fills the selected feed row.

**The Evidence Color Rule.** Use color to identify a capture or evidence state. Keep general navigation and actions monochrome.

## Typography

Geist and Geist Mono load from packaged variable WOFF2 files. Preserve the system fallbacks in the frontmatter. Technical values use tabular numerals. The serif wordmark and X are supplied SVG paths, not a third interface font.

The hierarchy uses page headings at 28px, card titles at 17px, row names at 15px and supporting body copy at 14px. Session names use 16px. Session metrics use 18px Geist Mono. Labels and compact controls use 12px. Keep addresses, timestamps, ports and rates in mono without shrinking the primary process name.

**The Identity Rule.** Use the supplied serif wordmark and its matching X. Do not add a crossing line or reconstruct the logo with a system font.

## Layout

The desktop shell has a 196px sidebar, 64px top bar and 44px metric strip. The sidebar spans the window height. Today, Live, Investigate, Exposure and Data & Health remain in that order. Search, capture state and actions sit in the top bar. Contextual inspection does not require another permanent navigation column.

The native window has a minimum size of 720 by 600. At 1180px and below, the sidebar becomes a 72px icon rail and the wordmark yields to the X. Names remain available through button labels and tooltips. At 1100px and below, the Today session strip stacks and its metrics use three columns. At 900px and below, the Live canvas stacks above the feed, with a canvas row of at least 250px. Settings navigation wraps above its content. Narrow capture actions move into a compact menu.

Use the frontmatter spacing for related controls, rows, cards and sections. Connection rows have a 112px minimum height and 14px by 16px padding. Evidence card grids use a 14px gap. Keep primary values visible before reducing spacing or wrapping metadata. Allow long evidence tables and the bottom metric strip to scroll within their bounds.

## Elevation & Depth

The main workspaces use tone and one-pixel borders for depth. Today cards, session strips and settings sections have no shadow. The inspector and notification center also have no shadow. Transient menus retain a dark shadow for separation from the underlying workspace; that exception is recorded in the sidecar.

**The Panel Rule.** Separate persistent panels with their background and edge. Reserve shadows for transient overlays that already use them.

## Shapes

Controls, search fields, sidebar items and view switchers use the control radius. Main evidence cards, session strips, comparison panels and settings sections use the panel radius. Small evidence badges use the badge radius; compound query chips retain the larger pill radius. These shapes come from the effective stylesheet, not a universal rounding rule.

The monochrome X has the wordmark's serif terminals and stroke contrast. Use light assets against the dark shell and dark assets against light backgrounds. Preserve the supplied geometry and keep the mark free of glow or additional crossing strokes.

## Components

### Buttons

Primary actions have a light fill and dark text. Hover uses a quieter warm light fill. Secondary actions have a dark fill and visible border. Base controls use a 32px minimum height, 12px labels, medium weight and the control radius. Card footer actions may use the existing 28px compact height. Icon buttons have text alternatives and a 32px square area. Disabled buttons reduce opacity and use the unavailable cursor.

Keyboard focus uses the global 2px selection outline with a 2px offset. State transitions last 120ms with cubic-bezier(0.2, 0.8, 0.2, 1). The inspector opacity transition lasts 160ms. Reduced motion removes decorative transitions; do not reintroduce continuous pulses.

### Inputs / Fields

The top-bar search is a compact dark field with a graphite border, search icon and shortcut hint. Its width can shrink without forcing the shell wider. Focus strengthens the border and a query error appears next to the field. External DNS fields use 14px input text and 38px height. Keep labels and invalid states visible.

### Chips

Compound query filters use compact rounded chips with values and a removal action. Evidence badges distinguish scope or change state with text and a border. Do not substitute unlabeled color dots for evidence categories.

### Cards / Containers

Evidence cards use the carbon background, graphite border and panel radius with 18px padding. Their title is stronger than their category label; body copy stays readable at 14px. Primary evidence actions remain near the description. The session strip uses the raised panel tone and larger mono values without an extra visible category kicker. Major settings sections use the same panel radius.

### Navigation

The left sidebar uses 14px text, small outline icons and a bordered muted fill for its active item. aria-current identifies the current area. Compact navigation retains accessible names and titles when the visible text is hidden. The bottom sidebar copy identifies local processing.

### Connection feed and globe

Process names lead the row, followed by destination, protocol, timing and rates. Selection gives a row a darker warm fill and an orange edge. Metadata remains readable in fog. Keep unlocated evidence in the feed and expose its missing location in context.

The globe uses local country geometry and bounded route rendering. It schedules a frame on changes or interaction, stops rendering while hidden, and caps displayed routes at 120. The cap is a rendering constraint, not a throughput claim. The home marker has a green core and white ring above a coincident destination marker.

## Do's and Don'ts

### Do:

- Do use the supplied serif AXVARIS wordmark and matching X.
- Do keep process names, evidence text and metric values stronger than metadata.
- Do preserve the five-area sidebar and stacked compact Live layout.
- Do keep visible focus, text labels for states and reduced motion behavior.
- Do distinguish local capture, imported DNS queries and demo data in the interface.

### Don't:

- Don't add a crossing line, glow or neon accent to the brand assets.
- Don't use continuous decorative particles or pulsing chrome.
- Don't draw destinations without known coordinates.
- Don't turn imported DNS queries into confirmed connections or remote process identities.
- Don't describe the UI as certified accessible or resource-efficient without corresponding validation evidence.
