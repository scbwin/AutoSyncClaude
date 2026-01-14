#!/bin/bash
# Generate Tauri icons from a SVG source
# This script requires ImageMagick and other tools

echo "Generating Tauri icons..."

# Create a simple SVG icon as base
cat > claude-sync-icon.svg << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<svg width="512" height="512" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
  <rect width="512" height="512" rx="100" fill="#3B82F6"/>
  <path d="M256 128L128 256H192V384H320V256H384L256 128Z" fill="white"/>
  <circle cx="256" cy="416" r="32" fill="#10B981"/>
</svg>
EOF

# Generate PNG icons
convert -background none claude-sync-icon.svg -resize 32x32 32x32.png
convert -background none claude-sync-icon.svg -resize 128x128 128x128.png
convert -background none claude-sync-icon.svg -resize 256x256 128x128@2x.png
convert -background none claude-sync-icon.svg -resize 512x512 icon.png

# Generate ICO (Windows)
convert 32x32.png 128x128.png icon.ico

# Generate ICNS (macOS) - requires iconutil
# On macOS: mkdir icon.iconset && cp *.png icon.iconset/
# iconutil -c icns icon.iconset

echo "Icons generated!"
echo "Note: For macOS, you'll need to run this on a Mac to generate the .icns file"
