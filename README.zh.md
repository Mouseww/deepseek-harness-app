# DeepSeek Harness App

[English](README.md) | 中文

面向官方 DeepSeek Harness Web UI 的 Tauri 2 窗口。本仓库 **只有原生外壳**。Harness 是引用，不是内嵌源码：

- 钉扎：[`upstream.json`](upstream.json) 与 `package.json` 的 `upstream` 字段
- 源码：[deepseek-ai/deepseek-harness](https://github.com/deepseek-ai/deepseek-harness)
- 运行时包：[`@deepseek-ai/dsh@0.1.0-rc.7`](https://www.npmjs.com/package/@deepseek-ai/dsh)

安装包开箱即用：内置官方 Node 22 和该 npm 包。用户不必再装 Node、pnpm 或 DSH。[anywhere-labs/deepseek-harness-desktop](https://github.com/anywhere-labs/deepseek-harness-desktop) 靠 Electron 自带 Node 做到这一点；本应用保持 Tauri，改为随包附带同样的官方 Node + `@deepseek-ai/dsh`。

## 功能

- **Windows / macOS / Linux：** 打开即进入官方 Web UI。外壳在后台启动 `dsh web`，只有失败或从托盘进入时才显示设置页。点关闭会隐藏到托盘，从托盘选退出才真正退出。未打包的 `pnpm dev` 也可以用系统 Node。
- **第一次初始化：** 在 Web UI 起来之前，向 `web` 配置安装四个插件（`dsh-web-ui`、Transparent UI、better-sidebar、`dsh-visualize`）。Node 解析钩子会先从 profile 的 `node_modules` 找这些包，再回退到捆绑运行时。若某个 starter 插件写进了启动 bundles 但磁盘上没有对应包，外壳会先删掉这条 bundle，避免 `dsh web` 因 plugin tree 加载失败而起不来。
- **应用更新：** 启动几秒后检查 GitHub Releases。有新安装包时，标题栏按钮或「设置 → Desktop app」可直接下载并运行，不必再手动去网页下。
- **Android / iOS：** 仅连接。填写可访问的 `dsh web` host 与 port。
- **设置：** 持久化 host、port、自动启动和启动模式。
- **DSH 更新：** 在应用数据前缀中执行 `npm install @deepseek-ai/dsh@latest`。这不会重编本外壳。

可选本地检出（不需要，也不是 submodule）：把 `DSH_CHECKOUT` 设为 `deepseek-ai/deepseek-harness` 的克隆。外壳才会在该树里跑 `pnpm dsh web`。旁边同名目录会被忽略，避免源码树第一次编译把启动卡住。

`dsh web` 拒绝 `--host 0.0.0.0`。手机无法在没有隧道的情况下访问仅回环的桌面服务器。

## 开发

需要 Node 22.19+ 与 Rust stable：

```sh
pnpm install
pnpm icons
pnpm dev
```

本地打安装包：`pnpm build`。

在 GitHub：**Actions → desktop → Run workflow**，或推送 `v*` 标签。成功的 run 会发布 GitHub Release，附带 Windows NSIS/MSI、macOS universal DMG、Linux AppImage/deb。GitHub 自动附带的源码 zip 不是安装包。

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
