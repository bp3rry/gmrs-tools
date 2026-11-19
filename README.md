# GMRS Toolkit (working title)

An independent, open-source toolkit for **learning GMRS radio basics** and **programming handheld radios** (starting with the Baofeng UV-5R family) — without depending on manufacturer software or third-party programmers like CHIRP.

> **Status:** Phase 1 started. Read-only backup of a UV-5R works against a simulated radio; not yet tested on real hardware.

## Quick start (macOS)

```sh
# one-time: build
cargo build --release

# find the programming cable
./target/release/gmrs ports

# check the radio answers (radio on, volume up, cable pushed in fully)
./target/release/gmrs detect /dev/cu.usbserial-XXXX

# save a full backup (read-only; never overwrites an existing backup)
./target/release/gmrs backup /dev/cu.usbserial-XXXX

# look inside a backup
./target/release/gmrs show <file.img>
```

Backups go to `~/Library/Application Support/GMRS Toolkit/backups/`. Turn the radio off and on after `detect`/`backup`.

Run the tests (no radio needed): `cargo test`

---

## Core concept

Most cheap handhelds are programmed with vendor Windows software or CHIRP. Both work, but they leave you dependent on someone else's update cycle, platform support, and design choices. This project aims to own the full stack:

1. **A small, well-documented core library** that knows how to talk to radios (serial "clone mode" protocol), how to represent their memory ("codeplug": channels + settings), and what the **rules** are (GMRS channel plan, power limits, bandwidth, repeater offsets).
2. **Thin front-ends** on top of that core: a CLI, a desktop app, and eventually mobile apps — all sharing the same tested logic.
3. **Learning built in.** The rules engine doubles as a teacher: it explains *why* a channel is configured the way it is, and warns when a configuration isn't legal.

### Design principles

- **No runtime dependency on vendors or CHIRP.** Protocols are implemented from our own documentation and from traces/permissively licensed references.
- **Clean-room protocol work.** CHIRP is GPLv3; we may *read* it to understand behavior, but we don't copy its code. Permissive references (e.g. MIT-licensed `baoclone`) and our own serial traces are the primary sources.
- **Safety first.** Always back up the radio's full factory image before the first write. Writes are verified by read-back.
- **Mac first, cross-platform by design.** macOS is the primary target; Linux and Windows follow from the same codebase; Android, then iOS.
- **Plain data formats.** Codeplugs import/export as JSON and CSV so they're diffable, versionable, and editable by hand.

### Important legal note

