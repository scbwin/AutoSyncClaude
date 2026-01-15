const { PNG } = require('pngjs');
const fs = require('fs');
const path = require('path');

// Generate a 1024x1024 PNG icon
const size = 1024;
const png = new PNG({
  width: size,
  height: size,
  filterType: -1 // Auto filtering
});

// Create a dark gray background with blue circle
for (let y = 0; y < size; y++) {
  for (let x = 0; x < size; x++) {
    const idx = (size * y + x) << 2;

    // Default dark gray background
    png.data[idx] = 0x1e;     // R
    png.data[idx + 1] = 0x1e;  // G
    png.data[idx + 2] = 0x1e;  // B
    png.data[idx + 3] = 0xff;  // A

    // Add blue circle in center
    const cx = size / 2;
    const cy = size / 2;
    const radius = size * 0.4;
    const dist = Math.sqrt((x - cx) ** 2 + (y - cy) ** 2);

    if (dist < radius) {
      png.data[idx] = 0x64;     // R - blue
      png.data[idx + 1] = 0x95;  // G
      png.data[idx + 2] = 0xed;  // B
      png.data[idx + 3] = 0xff;  // A
    }

    // Add inner dark circle
    if (dist < radius * 0.6) {
      png.data[idx] = 0x1e;     // R
      png.data[idx + 1] = 0x1e;  // G
      png.data[idx + 2] = 0x1e;  // B
      png.data[idx + 3] = 0xff;  // A
    }
  }
}

// Write the PNG file
const outputPath = path.join(__dirname, 'src-tauri', 'icon-source.png');
const writeStream = fs.createWriteStream(outputPath);

png.pack().pipe(writeStream);

writeStream.on('finish', () => {
  const stats = fs.statSync(outputPath);
  console.log(`✓ Generated ${size}x${size} icon at ${outputPath}`);
  console.log(`  File size: ${stats.size} bytes`);
});

writeStream.on('error', (err) => {
  console.error('Error writing PNG:', err);
  process.exit(1);
});
