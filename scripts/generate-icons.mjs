/**
 * Write a lime-on-charcoal PNG set plus a one-image ICO for Tauri bundle icons.
 * ICNS is left for `pnpm exec tauri icon` when a macOS build needs it.
 */
import { deflateSync } from 'node:zlib'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const outDir = join(dirname(fileURLToPath(import.meta.url)), '../src-tauri/icons')

/**
 * CRC-32 of a buffer, as PNG chunks require.
 * @param {Buffer} buffer
 * @returns {number}
 */
function crc32(buffer) {
  let crc = 0xffffffff
  for (const byte of buffer) {
    crc ^= byte
    for (let i = 0; i < 8; i += 1) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1))
  }
  return (crc ^ 0xffffffff) >>> 0
}

/**
 * One PNG chunk.
 * @param {string} type
 * @param {Buffer} data
 * @returns {Buffer}
 */
function chunk(type, data) {
  const header = Buffer.alloc(8)
  header.writeUInt32BE(data.length, 0)
  header.write(type, 4)
  const crcBuf = Buffer.alloc(4)
  crcBuf.writeUInt32BE(crc32(Buffer.concat([header.subarray(4), data])), 0)
  return Buffer.concat([header, data, crcBuf])
}

/**
 * Encode a square RGBA PNG.
 * @param {number} size
 * @returns {Buffer}
 */
function png(size) {
  const raw = Buffer.alloc((size * 4 + 1) * size)
  for (let y = 0; y < size; y += 1) {
    const row = y * (size * 4 + 1)
    raw[row] = 0
    for (let x = 0; x < size; x += 1) {
      const i = row + 1 + x * 4
      const nx = (x + 0.5) / size - 0.5
      const ny = (y + 0.5) / size - 0.5
      const r = Math.hypot(nx, ny)
      const inMark = r < 0.28 || (Math.abs(nx) < 0.08 && ny > -0.32 && ny < 0.34)
      raw[i] = inMark ? 0xc6 : 0x14
      raw[i + 1] = inMark ? 0xf0 : 0x1c
      raw[i + 2] = inMark ? 0x4d : 0x16
      raw[i + 3] = 255
    }
  }
  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(size, 0)
  ihdr.writeUInt32BE(size, 4)
  ihdr[8] = 8
  ihdr[9] = 6
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

/**
 * Pixel colors for the lime mark on charcoal.
 * @param {number} x
 * @param {number} y
 * @param {number} size
 * @returns {[number, number, number, number]}
 */
function pixel(x, y, size) {
  const nx = (x + 0.5) / size - 0.5
  const ny = (y + 0.5) / size - 0.5
  const r = Math.hypot(nx, ny)
  const inMark = r < 0.28 || (Math.abs(nx) < 0.08 && ny > -0.32 && ny < 0.34)
  return inMark ? [0xc6, 0xf0, 0x4d, 255] : [0x14, 0x1c, 0x16, 255]
}

/**
 * Classic 32-bit BMP image payload for one ICO entry. RC.EXE rejects PNG-in-ICO.
 * @param {number} size
 * @returns {Buffer}
 */
function icoBitmap(size) {
  const xor = Buffer.alloc(size * size * 4)
  for (let y = 0; y < size; y += 1) {
    const srcY = size - 1 - y
    for (let x = 0; x < size; x += 1) {
      const [r, g, b, a] = pixel(x, srcY, size)
      const i = (y * size + x) * 4
      xor[i] = b
      xor[i + 1] = g
      xor[i + 2] = r
      xor[i + 3] = a
    }
  }
  const andRow = Math.ceil(size / 32) * 4
  const andMask = Buffer.alloc(andRow * size)
  const header = Buffer.alloc(40)
  header.writeUInt32LE(40, 0)
  header.writeInt32LE(size, 4)
  header.writeInt32LE(size * 2, 8)
  header.writeUInt16LE(1, 12)
  header.writeUInt16LE(32, 14)
  return Buffer.concat([header, xor, andMask])
}

/**
 * Multi-size BMP ICO.
 * @param {number[]} sizes
 * @returns {Buffer}
 */
function ico(sizes) {
  const images = sizes.map(size => icoBitmap(size))
  const header = Buffer.alloc(6 + 16 * sizes.length)
  header.writeUInt16LE(0, 0)
  header.writeUInt16LE(1, 2)
  header.writeUInt16LE(sizes.length, 4)
  let offset = header.length
  for (const [index, size] of sizes.entries()) {
    const entry = 6 + index * 16
    header[entry] = size
    header[entry + 1] = size
    header.writeUInt16LE(1, entry + 4)
    header.writeUInt16LE(32, entry + 6)
    header.writeUInt32LE(images[index].length, entry + 8)
    header.writeUInt32LE(offset, entry + 12)
    offset += images[index].length
  }
  return Buffer.concat([header, ...images])
}

mkdirSync(outDir, { recursive: true })
const icon32 = png(32)
const icon128 = png(128)
const icon256 = png(256)
writeFileSync(join(outDir, '32x32.png'), icon32)
writeFileSync(join(outDir, '128x128.png'), icon128)
writeFileSync(join(outDir, 'icon.png'), icon256)
writeFileSync(join(outDir, 'icon.ico'), ico([16, 32, 48]))
console.log(`wrote icons in ${outDir}`)
