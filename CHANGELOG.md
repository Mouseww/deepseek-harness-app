# Changelog

All notable changes to DeepSeek Harness App will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-08-18

### Added
- 🎉 Initial release of DeepSeek Harness App
- ✅ Cross-platform desktop support (Windows, macOS, Linux)
- ✅ Tauri 2.0 based architecture for high performance
- ✅ Automatic DSH backend management via `npx @deepseek-ai/dsh web`
- ✅ Dual-layer update system:
  - App shell auto-update via GitHub Releases
  - DSH core one-click update via npm
- ✅ Configuration management (host, port, auto-start)
- ✅ System tray integration
- ✅ Real-time update progress feedback
- ✅ Persistent configuration storage
- ✅ GitHub Actions automated builds for all platforms
- ✅ Complete English and Chinese documentation

### Technical Details
- Package size: ~15MB (vs 150MB+ for Electron)
- Memory usage: ~50MB (vs 300MB+ for Electron)
- Startup time: <1 second (vs 2-3s for Electron)
- Built with Rust 2021 + Tauri 2.0 + TypeScript 6

### Supported Platforms
- Windows 10/11 (x64) - NSIS & MSI installers
- macOS 11+ (Intel & Apple Silicon) - DMG
- Linux (Ubuntu 22.04+, Debian) - DEB & AppImage

[Unreleased]: https://github.com/deepseek-ai/deepseek-harness-app/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/deepseek-ai/deepseek-harness-app/releases/tag/v1.0.0
