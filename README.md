# DeepSeek Harness App

English | [中文](README.zh.md)

Tauri 2 window for the official DeepSeek Harness Web UI. This repository is **only the native shell**. The harness itself is referenced, not vendored:

- Pin: [`upstream.json`](upstream.json) and `package.json` `peerDependencies["@deepseek-ai/dsh"]`
- Source: [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness)
- Runtime package: [`@deepseek-ai/dsh@0.1.0-rc.7`](https://www.npmjs.com/package/@deepseek-ai/dsh)

At runtime the shell launches or updates that npm package. It does not contain the harness tree.

## What it does

- **Windows / macOS / Linux:** spawn `dsh web --host <host> --port <port>` via a managed app-data install, `dsh` on PATH, or `npx @deepseek-ai/dsh`, then navigate to the ready URL.
- **Android / iOS:** connect-only. Enter a reachable `dsh web` host and port.
- **Settings:** persist host, port, auto-start, and launch mode.
- **DSH updates:** `npm install @deepseek-ai/dsh@latest` in the app-data prefix. That does not rebuild this shell.

Optional local checkout (not required, not a submodule): set `DSH_CHECKOUT` to a clone of `deepseek-ai/deepseek-harness`, or keep a sibling directory named `deepseek-harness`. The shell then uses `pnpm dsh web` from that tree.

`dsh web` refuses `--host 0.0.0.0`. A phone cannot reach a loopback-only desktop server without a tunnel.

## Develop

Node 22.19+ and Rust stable:

```sh
pnpm install
pnpm icons
pnpm dev
```

Local installers: `pnpm build`.

On GitHub: **Actions → desktop → Run workflow**. The run uploads Windows NSIS, macOS universal DMG, and Linux AppImage/deb. A `v*` tag also opens a draft Release.

## Tests

```sh
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml
```

## Upstream

| Piece | Where it lives |
|---|---|
| Agent, tools, Web UI, `dsh web` | [`@deepseek-ai/dsh`](https://www.npmjs.com/package/@deepseek-ai/dsh) / [deepseek-harness](https://github.com/deepseek-ai/deepseek-harness) |
| Native window, tray, host/port, DSH updater | this repository |
