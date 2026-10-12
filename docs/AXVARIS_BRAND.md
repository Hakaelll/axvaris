# Axvaris identity

The X shares the wordmark's serif terminals and stroke contrast, without an added crossing line. SVG exports contain paths and require no installed logo font. Light ink belongs on dark surfaces; dark ink belongs on light surfaces.

Assets in `apps/desktop/public/brand`:

- `axvaris-mark-light.svg`, `axvaris-mark-dark.svg`
- `axvaris-mark-light.png`, `axvaris-mark-dark.png`
- `axvaris-wordmark-light.svg`, `axvaris-wordmark-dark.svg`
- `axvaris-wordmark-light.png`, `axvaris-wordmark-dark.png`
- `axvaris-app.png`, with the application background

Windows uses `icon.ico`; Linux uses the PNG family in `apps/desktop/src-tauri/icons`: `16x16.png`, `24x24.png`, `32x32.png`, `48x48.png`, `64x64.png`, `128x128.png`, `256x256.png` and `128x128@2x.png`. Generate the vector and raster exports with `scripts/generate-brand.ps1` on Windows.

Interface fonts are packaged as `Geist-Variable.woff2` and `GeistMono-Variable.woff2`, with `OFL.txt` in `apps/desktop/public/fonts`. Vercel provides these fonts under the SIL Open Font License.

Axvaris preserves `io.sabicheiro.desktop`, the legacy data directory and `.sabi` compatibility. The name is a product choice, not confirmation of trademark or domain availability.
