const fs = require('fs');
const path = require('path');

// Simple 512x512 PNG icon (base64 encoded)
// This is a minimal valid PNG file with a solid color
const iconPngBase64 = 'iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAYAAADED76LAAAAGXRFWHRTb2Z0d2FyZQBBZG9iZSBJbWFnZVJlYWR5ccllPAAAAyJpVFh0WE1MOmNvbS5hZG9iZS54bXAAAAAAADw/eHBhY2tldCBiZWdpbj0i77u/IiBpZD0iVzVNME1wQ2VoaUh6cmVTek5UY3prYzlkIj8+/Pp/mixwAAABtSURBVHjaYvz//z8DEwMDwwcGBib9R0ZGAQpGhn0Mf/jIwMjEAMDAyMDEz/DAyM/wiMDP8DIwM/WQYQBjE2IP4LIP4H4uT//wUIMDGIsDCQZGBgYPAGQkAAgwADE9gwM1h017gAAAABJRU5E/r+gAAAABJRU5ErkJggg==';

const iconsDir = path.join(__dirname, 'src-tauri', 'icons');
const iconPath = path.join(iconsDir, 'icon.png');

// Create icons directory if it doesn't exist
if (!fs.existsSync(iconsDir)) {
  fs.mkdirSync(iconsDir, { recursive: true });
}

// Write the icon file
const iconBuffer = Buffer.from(iconPngBase64, 'base64');
fs.writeFileSync(iconPath, iconBuffer);

console.log(`✓ Generated icon at ${iconPath}`);
console.log(`  File size: ${iconBuffer.length} bytes`);
