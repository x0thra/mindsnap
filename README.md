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

Mindsnap runs in the background and monitors how long you spend in specific applications. When you stay on a tracked app past your configured threshold, it sends a desktop notification to help you stay aware of your screen time.

All data stays on your machine locally. No accounts, no telemetry, no background network activity.

## Features

- **Active Focus Tracking:** Counts elapsed time when a tracked window is focused.
- **Configurable Thresholds:** Set custom initial alert limits (e.g. 5 or 10 minutes) and repeat intervals.
- **Timer Freeze on Switch:** Switching to another window pauses the timer; returning resumes it. Closing the tracked app resets the session.
- **Desktop Notifications & Sound:** Native OS notifications with optional sound alerts.
- **System Tray:** Minimizes to the system tray to run quietly in the background.
- **Localization:** English and Turkish with automatic system language detection.

## Download

Download the portable executable (`mindsnap.exe`) from the [Releases](https://github.com/x0thra/mindsnap/releases) tab. No installation required.

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

