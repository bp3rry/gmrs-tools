# Repeater Station (Phase R)

A parallel track that adds a **GMRS repeater** to the project: programming it, and optionally running it from a repurposed Mac mini or Linux PC acting as a **repeater controller**.

> **Status:** Planning. No hardware purchased yet. Prices and product notes were gathered 2026-11-19 and will drift.

---

## 1. Goals

1. **Program the repeater from the toolkit** (backup, decode, write settings) — same safety rules as the UV-5R work: back up first, verify after write.
2. **Run the repeater with our own controller** — station ID, timeout timer, courtesy tone, activity log — on hardware we already own.
3. **Teach while doing** — the rules engine explains repeater pairs, power limits and ID rules.

Out of scope for now: internet linking (GMRS restricts network interconnection; revisit only after checking Part 95E).

---

## 2. Chosen hardware (proposed)

| Role | Choice | Why |
|---|---|---|
| Repeater | **Retevis RT97S** | Cheapest of the line (~$340), FCC Part 95E listed (FCC ID 2ASNSRT97 — verify on fcc.gov), built-in duplexer (one antenna), and has the **DB-9 controller port** with a known-working URIxB cable. |
| Controller interface | **DMK URIxB** (~$120) + RT97S-to-URIxB cable | USB sound card with PTT output and COS (carrier detect) input. |
| Controller computer | Existing Mac mini or Linux PC, **running Debian** | Headless, auto-restart services, no macOS sleep/update surprises. |
| Antenna (initial) | Vertical UHF dipole or J-pole hoisted in a tree | Cheap, quick, ~20 ft. See §4. |
| Antenna (permanent) | Fiberglass base antenna (e.g. Tram 1486) on a mast at the shop | See §4. |

Upgrade path: **RT97L** (~$535, ~20 W measured vs ~4 W) if coverage is too small — but confirm its DB-9 port supports a controller first.

---

## 3. System diagram

### 3.1 Physical components

```
                 ┌─────────────────────┐
                 │   ANTENNA           │   tree dipole (~20 ft)  — or —
                 │   (vertical, UHF)   │   shop mast (~25–30 ft)
                 └──────────┬──────────┘
                            │  LMR-400 coax (drip loop before entry)
                            │
   ┌────────────────────────▼─────────────────────────┐
   │  LIGHTNING ARRESTOR  (at building entry)         │
   └───────┬──────────────────────────┬───────────────┘
           │ #6 copper                │ short jumper (3 ft)
     ┌─────▼──────┐                   │
     │ GROUND ROD │                   │
     │ (bonded to │                   │
     │  service   │                   │
     │  ground)   │                   │
     └────────────┘                   │
                                      │
   ┌──────────────────────────────────▼───────────────────────────────┐
   │                 RETEVIS RT97S REPEATER                           │
   │                                                                  │
   │   antenna ──► DUPLEXER ──► RECEIVER (467.xxx in)                 │
   │                  ▲              │ audio                          │
   │                  └── TRANSMITTER (462.xxx out) ◄┘                │
   │                                                                  │
   │   [DB-9 mic/programming]          [DB-9 controller port]         │
   └────────┬───────────────────────────────────┬─────────────────────┘
            │                                   │  COS out, PTT in, audio
            │ USB programming cable             │  RT97S-to-URIxB cable
            │ (CH340 / "USB READER")            │
   ┌────────▼──────────┐              ┌─────────▼──────────┐
   │ LAPTOP            │              │ DMK URIxB          │
   │ • Retevis CPS     │              │ USB sound card +   │
   │   (Windows, for   │              │ GPIO (PTT / COS)   │
   │   trace capture)  │              └─────────┬──────────┘
   │ • `gmrs` CLI      │                        │ USB
   │   (R1/R2)         │              ┌─────────▼──────────┐
   └───────────────────┘              │ CONTROLLER PC      │
                                      │ old Mac mini or    │
                                      │ Linux PC (Debian)  │
                                      │ • `gmrs-controller`│
                                      │   service (R3)     │
                                      └─────────┬──────────┘
                                                │
   ┌────────────────────┐  13.8 V DC            │ AC
   │ POWER SUPPLY       ├─────► RT97S           │
   │ + battery backup   │                ┌──────▼─────┐
   └────────────────────┘                │    UPS     │
                                         └────────────┘
```

Notes:
- The repeater repeats **on its own**. The controller adds ID, timeout, courtesy tone and logging. (To verify on the bench: the RT97S keeps repeating with a controller attached.)
- The programming laptop is only needed when changing settings; it is not part of the running station.

### 3.2 Software components

