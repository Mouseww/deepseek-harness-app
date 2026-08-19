# DeepSeek Harness App

[English](README.md) | 中文

面向官方 DeepSeek Harness Web UI 的 Tauri 2 窗口。本仓库 **只有原生外壳**。Harness 是引用，不是内嵌源码：

- 钉扎：[`upstream.json`](upstream.json) 与 `package.json` 的 `peerDependencies["@deepseek-ai/dsh"]`
- 源码：[deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness)
- 运行时包：[`@deepseek-ai/dsh@0.1.0-rc.7`](https://www.npmjs.com/package/@deepseek-ai/dsh)

运行时启动或更新该 npm 包，仓库里没有 harness 源码树。

## 功能

- **Windows / macOS / Linux：** 通过应用数据目录中的托管安装、PATH 上的 `dsh`，或 `npx @deepseek-ai/dsh` 启动 `dsh web --host <host> --port <port>`，再导航到就绪 URL。
- **Android / iOS：** 仅连接。填写可访问的 `dsh web` host 与 port。
- **设置：** 持久化 host、port、自动启动和启动模式。
- **DSH 更新：** 在应用数据前缀中执行 `npm install @deepseek-ai/dsh@latest`。这不会重编本外壳。

可选本地检出（不需要，也不是 submodule）：把 `DSH_CHECKOUT` 设为 `deepseek-ai/deepseek-harness` 的克隆，或在旁边放一个名为 `deepseek-harness` 的目录。外壳会在该树里跑 `pnpm dsh web`。

`dsh web` 拒绝 `--host 0.0.0.0`。手机无法在没有隧道的情况下访问仅回环的桌面服务器。

## 开发

需要 Node 22.19+ 与 Rust stable：

```sh
pnpm install
pnpm icons
pnpm dev
```

本地打安装包：`pnpm build`。

在 GitHub：**Actions → desktop → Run workflow**。该 run 会上传 Windows NSIS、macOS universal DMG、Linux AppImage/deb。打 `v*` 标签还会打开一份 draft Release。

## 测试

```sh
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml
```

## 上游

| 部分 | 所在位置 |
|---|---|
| Agent、工具、Web UI、`dsh web` | [`@deepseek-ai/dsh`](https://www.npmjs.com/package/@deepseek-ai/dsh) / [deepseek-harness](https://github.com/deepseek-ai/deepseek-harness) |
| 原生窗口、托盘、host/port、DSH 更新 | 本仓库 |
