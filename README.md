# DeepSeek Harness App

English | [中文](README.zh.md)

Tauri 2 window for the official DeepSeek Harness Web UI. This repository is **only the native shell**. The harness itself is referenced, not vendored:

- Pin: [`upstream.json`](upstream.json) and `package.json` `upstream`
- Source: [deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness)
- Runtime package: [`@deepseek-ai/dsh@0.1.0-rc.7`](https://www.npmjs.com/package/@deepseek-ai/dsh)

Packaged installers are out of the box: they ship official Node 22 plus that npm package. Users do not install Node, pnpm, or DSH. [anywhere-labs/deepseek-harness-desktop](https://github.com/anywhere-labs/deepseek-harness-desktop) does the same by embedding Node inside Electron; this app stays on Tauri and embeds the same official Node + `@deepseek-ai/dsh` binaries instead.

## What it does

- **Windows / macOS / Linux:** open straight into the official Web UI. The shell starts `dsh web` in the background and only shows settings on failure or from the tray. Close hides to the tray; Quit from the tray exits. Unpackaged `pnpm dev` can also use a system Node.
- **First launch:** installs four `web` profile plugins (`dsh-web-ui`, Transparent UI, better-sidebar, `dsh-visualize`) before the UI loads.
- **Android / iOS:** connect-only. Enter a reachable `dsh web` host and port.
- **Settings:** persist host, port, auto-start, and launch mode.
- **DSH updates:** `npm install @deepseek-ai/dsh@latest` in the app-data prefix. That does not rebuild this shell.

Optional local checkout (not required, not a submodule): set `DSH_CHECKOUT` to a clone of `deepseek-ai/deepseek-harness`. The shell then uses `pnpm dsh web` from that tree. Nearby folders named `deepseek-harness` are ignored so a source tree cannot stall first launch.

`dsh web` refuses `--host 0.0.0.0`. A phone cannot reach a loopback-only desktop server without a tunnel.

## Develop

Node 22.19+ and Rust stable:

```sh
pnpm install
pnpm icons
pnpm dev
```

Local installers: `pnpm build`.

On GitHub: **Actions → desktop → Run workflow**, or push a `v*` tag. A successful run publishes a GitHub Release with Windows NSIS/MSI, macOS universal DMG, and Linux AppImage/deb. GitHub's source zip is not the app.

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