```
   ┌──────────────────────────┐        ┌────────────────────────────────┐
   │ `gmrs` CLI (existing)    │        │ `gmrs-controller` (new, R3)    │
   │ + rt97 backup/show/write │        │ long-running service           │
   └────────────┬─────────────┘        │  • COS watcher                 │
                │                      │  • ID scheduler (CW / voice)   │
                │                      │  • timeout timer, courtesy tone│
                │                      │  • activity log                │
                │                      │  • PTT safety (see §6)         │
                │                      └───────┬───────────────┬────────┘
                │                              │ hidapi        │ cpal
                │                              │ (GPIO)        │ (audio)
   ╔════════════▼══════════════════════════════▼═══════════════▼════════╗
   ║ CORE LIBRARY (Rust, zero dependencies)                             ║
   ║  • drivers: uv5r, rt97 (new)                                       ║
   ║  • rules: GMRS repeater pairs, power, station-ID intervals (new)   ║
   ║  • controller logic as pure state machine (testable, no hardware) ║
   ╚════════════════════════════════════════════════════════════════════╝
```

The controller's decision-making (when to ID, when to time out) lives in the core as plain logic with no hardware access, so it can be fully tested without a radio. Only the thin `gmrs-controller` program touches USB audio and GPIO.

---

## 4. Antenna site options

Two candidate sites. Either works; **start with the tree, move to the shop** once things are proven.

| | A. Tree at garage corner | B. Shop (30×40, ~15 ft peak) |
|---|---|---|
| Antenna height | ~20 ft | ~25–30 ft (gable-end mast, 10–15 ft above peak) |
| Antenna | Vertical dipole / roll-up J-pole hung from a rope + pulley | Fiberglass base antenna (Tram 1486 class) on mast |
| Cost | Low (~$30–80 antenna + rope) | Higher (mast, brackets, ~$60–200 extra) |
| Effort | An afternoon | A weekend; ladder/roof work |
| Durability | Wind sway, branches, re-hoisting | Permanent |
| Best for | Bench-to-field testing, first coverage check | Long-term station |

### Tree dipole tips
- **Mount it vertically.** Handhelds and mobiles are vertical; a horizontal dipole loses a lot of signal to them.
- **Keep it clear of the trunk and foliage** — a few feet out on a branch or a separate rope line. Wet leaves absorb UHF.
- **Use a halyard (rope + pulley)** so you can lower it without climbing. Leave slack for sway so the coax isn't tugged.
- **Measure the coax run** from the antenna to where the repeater will sit. At 462 MHz, 50 ft of LMR-400 loses about 1.4 dB; the same length of RG-58 loses roughly 5 dB (most of your signal).

### Shop mast tips
- A **gable-end wall bracket pair** holding a 10–20 ft mast is simpler than a roof penetration.
- If the shop has a **metal roof/siding**, that's fine — just keep the antenna's radiating section well above the roofline (several feet), and put the repeater inside with only the coax passing through.
- The shop is also a good home for the repeater, power and controller PC (shorter coax run, one grounding point).

### Height rules
Check the GMRS antenna-height rule in 47 CFR Part 95E and any FAA / local / HOA limits before going much higher than the shop mast. Both options above are modest heights.

---

## 5. Parts list

Prices are from listings found 2026-11-19; **est.** = estimate, not from a specific listing.