The Baofeng **UV-5R / UV-5RA are not FCC Part 95E certified**, so they may not legally transmit on GMRS (even with a GMRS license). They can be used to *receive* GMRS, and to transmit on amateur bands with an amateur license. GMRS-certified alternatives include the Baofeng **UV-5G** (FCC ID 2AN62-UV5G) — verify any radio in the [FCC ID database](https://www.fcc.gov/oet/ea/fccid). The toolkit tracks each radio's certification and warns accordingly.

---

## Architecture (proposed)

```
┌──────────────────────────────────────────────────────────┐
│                     Front-ends                            │
│  CLI  │  Desktop (macOS/Win/Linux)  │  Android  │  iOS    │
└───┬───────────────┬───────────────────┬───────────┬──────┘
    │               │                   │           │
┌───▼───────────────▼───────────────────▼───────────▼──────┐
│                       Core library                        │
│  • Radio drivers (UV-5R clone protocol, more later)       │
│  • Codeplug model (channels, settings, raw image)         │
│  • Rules engine (GMRS / amateur band plans, validation)   │
│  • Import/export (JSON, CSV, raw .img)                    │
└───┬──────────────────────────────────────────────────────┘
    │  Transport abstraction
┌───▼──────────────────────────────────────────────────────┐
│  USB serial (desktop) │ USB OTG (Android) │ BLE (iOS)    │
└──────────────────────────────────────────────────────────┘
```

**Stack (decided):** Rust core (`serialport` crate), CLI via `clap`, desktop GUI via Tauri 2, mobile bindings via UniFFI (Kotlin/Swift). Android USB via `usb-serial-for-android`.

See **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** for the full architecture, per-OS deployment diagrams, and the decision log. That document is the source of truth — changes to the architecture are made there first.

---

## Phases

### Phase 0 — Research & specs
- Document the UV-5R clone-mode protocol and memory map in our own words (`docs/protocol/uv5r.md`).
- Capture serial traces from a real radio to validate the spec.
- Compile the GMRS rules reference (22 channels, power limits, bandwidth, repeater offsets, CTCSS/DCS).
- Choose and confirm the tech stack.
- Hardware: obtain a programming cable with a **genuine FTDI or CH340** chip (counterfeit PL2303 cables are a common macOS failure).

### Phase 1 — Core library + CLI (macOS)
- Serial transport, UV-5R handshake, full image read/write with verification.
- Decode/encode channels and key settings.
- CLI commands: `detect`, `backup`, `read`, `write`, `diff`, `export`, `import`, `validate`.
- Test suite built on captured radio images (no radio needed for most tests).

### Phase 2 — Learning content & rules engine
- GMRS and amateur band plans as data.
- Validation with explanations ("Channel 9 is limited to 0.5 W and narrowband").
- Starter codeplugs (e.g. "GMRS 22 + local repeaters") and guided tutorials.

### Phase 3 — Desktop app & packaging
- Spreadsheet-style channel editor, backup/restore, validation panel.
- Packaging: signed & notarized macOS `.dmg` (Homebrew cask later), Windows `.msi`, Linux AppImage/`.deb`.
- CI builds for all three platforms.

### Phase 4 — Android
- Reuse the core via bindings; USB OTG cable support.
- Field-friendly UI: quick channel edits, backups, reference cards.

### Phase 5 — iOS
- iOS does not allow generic USB-serial access, so programming requires a **Bluetooth LE serial bridge** to the radio's K1 port (DIY or commercial).
- Reuse the core via Swift bindings.

### Phase R — Repeater station (parallel track)
Add a GMRS repeater (proposed: **Retevis RT97S**) and run it from a repurposed Mac mini / Linux PC. Full plan, system diagram, antenna site options and parts list: **[docs/REPEATER.md](docs/REPEATER.md)**.
- **R0** Research & bench setup: buy hardware, trace the Retevis programming protocol, verify the controller-port pinout, put up a tree-mounted dipole (~20 ft).
- **R1** Read-only programming: `gmrs` backup/show for the RT97S (after Phase 1 is proven on real hardware).
- **R2** Write settings with verification; repeater rules (462/467 MHz pairs, power, station ID).
- **R3** Controller: prove wiring with SVXLink, then build `gmrs-controller` (ID, timeout, courtesy tone, logging) with hard PTT safety limits.
- **R4** Permanent site: Comet CA-GMRS on a 10 ft mast, gable-end wall mount on the metal shop (~22–25 ft), grounding and battery backup.

### Later / ideas
- Additional radios (UV-5G, other Baofeng/BTECH models, Quansheng UV-K5).
- Repeater directory import (subject to each directory's terms of use).
- Web version for Chromium browsers (Web Serial).

---

## References

- [baoclone](https://github.com/sergev/baoclone) — MIT, UV-5R clone tool (primary protocol reference)
- [uv5rtool](https://github.com/bogde/uv5rtool) — minimal UV-5R dump/upload tool
- [CHIRP](https://chirpmyradio.com) — GPLv3; behavioral reference only
- [Miklor UV-5R resources](https://www.miklor.com/UV5R/UV5R-Software.php) — firmware versions, cables, menus
- [usb-serial-for-android](https://github.com/mik3y/usb-serial-for-android) — LGPL-2.1, Android USB serial
- [FCC GMRS rules (47 CFR Part 95 Subpart E)](https://www.ecfr.gov/current/title-47/chapter-I/subchapter-D/part-95/subpart-E)

## License

TBD (MIT or Apache-2.0 suggested, to keep the core reusable).
