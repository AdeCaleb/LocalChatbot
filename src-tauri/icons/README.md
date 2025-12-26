# App Icons

Icons need to be added here before building for production.

## Required icon files:
- 32x32.png
- 128x128.png
- 128x128@2x.png
- icon.icns (macOS)
- icon.ico (Windows)

## Generate icons:

You can use Tauri's icon generator:
```bash
cargo tauri icon path/to/your/app-icon.png
```

For development, the app will use default Tauri icons.
