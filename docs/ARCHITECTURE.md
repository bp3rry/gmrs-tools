# Architecture

This document is the **source of truth** for how the project is built. New work should follow it; if something here needs to change, update this document first (and add an entry to the Decision Log at the bottom).

---

## 1. Technology decisions

| Area | Choice | Why |
|---|---|---|
| Core language | **Rust** | One codebase compiles natively for macOS, Windows, Linux, Android and iOS. The compiler catches whole classes of bugs before the code runs — valuable when the project owner isn't reviewing code line-by-line. |
| Serial (desktop) | `serialport` crate | Mature, cross-platform USB-serial access. |
| Command line tool | Hand-written argument parsing | Only a handful of commands; not worth a dependency. Revisit if the CLI grows large. |
| Utility scripts | Python 3, standard library only | Quick to write and read; not shipped to users. |
| Desktop GUI | **Tauri 2** (Rust backend + HTML/TypeScript UI) | Small installers (~10 MB vs ~100 MB for Electron), native packaging for all 3 desktop OSes, and the same framework can target mobile later. |
| Mobile bindings | **UniFFI** | Generates Kotlin (Android) and Swift (iOS) wrappers around the Rust core automatically. |
| Android USB | `usb-serial-for-android` (LGPL-2.1) | Supports CH340, PL2303, FTDI, CP210x cables over USB OTG. |
| iOS transport | Bluetooth LE serial bridge | iOS does not allow apps to use generic USB-serial cables. |
| Data formats | JSON (codeplugs, rules), CSV (channel lists), `.img` (raw radio image) | Human-readable, diffable, versionable. |
| Build/CI | GitHub Actions | Free builds on macOS, Windows and Linux runners. |
| License | MIT or Apache-2.0 (TBD) | Keeps the core reusable. **No GPL code is copied in.** |

### Rules we stick to
1. **All radio logic lives in the core.** Front-ends (CLI, desktop, mobile) only display and collect data — they never talk the radio protocol themselves.
2. **Transports are pluggable.** The core talks to a `Transport` interface; USB, USB-OTG and BLE are just different implementations.
3. **Back up before write. Verify after write.** No exceptions.
4. **Tests run without a radio** using captured `.img` files.
5. **Clean-room protocols.** Specs are written in `docs/protocol/` from permissive sources and our own traces; CHIRP (GPL) is a behavioral reference only.
6. **Minimal dependencies.** `core` has **zero** external crates. Each front-end adds only what it can't do without (the CLI adds just `serialport`). Adding any dependency requires a Decision Log entry saying why.
7. **Python is welcome for utilities** (one-off analysis, data conversion, trace inspection) in `tools/`, using only the Python standard library. Python never becomes part of the shipped product.

---

## 2. Logical architecture

How the pieces fit together, regardless of platform.

```
                        ┌────────────────────────────┐
                        │           USER             │
                        └─────────────┬──────────────┘
                                      │
   ┌──────────────┬───────────────────┼───────────────────┬──────────────┐
   │              │                   │                   │              │
┌──▼───┐   ┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐  ┌────▼────┐
│ CLI  │   │ Desktop App │     │ Android App │     │   iOS App   │  │ (Web —  │
│      │   │  (Tauri 2)  │     │  (Kotlin)   │     │   (Swift)   │  │  later) │
└──┬───┘   └──────┬──────┘     └──────┬──────┘     └──────┬──────┘  └─────────┘
   │              │                   │ UniFFI            │ UniFFI
   └──────────────┴─────────┬─────────┴───────────────────┘
                            │
   ╔════════════════════════▼═════════════════════════════════════╗
   ║                    CORE LIBRARY  (Rust)                      ║
   ║                                                              ║
   ║  ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐  ║
   ║  │ Radio Drivers│  │  Codeplug    │  │   Rules Engine     │  ║
   ║  │  • UV-5R     │◄─┤  Model       │─►│  • GMRS (Part 95E) │  ║
   ║  │  • (UV-5G…)  │  │  • channels  │  │  • Amateur bands   │  ║
   ║  │              │  │  • settings  │  │  • explanations    │  ║
   ║  └──────┬───────┘  └──────┬───────┘  └────────────────────┘  ║
   ║         │                 │                                  ║
   ║         │          ┌──────▼───────┐  ┌────────────────────┐  ║
   ║         │          │ Import/Export│  │ Learning Content   │  ║
   ║         │          │ JSON/CSV/img │  │ (lessons, refs)    │  ║
   ║         │          └──────────────┘  └────────────────────┘  ║
   ║  ┌──────▼──────────────────────────────────────────────┐     ║
   ║  │            Transport interface                      │     ║
   ║  └──────┬──────────────────┬──────────────────┬────────┘     ║
   ╚═════════╪══════════════════╪══════════════════╪══════════════╝
             │                  │                  │
      ┌──────▼──────┐   ┌───────▼──────┐   ┌───────▼──────┐
      │ USB Serial  │   │ USB OTG      │   │ Bluetooth LE │
      │ (desktop)   │   │ (Android)    │   │ (iOS)        │
      └──────┬──────┘   └───────┬──────┘   └───────┬──────┘
             └──────────────────┼──────────────────┘
                                │
                    ┌───────────▼───────────┐
                    │  K1 programming plug  │
                    │  ──► Baofeng radio    │
                    └───────────────────────┘
```

