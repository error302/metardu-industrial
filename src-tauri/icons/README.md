# Icons

Tauri requires platform-specific icon files (referenced in `src-tauri/tauri.conf.json`):
- `32x32.png`
- `128x128.png`
- `128x128@2x.png`
- `icon.icns` (macOS)
- `icon.ico` (Windows)

Generate these from a source SVG using `cargo tauri icon path/to/source.png` once we have a final 1024×1024 PNG of the MetaRDU Industrial logo.

For now, copy the defaults from `cargo create-tauri-app` or run `cargo tauri icon` against `public/favicon.svg` (after converting to PNG).

Until icons are present, the Tauri build will fail at the bundling step. The frontend (`npm run dev`) works fine without them.
