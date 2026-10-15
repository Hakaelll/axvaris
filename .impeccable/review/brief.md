# Axvaris reference implementation

Original request: "mira las imagens de la guia para ia y actualiza la interfaz siguiendo las pautas".

The visual authority is `design-references/axvaris-2026-10-13/README.md` and the 13 selected PNG files in that directory. `originales/` contains the source captures. `variantes/` is not the selected direction. Apply the selected composition and materials while preserving the source data, product behavior and its uncertainty.

Requirements:

- Preserve the original X, wordmark, local Geist fonts, warm dark palette and five investigation areas.
- Give observations, device groups, socket endpoints, stored connections and signed snapshots their own appropriate structure. Put actions beside the investigated object and long evidence in an inspector.
- Separate known neighbor cache entries from observed traffic. Draw only the process-to-endpoint relationships present in a listener record.
- Plot only existing coordinates. Use actual packaged atlas geometry, not generated map pixels, as geographic data.
- Keep zeros, unavailable counters, unknown attribution, empty states, dates and snapshot identity accurate. Do not add security scores or malware verdicts.
- Keep one explanation or a consultable detail instead of repeating long explanations in each row.
- Use warm glass for floating map controls, temporal controls and the command palette. Keep fields, tables and evidence opaque. Support reduced transparency and an opaque enhanced-contrast option.
- Preserve visible labels, keyboard focus, distinguishable selection and text labels for states.

Implementation is in `apps/desktop/src/`. The native minimum window remains 720 by 600. Browser screenshots also cover a 390px fallback; that width is not a new native device target.

Evidence is in this directory. `verification.json` lists the 20 required captures and tested behaviors. The 13 numbered screenshots correspond to the guide, plus an opaque palette and Today, Live and DNS at 720px and 390px. The verification runs against the browser preview. Additional listener and retained snapshot fixtures are injected only into the test page, under its visible demonstration mode. They do not establish native capture reliability or represent live machine data.

The design detector output is `detector.json`. Type checks, Spanish translation audit, component behavior tests, production build and frontend audit have passed. The component tests also check local listener endpoints, unavailable counters and geographic projection anchors. A packaged NASA equirectangular relief image supplies the neutral geographic backdrop; the authoritative atlas borders and known-coordinate markers remain separate overlays.

Review every required screenshot against its matching selected reference and inspect implementation where needed. Report the actual disposition and scope; compilation alone does not establish visual fidelity or accessibility.