---

## 3. Deployment by operating system

### 3.1 macOS (primary — Phases 1–3)

```
┌─────────────────────────────── Mac ──────────────────────────────────┐
│                                                                      │
│  Install:  GMRS-Toolkit.dmg  (signed + notarized)                    │
│            or  brew install --cask gmrs-toolkit   (later)            │
│                                                                      │
│  /Applications/GMRS Toolkit.app                                      │
│  ┌────────────────────────────────────────────┐                      │
│  │  UI (WebKit view, built into macOS)        │                      │
│  │  ─────────────────────────────────────     │                      │
│  │  Rust core + serialport                    │   ~/Library/         │
│  └─────────────────────┬──────────────────────┘   Application Support│
│                        │                          /GMRS Toolkit/     │
│  /usr/local/bin/gmrs   │  (CLI, same core)          backups/*.img    │
│                        │                            codeplugs/*.json │
│               /dev/cu.usbserial-XXXX                                 │
│               /dev/cu.wchusbserial-XXXX  (CH340)                     │
│                        │                                             │
└────────────────────────┼─────────────────────────────────────────────┘
                         │  USB
                 ┌───────▼────────┐        ┌─────────────┐
                 │ USB-K1 cable   │───────►│  UV-5R(A)   │
                 │ FTDI or CH340  │        └─────────────┘
                 └────────────────┘
  Driver: FTDI built into macOS; CH340 built in on macOS 11+.
  Avoid counterfeit Prolific PL2303 cables.
```

### 3.2 Windows (Phase 3)

```
┌──────────────────────────── Windows 10/11 ───────────────────────────┐
│                                                                      │
│  Install:  GMRS-Toolkit-setup.msi   (or winget, later)               │
│                                                                      │
│  C:\Program Files\GMRS Toolkit\                                      │
│  ┌────────────────────────────────────────────┐                      │
│  │  UI (WebView2, built into Windows)         │   %APPDATA%\         │
│  │  Rust core + serialport                    │   GMRS Toolkit\      │
│  └─────────────────────┬──────────────────────┘     backups\         │
│  gmrs.exe (CLI)        │                            codeplugs\       │
│                        │                                             │
│                     COM3 / COM4 …                                    │
└────────────────────────┼─────────────────────────────────────────────┘
                         │  USB
                 ┌───────▼────────┐        ┌─────────────┐
                 │ USB-K1 cable   │───────►│  UV-5R(A)   │
                 └────────────────┘        └─────────────┘
  Driver: may need FTDI/CH340 driver from chip maker (not Baofeng).
```

### 3.3 Linux (Phase 3)

```
┌────────────────────────────── Linux ─────────────────────────────────┐
│                                                                      │
│  Install:  gmrs-toolkit.AppImage   (runs anywhere)                   │
│            gmrs-toolkit.deb        (Debian/Ubuntu)                   │
│                                                                      │
│  ┌────────────────────────────────────────────┐                      │
│  │  UI (WebKitGTK)                            │   ~/.local/share/    │
│  │  Rust core + serialport                    │   gmrs-toolkit/      │
│  └─────────────────────┬──────────────────────┘     backups/         │
│  /usr/bin/gmrs (CLI)   │                            codeplugs/       │
│                        │                                             │
│                 /dev/ttyUSB0                                         │
│     (user must be in the "dialout" group — installer explains)       │
└────────────────────────┼─────────────────────────────────────────────┘
                         │  USB
                 ┌───────▼────────┐        ┌─────────────┐
                 │ USB-K1 cable   │───────►│  UV-5R(A)   │
                 └────────────────┘        └─────────────┘
  Driver: FTDI and CH340 drivers built into the kernel.
```

### 3.4 Android (Phase 4)

