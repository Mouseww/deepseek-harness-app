import { chmodSync, cpSync, createWriteStream, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { spawnSync } from 'node:child_process'
import { dirname, join } from 'node:path'
import { Readable } from 'node:stream'
import { pipeline } from 'node:stream/promises'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const runtime = join(root, 'src-tauri', 'runtime')
const nodeDir = join(runtime, 'node')
const dshDir = join(runtime, 'dsh')
const nodeVersion = '22.19.0'
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))
const dshSpec = pkg.upstream.package + '@' + pkg.upstream.version

function target() {
  const platform = process.platform
  const arch = process.arch
  if (platform === 'win32' && arch === 'x64') {
    return { id: 'win-x64', archive: 'node-v' + nodeVersion + '-win-x64.zip', kind: 'zip', nodeRel: ['node.exe'], dest: 'node.exe' }
  }
  if (platform === 'darwin' && arch === 'arm64') {
    return { id: 'darwin-arm64', archive: 'node-v' + nodeVersion + '-darwin-arm64.tar.gz', kind: 'tar', nodeRel: ['bin', 'node'], dest: 'node' }
  }
  if (platform === 'darwin' && arch === 'x64') {
    return { id: 'darwin-x64', archive: 'node-v' + nodeVersion + '-darwin-x64.tar.gz', kind: 'tar', nodeRel: ['bin', 'node'], dest: 'node' }
  }
  if (platform === 'linux' && arch === 'x64') {
    return { id: 'linux-x64', archive: 'node-v' + nodeVersion + '-linux-x64.tar.xz', kind: 'tar', nodeRel: ['bin', 'node'], dest: 'node' }
  }
  throw new Error('bundle-runtime: unsupported ' + platform + '-' + arch)
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, stdio: 'inherit', shell: false })
  if (result.status !== 0) throw new Error(command + ' failed (' + result.status + ')')
}

async function download(url, dest) {
  const response = await fetch(url)
  if (!response.ok || response.body === null) throw new Error('download failed ' + response.status + ' ' + url)
  mkdirSync(dirname(dest), { recursive: true })
  await pipeline(Readable.fromWeb(response.body), createWriteStream(dest))
}

function extract(archive, outDir) {
  mkdirSync(outDir, { recursive: true })
  run('tar', ['-xf', archive, '-C', outDir])
}

function findExtractedRoot(outDir, prefix) {
  const match = readdirSync(outDir).find((name) => name.startsWith(prefix) && statSync(join(outDir, name)).isDirectory())
  if (match === undefined) throw new Error('extracted Node tree not found in ' + outDir)
  return join(outDir, match)
}

const spec = target()
const nodeBin = join(nodeDir, spec.dest)
const dshBin = join(dshDir, 'node_modules', '@deepseek-ai', 'dsh', 'lib', 'bin.js')
const manifestPath = join(runtime, 'manifest.json')
const wanted = { node: nodeVersion, dsh: pkg.upstream.version, platform: spec.id }

if (existsSync(nodeBin) && existsSync(dshBin) && existsSync(manifestPath)) {
  try {
    const have = JSON.parse(readFileSync(manifestPath, 'utf8'))
    if (have.node === wanted.node && have.dsh === wanted.dsh && have.platform === wanted.platform) {
      console.log('bundle-runtime: reuse ' + spec.id)
      process.exit(0)
    }
  } catch {
    // rebuild
  }
}

rmSync(runtime, { recursive: true, force: true })
mkdirSync(runtime, { recursive: true })

const cache = join(root, '.runtime-cache')
mkdirSync(cache, { recursive: true })
const archive = join(cache, spec.archive)
const url = 'https://nodejs.org/dist/v' + nodeVersion + '/' + spec.archive
if (!existsSync(archive)) {
  console.log('bundle-runtime: download ' + url)
  await download(url, archive)
}

const unpacked = join(cache, 'unpacked-' + spec.id)
rmSync(unpacked, { recursive: true, force: true })
extract(archive, unpacked)
const extractedRoot = findExtractedRoot(unpacked, 'node-v' + nodeVersion)
mkdirSync(nodeDir, { recursive: true })
const extractedNode = join(extractedRoot, ...spec.nodeRel)
if (!existsSync(extractedNode)) throw new Error('missing ' + extractedNode)
cpSync(extractedNode, nodeBin)
if (process.platform !== 'win32') chmodSync(nodeBin, 0o755)

console.log('bundle-runtime: npm install ' + dshSpec)
mkdirSync(dshDir, { recursive: true })
writeFileSync(join(dshDir, 'package.json'), JSON.stringify({ private: true, name: 'dsh-runtime' }, null, 2) + '\n')
run(process.platform === 'win32' ? 'npm.cmd' : 'npm', [
  'install',
  dshSpec,
  '--omit=dev',
  '--no-fund',
  '--no-audit',
  '--prefix',
  dshDir,
], dshDir)

if (!existsSync(dshBin)) throw new Error('npm install did not produce ' + dshBin)
writeFileSync(manifestPath, JSON.stringify(wanted, null, 2) + '\n')
console.log('bundle-runtime: wrote ' + runtime)

