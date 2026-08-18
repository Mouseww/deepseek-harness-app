# DeepSeek Harness App

<div align="center">

![DeepSeek Harness Logo](src-tauri/icons/128x128.png)

**🚀 High-Performance Cross-Platform Desktop Application for DeepSeek Harness**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub Release](https://img.shields.io/github/v/release/deepseek-ai/deepseek-harness-app)](https://github.com/deepseek-ai/deepseek-harness-app/releases)
[![GitHub Stars](https://img.shields.io/github/stars/deepseek-ai/deepseek-harness-app?style=social)](https://github.com/deepseek-ai/deepseek-harness-app)
[![Build Status](https://github.com/deepseek-ai/deepseek-harness-app/workflows/Build%20Desktop%20Apps/badge.svg)](https://github.com/deepseek-ai/deepseek-harness-app/actions)

[English](README.md) | [简体中文](README.zh-CN.md)

[Download](#-installation) | [Features](#-why-choose-deepseek-harness-app) | [Documentation](#-documentation) | [Contributing](#-contributing)

</div>

---

## 📖 What is DeepSeek Harness App?

**DeepSeek Harness App** is a native desktop application that provides a seamless, high-performance interface for [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) — the powerful AI agent framework.

Built with **Tauri 2.0** and **Rust**, this app delivers:
- ⚡ **10x smaller** than Electron alternatives (15MB vs 150MB+)
- 🚀 **6x less memory** usage (50MB vs 300MB+)
- ⏱️ **3x faster** startup time (<1s vs 2-3s)
- 🔄 **Smart dual-layer updates** for both app shell and DSH core

---

## ✨ Why Choose DeepSeek Harness App?

### 🎯 Extreme Performance

| Metric | DeepSeek Harness App (Tauri) | Electron-based Apps |
|--------|------------------------------|---------------------|
| **Package Size** | ~15 MB | ~150 MB |
| **Memory Usage** | ~50 MB | ~300 MB |
| **Startup Time** | < 1 second | 2-3 seconds |
| **Native Feel** | ✅ True native | ❌ Web wrapper |

### 🔄 Intelligent Update System

**Dual-Layer Update Strategy:**

1. **App Shell Auto-Update**
   - Silent background updates via GitHub Releases
   - Automatic installation on restart
   - No user intervention required

2. **DSH Core One-Click Update**
   - Update DSH independently without re-downloading the entire app
   - Click "Check for Updates" button in settings
   - Real-time progress feedback
   - Instant availability after update

### 🌍 True Cross-Platform Support

**Desktop Platforms (Available Now):**
- 🪟 **Windows** (NSIS & MSI installers)
- 🍎 **macOS 10.14+** (universal DMG for Intel & Apple Silicon)
- 🐧 **Linux** (DEB & AppImage)

**Mobile Platforms (Coming Soon):**
- 📱 **Android** (APK)
- 🍏 **iOS** (IPA)

### ⚡ Zero Configuration Required

- **No Node.js installation needed** — DSH runs via official npm package
- **Automatic process management** — App handles DSH backend lifecycle
- **System tray integration** — Always accessible, minimal intrusion
- **Persistent configuration** — Host and port settings saved automatically

### 🛠️ Developer Friendly

- Built on official `@deepseek-ai/dsh` npm package
- Open-source MIT license
- GitHub Actions automated builds
- Easy to fork and customize

---

## 📥 Installation

### Download Pre-Built Binaries

Visit our [Releases Page](https://github.com/deepseek-ai/deepseek-harness-app/releases) and download the installer for your platform:

**Windows:**
```
deepseek-harness-app_1.0.2_x64_en-US.msi
```

**macOS:**
```
DeepSeek.Harness.App_1.0.2_universal.dmg
```

**Linux (Debian/Ubuntu):**
```bash
sudo dpkg -i deepseek-harness-app_1.0.2_amd64.deb
```

**Linux (AppImage):**
```bash
chmod +x deepseek-harness-app_1.0.2_amd64.AppImage
./deepseek-harness-app_1.0.2_amd64.AppImage
```

### First Launch

1. **Install the application** using the appropriate installer for your OS
2. **Launch DeepSeek Harness App** from your applications menu
3. **Wait for DSH to initialize** (first launch takes ~5-10 seconds)
4. **Start using DeepSeek Harness!** The web UI loads automatically

---

## 🎮 Usage

### Basic Operations

**Start DSH Backend:**
- The app automatically starts DSH on launch
- Default: `http://127.0.0.1:3080`

**Access Web UI:**
- Opens automatically in the app window
- All DeepSeek Harness features available

**Update DSH Core:**
1. Click **Settings** icon in the app
2. Click **Check for Updates** button
3. If a new version is available, click **Update Now**
4. Wait for installation to complete
5. Restart the app to use the new version

**Configure DSH:**
- Go to **Settings** → **DSH Configuration**
- Modify **Host** (default: `127.0.0.1`)
- Modify **Port** (default: `3080`)
- Toggle **Auto-start on Launch**

**System Tray:**
- Minimize to tray for background operation
- Right-click tray icon for quick actions
- Close the window to minimize (not quit)

---

## 🏗️ Build from Source

### Prerequisites

- **Node.js** 22+ and **pnpm** 11+
- **Rust** 1.75+ (install via [rustup](https://rustup.rs/))
- **System dependencies:**
  - **Linux:** `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
  - **macOS:** Xcode Command Line Tools
  - **Windows:** Visual Studio 2022 Build Tools

### Build Steps

```bash
# Clone the repository
git clone https://github.com/deepseek-ai/deepseek-harness-app.git
cd deepseek-harness-app

# Install dependencies
pnpm install

# Development mode
pnpm tauri dev

# Production build
pnpm tauri build

# macOS universal DMG (Intel + Apple Silicon, 10.14+)
rustup target add aarch64-apple-darwin x86_64-apple-darwin
MACOSX_DEPLOYMENT_TARGET=10.14 pnpm tauri:build:macos
```

**Build outputs:**
- Windows: `src-tauri/target/release/bundle/nsis/` and `msi/`
- macOS (native): `src-tauri/target/release/bundle/dmg/`
- macOS (universal, 10.14+): `src-tauri/target/universal-apple-darwin/release/bundle/dmg/`
- Linux: `src-tauri/target/release/bundle/deb/` and `appimage/`

---

## 📚 Documentation

- [User Guide](docs/USER_GUIDE.md) — Detailed usage instructions
- [Development Guide](docs/DEVELOPMENT.md) — Contributing and development setup
- [Architecture Overview](docs/ARCHITECTURE.md) — Technical design details
- [FAQ](docs/FAQ.md) — Frequently asked questions
- [Changelog](CHANGELOG.md) — Version history and updates

---

## 🤝 Contributing

We welcome contributions! Whether it's:
- 🐛 Bug reports
- 💡 Feature requests
- 📖 Documentation improvements
- 🔧 Code contributions

Please read our [Contributing Guide](CONTRIBUTING.md) before submitting PRs.

### Development Workflow

1. Fork this repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Commit your changes: `git commit -m 'Add amazing feature'`
4. Push to the branch: `git push origin feature/amazing-feature`
5. Open a Pull Request

---

## 🌟 Star History

If you find this project useful, please consider giving it a star! ⭐

[![Star History Chart](https://api.star-history.com/svg?repos=deepseek-ai/deepseek-harness-app&type=Date)](https://star-history.com/#deepseek-ai/deepseek-harness-app&Date)

---

## 📄 License

This project is licensed under the **MIT License** — see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- **[Tauri](https://tauri.app/)** — Modern, secure, and lightweight desktop framework
- **[DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness)** — The powerful AI agent framework
- **[Rust](https://www.rust-lang.org/)** — Blazingly fast and memory-safe systems programming language

---

## 📞 Support

- 🐛 **Report Issues:** [GitHub Issues](https://github.com/deepseek-ai/deepseek-harness-app/issues)
- 💬 **Discussions:** [GitHub Discussions](https://github.com/deepseek-ai/deepseek-harness-app/discussions)
- 📧 **Email:** support@deepseek.ai
- 🌐 **Website:** [https://www.deepseek.ai](https://www.deepseek.ai)

---

<div align="center">

**Made with ❤️ by the DeepSeek Team**

[⬆ Back to Top](#deepseek-harness-app)

</div>
