# DeepSeek Harness App

<div align="center">

![DeepSeek Harness Logo](src-tauri/icons/128x128.png)

**🚀 DeepSeek Harness 高性能跨平台桌面应用**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub Release](https://img.shields.io/github/v/release/deepseek-ai/deepseek-harness-app)](https://github.com/deepseek-ai/deepseek-harness-app/releases)
[![GitHub Stars](https://img.shields.io/github/stars/deepseek-ai/deepseek-harness-app?style=social)](https://github.com/deepseek-ai/deepseek-harness-app)
[![Build Status](https://github.com/deepseek-ai/deepseek-harness-app/workflows/Build%20Desktop%20Apps/badge.svg)](https://github.com/deepseek-ai/deepseek-harness-app/actions)

[English](README.md) | [简体中文](README.zh-CN.md)

[立即下载](#-安装方式) | [核心特性](#-为什么选择-deepseek-harness-app) | [使用文档](#-文档) | [参与贡献](#-参与贡献)

</div>

---

## 📖 什么是 DeepSeek Harness App？

**DeepSeek Harness App** 是一款原生桌面应用，为 [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)（强大的 AI 智能体框架）提供流畅、高性能的交互界面。

基于 **Tauri 2.0** 和 **Rust** 构建，带来：
- ⚡ **体积缩减 10 倍**（15MB vs 150MB+）
- 🚀 **内存占用减少 6 倍**（50MB vs 300MB+）
- ⏱️ **启动速度提升 3 倍**（<1秒 vs 2-3秒）
- 🔄 **智能双层更新**：应用壳和 DSH 核心独立更新

---

## ✨ 为什么选择 DeepSeek Harness App？

### 🎯 极致性能

| 指标 | DeepSeek Harness App (Tauri) | 基于 Electron 的应用 |
|------|------------------------------|---------------------|
| **安装包体积** | ~15 MB | ~150 MB |
| **内存占用** | ~50 MB | ~300 MB |
| **启动时间** | < 1 秒 | 2-3 秒 |
| **原生体验** | ✅ 真正原生 | ❌ Web 包装器 |

### 🔄 智能更新系统

**双层更新策略：**

1. **应用壳自动更新**
   - 通过 GitHub Releases 后台静默更新
   - 重启时自动安装
   - 无需用户干预

2. **DSH 核心一键更新**
   - 独立更新 DSH，无需重新下载整个应用
   - 在设置中点击"检查更新"按钮
   - 实时显示更新进度
   - 更新后立即可用

### 🌍 真正的全平台支持

**桌面平台（已发布）：**
- 🪟 **Windows**（NSIS & MSI 安装包）
- 🍎 **macOS 10.14+**（universal DMG，支持 Intel 和 Apple Silicon）
- 🐧 **Linux**（DEB & AppImage）

**移动平台（即将推出）：**
- 📱 **Android**（APK）
- 🍏 **iOS**（IPA）

### ⚡ 开箱即用

- **无需安装 Node.js** — DSH 通过官方 npm 包运行
- **自动进程管理** — 应用自动处理 DSH 后端生命周期
- **系统托盘集成** — 常驻后台，随时唤起
- **配置持久化** — Host 和端口设置自动保存

### 🛠️ 开发者友好

- 基于官方 `@deepseek-ai/dsh` npm 包构建
- 开源 MIT 协议
- GitHub Actions 自动化构建
- 易于 fork 和定制

---

## 📥 安装方式

### 下载预构建安装包

访问我们的 [Releases 页面](https://github.com/deepseek-ai/deepseek-harness-app/releases)，下载适合您平台的安装包：

**Windows：**
```
deepseek-harness-app_1.0.2_x64_zh-CN.msi
```

**macOS：**
```
DeepSeek.Harness.App_1.0.2_universal.dmg
```

**Linux（Debian/Ubuntu）：**
```bash
sudo dpkg -i deepseek-harness-app_1.0.2_amd64.deb
```

**Linux（AppImage）：**
```bash
chmod +x deepseek-harness-app_1.0.2_amd64.AppImage
./deepseek-harness-app_1.0.2_amd64.AppImage
```

### 首次启动

1. **安装应用程序**：使用适合您操作系统的安装包
2. **启动 DeepSeek Harness App**：从应用程序菜单启动
3. **等待 DSH 初始化**：首次启动需要约 5-10 秒
4. **开始使用 DeepSeek Harness！** Web UI 会自动加载

---

## 🎮 使用方法

### 基本操作

**启动 DSH 后端：**
- 应用启动时自动启动 DSH
- 默认地址：`http://127.0.0.1:3080`

**访问 Web UI：**
- 在应用窗口中自动打开
- 所有 DeepSeek Harness 功能均可使用

**更新 DSH 核心：**
1. 点击应用中的**设置**图标
2. 点击**检查更新**按钮
3. 如果有新版本，点击**立即更新**
4. 等待安装完成
5. 重启应用以使用新版本

**配置 DSH：**
- 进入**设置** → **DSH 配置**
- 修改 **Host**（默认：`127.0.0.1`）
- 修改 **端口**（默认：`3080`）
- 切换**启动时自动运行**

**系统托盘：**
- 最小化到托盘在后台运行
- 右键托盘图标快速操作
- 关闭窗口最小化到托盘（不退出）

---

## 🏗️ 从源码构建

### 前置要求

- **Node.js** 22+ 和 **pnpm** 11+
- **Rust** 1.75+（通过 [rustup](https://rustup.rs/) 安装）
- **系统依赖：**
  - **Linux：** `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
  - **macOS：** Xcode 命令行工具
  - **Windows：** Visual Studio 2022 生成工具

### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/deepseek-ai/deepseek-harness-app.git
cd deepseek-harness-app

# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 生产构建
pnpm tauri build

# macOS universal DMG（Intel + Apple Silicon，支持 10.14+）
rustup target add aarch64-apple-darwin x86_64-apple-darwin
MACOSX_DEPLOYMENT_TARGET=10.14 pnpm tauri:build:macos
```

**构建输出位置：**
- Windows：`src-tauri/target/release/bundle/nsis/` 和 `msi/`
- macOS（本机架构）：`src-tauri/target/release/bundle/dmg/`
- macOS（universal，10.14+）：`src-tauri/target/universal-apple-darwin/release/bundle/dmg/`
- Linux：`src-tauri/target/release/bundle/deb/` 和 `appimage/`

---

## 📚 文档

- [用户指南](docs/USER_GUIDE.zh-CN.md) — 详细使用说明
- [开发指南](docs/DEVELOPMENT.zh-CN.md) — 贡献和开发环境配置
- [架构概览](docs/ARCHITECTURE.zh-CN.md) — 技术设计细节
- [常见问题](docs/FAQ.zh-CN.md) — 常见问题解答
- [更新日志](CHANGELOG.md) — 版本历史和更新记录

---

## 🤝 参与贡献

我们欢迎任何形式的贡献！包括：
- 🐛 Bug 报告
- 💡 功能建议
- 📖 文档改进
- 🔧 代码贡献

提交 PR 前请阅读我们的[贡献指南](CONTRIBUTING.zh-CN.md)。

### 开发工作流

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feature/amazing-feature`
3. 提交更改：`git commit -m 'Add amazing feature'`
4. 推送到分支：`git push origin feature/amazing-feature`
5. 提交 Pull Request

---

## 🌟 Star 历史

如果觉得这个项目有用，请给我们一个 Star！⭐

[![Star History Chart](https://api.star-history.com/svg?repos=deepseek-ai/deepseek-harness-app&type=Date)](https://star-history.com/#deepseek-ai/deepseek-harness-app&Date)

---

## 📄 开源协议

本项目采用 **MIT 协议** 开源 — 详见 [LICENSE](LICENSE) 文件。

---

## 🙏 致谢

- **[Tauri](https://tauri.app/)** — 现代、安全、轻量的桌面应用框架
- **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)** — 强大的 AI 智能体框架
- **[Rust](https://www.rust-lang.org/)** — 极速且内存安全的系统编程语言

---

## 📞 技术支持

- 🐛 **反馈问题：** [GitHub Issues](https://github.com/deepseek-ai/deepseek-harness-app/issues)
- 💬 **讨论交流：** [GitHub Discussions](https://github.com/deepseek-ai/deepseek-harness-app/discussions)
- 📧 **邮箱：** support@deepseek.ai
- 🌐 **官网：** [https://www.deepseek.ai](https://www.deepseek.ai)

---

<div align="center">

**由 DeepSeek 团队用 ❤️ 打造**

[⬆ 返回顶部](#deepseek-harness-app)

</div>
