const fs = require('fs');
const path = require('path');

// Generate a 1024x1024 PNG icon
// This creates a simple but valid PNG file with reasonable size
const size = 1024;

// Create a simple dark blue background with a white 'C' in the center
// We'll create a minimal valid PNG that Tauri can use

// For simplicity, we'll create a solid color PNG
// PNG file structure (simplified):
const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

// Create IHDR chunk
function createIHDR(width, height) {
  const data = Buffer.alloc(13);
  data.writeUInt32BE(width, 0);
  data.writeUInt32BE(height, 4);
  data.writeUInt8(8, 8);  // bit depth
  data.writeUInt8(6, 9);  // color type (RGBA)
  data.writeUInt8(0, 10); // compression
  data.writeUInt8(0, 11); // filter
  data.writeUInt8(0, 12); // interlace

  return createChunk('IHDR', data);
}

function createChunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);

  const typeBuffer = Buffer.from(type, 'ascii');
  const crc = calculateCRC(Buffer.concat([typeBuffer, data]));

  return Buffer.concat([length, typeBuffer, data, crc]);
}

function calculateCRC(buffer) {
  let crc = 0xffffffff;
  for (let i = 0; i < buffer.length; i++) {
    crc ^= buffer[i];
    for (let j = 0; j < 8; j++) {
      if (crc & 1) {
        crc = (crc >>> 1) ^ 0xedb88320;
      } else {
        crc = crc >>> 1;
      }
    }
  }
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE((crc ^ 0xffffffff) >>> 0, 0);
  return crcBuf;
}

// Create image data (1024x1024 dark gray with some pattern)
function createImageData(size) {
  const pixels = Buffer.alloc(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const idx = (y * size + x) * 4;
      // Dark gray background
      pixels[idx] = 0x1e;     // R
      pixels[idx + 1] = 0x1e;  // G
      pixels[idx + 2] = 0x1e;  // B
      pixels[idx + 3] = 0xff;  // A

      // Add a blue circle in the center
      const cx = size / 2;
      const cy = size / 2;
      const radius = size * 0.4;
      const dist = Math.sqrt((x - cx) ** 2 + (y - cy) ** 2);

      if (dist < radius) {
        pixels[idx] = 0x64;     // R - blue
        pixels[idx + 1] = 0x95;  // G
        pixels[idx + 2] = 0xed;  // B
        pixels[idx + 3] = 0xff;  // A
      }

      // Add inner dark circle
      if (dist < radius * 0.6) {
        pixels[idx] = 0x1e;     // R
        pixels[idx + 1] = 0x1e;  // G
        pixels[idx + 2] = 0x1e;  // B
        pixels[idx + 3] = 0xff;  // A
      }
    }
  }
  return pixels;
}

// Compress with zlib (simplified - just store uncompressed)
function createIDAT(data) {
  const zlib = require('zlib');
  const compressed = zlib.deflateSync(data);
  return createChunk('IDAT', compressed);
}

function createIEND() {
  return createChunk('IEND', Buffer.alloc(0));
}

// Generate the PNG
const ihdr = createIHDR(size, size);
const imageData = createImageData(size);
const idat = createIDAT(imageData);
const iend = createIEND();

const png = Buffer.concat([PNG_SIGNATURE, ihdr, idat, iend]);

// Write the file
const outputPath = path.join(__dirname, 'src-tauri', 'icon-source.png');
fs.writeFileSync(outputPath, png);

console.log(`✓ Generated ${size}x${size} icon at ${outputPath}`);
console.log(`  File size: ${png.length} bytes`);
