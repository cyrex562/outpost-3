# Icons

Tauri expects platform icons here, referenced by `tauri.conf.json`:

- `32x32.png`
- `128x128.png`
- `128x128@2x.png`
- `icon.ico` (Windows)

Generate them all from a single source image once a real logo exists:

```sh
cargo tauri icon path/to/source-1024x1024.png
```

Until then the bundle will use placeholder icons and `cargo tauri build` will
warn about the missing files. They are intentionally not committed as binaries.