| # | Item | Suggested | Price | Notes |
|---|---|---|---|---|
| 1 | Repeater | [Retevis RT97S](https://www.retevis.com/products/rt97s-full-duplex-portable-gmrs-repeater) or [myGMRS bundle](https://shop.mygmrs.com/products/retevis-rt97s-repeater-bundle-with-antenna-and-coax-cable) | $339.99 | Bundle adds AC supply, programming cable, small antenna, RG-58 (use only for bench tests). |
| 2a | Antenna — tree | Vertical UHF dipole or roll-up J-pole (GMRS-tuned) | est. $30–80 | Site A. |
| 2b | Antenna — shop | [Tram 1486](https://reach.dog/shop/mygmrscom/products/tram-1486-uhf-fiberglass-base-antenna) | ~$94–110 | Site B. Upgrades: [Comet CA-GMRS](https://www.buytwowayradios.com/comet-ca-gmrs.html) (~$116), [Comet CA-712EFC](https://www.gigaparts.com/comet-ca-712efc-460-470mhz-base-repeater-antenna.html) (~$160–223). |
| 3 | Feedline | LMR-400 (or equivalent), PL-259 ends, cut to length | est. $1–2/ft | Over ~75 ft, consider ½" hardline. |
| 4 | Jumper | 3 ft LMR-240 / RG-8X | est. $15–25 | Arrestor → repeater. |
| 5 | Lightning arrestor | UHF-rated coax protector (PolyPhaser, Alpha Delta class) | est. $40–90 | At building entry, bonded to ground. |
| 6 | Grounding | 8–10 ft ground rod, #6 copper, clamps | est. $40–70 | Bond to electrical service ground — have an electrician check. |
| 7 | Mounting | Site A: rope, pulley, throw line. Site B: gable brackets + 10–20 ft mast | est. $20–200 | |
| 8 | Weatherproofing | Self-amalgamating tape + electrical tape, UV zip ties | est. $15 | |
| 9 | Power | 13.8 V supply (in bundle) + optional battery backup | $0–150 est. | |
| 10 | Controller interface | [DMK URIxB](https://reach.dog/shop/mygmrscom/products/dmk-engineering-urixb) | $119.95 | |
| 11 | Interface cable | [RT97S-to-URIxB cable](https://shop.mygmrs.com/products/retevis-rt97s-to-dmk-urixb-custom-interface-cable) | see listing | |
| 12 | Controller PC | Existing Mac mini / Linux PC + small UPS | $0 + est. $60–100 | Set "power on after power loss" in BIOS/settings. |
| 13 | License | FCC GMRS license | $35 / 10 yrs | Required. |

**Rough totals:** tree-dipole starter ≈ **$700–900**; shop-mast station ≈ **$850–1,250** (both including controller parts, excluding the PC you already own).

---

## 6. Sub-phases

### R0 — Research & bench setup
- Buy RT97S + license; bench-test with the bundled antenna.
- Capture serial traces of the Retevis programming software (needs a Windows machine) → `docs/protocol/rt97.md`, clean-room.
- Verify the DB-9 controller-port pinout on our unit (COS pin 3 / PTT pin 9, both active-low per Repeater Builder) and that the repeater keeps repeating with a controller attached.
- Confirm FCC certification on fcc.gov; read Part 95E sections on repeaters, station ID, remote control, interconnection.
- Put up the tree antenna; do a drive-around coverage check.

### R1 — Read-only programming
- Generalize the driver layer (a common radio-driver interface) before adding a second driver.
- `gmrs` CLI: `detect`, `backup`, `show` for the RT97S. Never writes.

### R2 — Write settings
- Write channel / tones / power with backup-first and read-back verification.
- Rules engine: GMRS repeater pairs (462.xxx out / 467.xxx in), power limits, explanations.

### R3 — Controller
- **R3a:** Run SVXLink (GPL, used as-is, not copied) on the Linux box to prove wiring, COS/PTT polarity and audio levels.
- **R3b:** Build `gmrs-controller`: COS monitoring, station ID, timeout timer, courtesy tone, activity log. Runs as a systemd service.
- **Safety requirements (non-negotiable):**
  - Hard PTT time limit, independent of the main logic.
  - PTT released on exit, crash or USB disconnect.
  - Watchdog: if the controller stops responding, transmitter drops.

### R4 — Permanent site (optional)
- Move antenna to the shop mast, proper grounding/arrestor, battery backup.

---

## 7. Open questions
- Does the RT97L's DB-9 port support an external controller? (Decides the upgrade path.)
- Exact GMRS station-ID and remote-control requirements for repeaters (Part 95E).
- Retevis programming protocol — unknown until traced.
- URIxB GPIO mapping on our unit (confirm against its manual).

## 8. References
- [Retevis RT97S](https://www.retevis.com/products/rt97s-full-duplex-portable-gmrs-repeater) · [RT97S vs RT97L](https://www.retevis.com/blog/rt97s-vs-rt97l-which-gmrs-repeater-is-right-for-you-) · [RT97L test](https://www.retevis.com/blog/retevis-rt97l-repeater-review-real-world-testing-for-gmrs-enthusiasts)
- [Repeater Builder: RIM Lite to RT97S pinout (PDF)](https://www.repeater-builder.com/products/RIM_pdfs/RB_RIM_Lite_to_RT97S.pdf)
- [myGMRS RT97S-to-URIxB cable](https://shop.mygmrs.com/products/retevis-rt97s-to-dmk-urixb-custom-interface-cable) · [DMK URIxB](https://reach.dog/shop/mygmrscom/products/dmk-engineering-urixb)
- [RT97/RT97S USB driver notes](https://shop.mygmrs.com/pages/retevis-rt97-and-rt97s-usb-driver-installation)
- [FCC GMRS rules (47 CFR Part 95 Subpart E)](https://www.ecfr.gov/current/title-47/chapter-I/subchapter-D/part-95/subpart-E)