```
┌──────────────────────────── Android phone ───────────────────────────┐
│                                                                      │
│  Install:  Google Play  /  F-Droid  /  direct .apk                   │
│                                                                      │
│  ┌────────────────────────────────────────────┐                      │
│  │  Kotlin UI (Jetpack Compose)               │   App storage:       │
│  │  ─────────────────────────────────────     │     backups/         │
│  │  UniFFI bindings                           │     codeplugs/       │
│  │  Rust core (compiled for ARM)              │   (+ share/export)   │
│  │  ─────────────────────────────────────     │                      │
│  │  usb-serial-for-android transport          │                      │
│  └─────────────────────┬──────────────────────┘                      │
└────────────────────────┼─────────────────────────────────────────────┘
                         │  USB-C port
                 ┌───────▼────────┐  ┌────────────────┐  ┌───────────┐
                 │ USB-C OTG      │─►│ USB-K1 cable   │─►│ UV-5R(A)  │
                 │ adapter        │  │ FTDI / CH340   │  └───────────┘
                 └────────────────┘  └────────────────┘
  Android asks "Allow app to access USB device?" on first connect.
```

### 3.5 iOS (Phase 5)

```
┌───────────────────────────── iPhone ─────────────────────────────────┐
│                                                                      │
│  Install:  App Store  (TestFlight for testing)                       │
│                                                                      │
│  ┌────────────────────────────────────────────┐                      │
│  │  Swift UI (SwiftUI)                        │   App storage +      │
│  │  ─────────────────────────────────────     │   Files app export   │
│  │  UniFFI bindings                           │                      │
│  │  Rust core (compiled for ARM)              │                      │
│  │  ─────────────────────────────────────     │                      │
│  │  CoreBluetooth (BLE) transport             │                      │
│  └─────────────────────┬──────────────────────┘                      │
└────────────────────────┼─────────────────────────────────────────────┘
                         │  Bluetooth LE  (no cable to phone)
                 ┌───────▼────────────────┐          ┌─────────────┐
                 │ BLE-to-serial bridge   │─── K1 ──►│  UV-5R(A)   │
                 │ (DIY or commercial,    │          └─────────────┘
                 │  battery or radio-pwr) │
                 └────────────────────────┘
  iOS blocks generic USB-serial cables, so Bluetooth is the path.
```

---

## 4. Repository layout (planned)

```
GMRS/
├── README.md
├── docs/
│   ├── ARCHITECTURE.md        ◄── this file
│   ├── REPEATER.md            Phase R plan, station diagram, parts list
│   ├── protocol/uv5r.md       clean-room protocol + memory map
│   ├── protocol/rt97.md       Retevis RT97S protocol (Phase R)
│   ├── rules/gmrs.md          GMRS reference (human-readable)
│   └── learn/                 lessons
├── data/
│   ├── rules/gmrs.json        channel plan, power, bandwidth (machine-readable)
│   └── radios/uv5r.json       radio capabilities + certification
├── core/                      Rust library (drivers, codeplug, rules, transport)
├── cli/                       `gmrs` command-line tool
├── controller/                `gmrs-controller` repeater service (Phase R3)
├── desktop/                   Tauri 2 app (Phase 3)
├── mobile/
│   ├── android/               (Phase 4)
│   └── ios/                   (Phase 5)
├── tools/                     Python utility scripts (stdlib only, not shipped)
├── tests/fixtures/            captured .img files from real radios
└── .github/workflows/         CI builds for macOS / Windows / Linux
```

---

## 5. Decision log

| Date | Decision | Notes |
|---|---|---|
| 2026-11-19 | Rust core + Tauri 2 desktop + UniFFI mobile | Owner is not a programmer; prioritized one codebase for all platforms and compiler-enforced safety. |
| 2026-11-19 | Clean-room protocol implementation | Avoid GPL (CHIRP) code; use MIT `baoclone` + own traces. |
| 2026-11-19 | iOS uses BLE bridge | iOS does not permit generic USB-serial access. |
| 2026-11-19 | Minimal dependencies; no `clap` | Owner asked for minimal dependencies. Core: none. CLI: `serialport` only (default features off). |
| 2026-11-19 | Python allowed for utilities only | Owner is fine with Python scripts for tooling; product stays Rust. |
| 2026-11-19 | **Proposed:** Phase R repeater track (Retevis RT97S + URIxB controller) | See [REPEATER.md](REPEATER.md). Controller logic lives in `core` as a hardware-free state machine; a separate `controller/` program does audio/GPIO. That program would need `hidapi` (GPIO) and `cpal` (audio). Those dependencies are **pending approval** in R3. `core` stays at zero dependencies. |
