<p align="center">
  <img src="src/icon.png" alt="Mindsnap" width="128" />
</p>

<h1 align="center">Mindsnap</h1>

<p align="center">
  A lightweight desktop utility built with Tauri and Rust that tracks active window usage and reminds you to take breaks.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>
  <a href="https://tauri.app"><img src="https://img.shields.io/badge/Tauri-v2-orange.svg" alt="Tauri v2" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-2021-red.svg" alt="Rust 2021" /></a>
</p>

---

- Mindsnap runs in the background and monitors how long you spend in specific applications. When you stay on a tracked app past your configured threshold, it sends a desktop notification to help you stay aware of your screen time.

- All data stays on your machine locally. No accounts, no telemetry and no background network activity.

> [!NOTE]
> This software contains code written by artificial intelligence. However, the app owner and the testing team test the app for bugs and other issues in every release, and the code is reviewed after the artificial intelligence makes a change.

## Download

> [!CAUTION]
> The Linux version of Mindsnap is still under development. Expect broken functions or the app itself completely. You can still download the .AppImage to help us testing mindsnap. Your help is very appreciated!

- No installation is required for Mindsnap. You can find the Platform-specific executables in **[Releases](https://github.com/x0thra/mindsnap/releases)** page, download and run them right away. Or just proceed with building from the source.

## Building from Source

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/) (stable)

### Development

```bash
git clone https://github.com/x0thra/mindsnap.git
cd mindsnap
npm install
npm run dev
```

### Production Build

```bash
npm run build
```

The standalone binary will be created in `src-tauri/target/release/mindsnap.exe`.

## License

[MIT](LICENSE) © [x0thra](https://github.com/x0thra)

