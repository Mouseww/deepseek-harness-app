import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const branding = join(root, 'branding')
const icons = join(root, 'src-tauri', 'icons')
const pub = join(root, 'public')

function pngIco(png) {
  const header = Buffer.alloc(22)
  header.writeUInt16LE(0, 0)
  header.writeUInt16LE(1, 2)
  header.writeUInt16LE(1, 4)
  header.writeUInt8(0, 6)
  header.writeUInt8(0, 7)
  header.writeUInt8(0, 8)
  header.writeUInt8(0, 9)
  header.writeUInt16LE(1, 10)
  header.writeUInt16LE(32, 12)
  header.writeUInt32LE(png.length, 14)
  header.writeUInt32LE(22, 18)
  return Buffer.concat([header, png])
}

mkdirSync(icons, { recursive: true })
mkdirSync(pub, { recursive: true })
const source = join(branding, 'app-icon.png')
if (!existsSync(source)) throw new Error('missing branding/app-icon.png')
copyFileSync(source, join(icons, 'icon.png'))
const splash = join(icons, '256x256.png')
if (existsSync(splash)) copyFileSync(splash, join(pub, 'app-icon.png'))
else copyFileSync(source, join(pub, 'app-icon.png'))
if (existsSync(splash)) writeFileSync(join(icons, 'icon.ico'), pngIco(readFileSync(splash)))
console.log('icons: copied official DSH artwork from branding/')

