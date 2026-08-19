import { copyFileSync, existsSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const icons = join(root, 'src-tauri', 'icons')
const pub = join(root, 'public')

function u32(value) {
  const buf = Buffer.alloc(4)
  buf.writeUInt32BE(value)
  return buf
}

function pngIco(entries) {
  const count = entries.length
  const header = Buffer.alloc(6)
  header.writeUInt16LE(0, 0)
  header.writeUInt16LE(1, 2)
  header.writeUInt16LE(count, 4)
  const dir = []
  const blobs = []
  let offset = 6 + 16 * count
  for (const { size, png } of entries) {
    const row = Buffer.alloc(16)
    row.writeUInt8(size >= 256 ? 0 : size, 0)
    row.writeUInt8(size >= 256 ? 0 : size, 1)
    row.writeUInt8(0, 2)
    row.writeUInt8(0, 3)
    row.writeUInt16LE(1, 4)
    row.writeUInt16LE(32, 6)
    row.writeUInt32LE(png.length, 8)
    row.writeUInt32LE(offset, 12)
    dir.push(row)
    blobs.push(png)
    offset += png.length
  }
  return Buffer.concat([header, ...dir, ...blobs])
}

function icns(entries) {
  const blocks = entries.map(({ type, png }) => Buffer.concat([Buffer.from(type), u32(8 + png.length), png]))
  const body = Buffer.concat(blocks)
  return Buffer.concat([Buffer.from('icns'), u32(8 + body.length), body])
}

function readPng(name) {
  const path = join(icons, name)
  if (!existsSync(path)) throw new Error('missing ' + path)
  return readFileSync(path)
}

const png16 = readPng('16x16.png')
const png32 = readPng('32x32.png')
const png64 = readPng('64x64.png')
const png128 = readPng('128x128.png')
const png256 = readPng('256x256.png')
const png512 = readPng('512x512.png')

writeFileSync(join(icons, 'icon.png'), png512)
writeFileSync(join(icons, 'icon.ico'), pngIco([
  { size: 32, png: png32 },
  { size: 256, png: png256 },
]))
writeFileSync(join(icons, 'icon.icns'), icns([
  { type: 'icp4', png: png16 },
  { type: 'icp5', png: png32 },
  { type: 'icp6', png: png64 },
  { type: 'ic07', png: png128 },
  { type: 'ic08', png: png256 },
  { type: 'ic09', png: png512 },
  { type: 'ic11', png: png32 },
  { type: 'ic12', png: png64 },
  { type: 'ic13', png: png256 },
  { type: 'ic14', png: png512 },
]))
if (existsSync(pub)) writeFileSync(join(pub, 'app-icon.png'), png256)
console.log('icons: wrote icon.icns / icon.ico / icon.png from official DSH artwork')

