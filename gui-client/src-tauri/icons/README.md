# Tauri Icons

This directory should contain the application icons for different platforms.

## Required Icons

- `32x32.png` - Linux small icon
- `128x128.png` - Linux medium icon
- `128x128@2x.png` - Linux high DPI icon (256x256)
- `icon.icns` - macOS icon
- `icon.ico` - Windows icon

## Generating Icons

For production builds, you should create proper icons:

### Option 1: Online Tool
Visit https://tauri.app/v1/guides/features/icons/
Use the online icon generator

### Option 2: Command Line (requires ImageMagick)

```bash
# On Linux/macOS
convert icon.svg -resize 32x32 32x32.png
convert icon.svg -resize 128x128 128x128.png
convert icon.svg -resize 256x256 128x128@2x.png
convert icon.svg icon.ico
```

For macOS .icns, use iconutil on a Mac:
```bash
mkdir icon.iconset
# Copy various sizes to icon.iconset/
iconutil -c icns icon.iconset
```

### Option 3: Use Tauri's Default Icons

For development and testing, Tauri will use its default icon
if the custom icons are missing.

## Temporary Solution

For now, the build will work with Tauri's default icons.
You can add custom icons later before final release.
