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
| Repeater | **Retevis RT97S** (GMRS version — not the "Customizable" frequency version) | Cheapest of the line (~$340), FCC Part 95E listed (FCC ID 2ASNSRT97 — verify on fcc.gov), built-in duplexer (one antenna), and has the **DB-9 controller port** with a known-working URIxB cable. Amazon listing [B0CTKHTCWR](https://www.amazon.com/dp/B0CTKHTCWR) is this model. |
| Controller interface | **DMK URIxB** (~$120) + RT97S-to-URIxB cable | USB sound card with PTT output and COS (carrier detect) input. |
| Controller computer | Existing Mac mini or Linux PC, **running Debian** | Headless, auto-restart services, no macOS sleep/update surprises. |
| Antenna (initial) | Vertical UHF dipole or J-pole hoisted in a tree | Cheap, quick, ~20 ft. See §4. |
| Antenna (permanent) | **Comet CA-GMRS** on a 10 ft mast, gable-end wall mount on the shop | Factory-tuned for 462–468 MHz, no adjustment needed. See §4.1. |

Upgrade path: **RT97L** (~$535, ~20 W measured vs ~4 W) if coverage is too small — but confirm its DB-9 port supports a controller first.

---

## 3. System diagram

### 3.1 Physical components

```
                 ┌─────────────────────┐
                 │   ANTENNA           │   tree dipole (~20 ft)  — or —
                 │   (vertical, UHF)   │   Comet CA-GMRS, shop wall mast (~22–25 ft)
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
| Antenna height | ~20 ft | ~22–25 ft (10 ft mast on gable-end wall brackets) |
| Antenna | Vertical dipole / roll-up J-pole hung from a rope + pulley | Comet CA-GMRS on 1.5" steel mast |
| Cost | Low (~$30–80 antenna + rope) | ~$400–670 for antenna, plated wall mount, coax and grounding (§4.1) |
| Effort | An afternoon | A weekend; ladder/roof work |
| Durability | Wind sway, branches, re-hoisting | Permanent |
| Best for | Bench-to-field testing, first coverage check | Long-term station |

### Tree dipole tips
- **Mount it vertically.** Handhelds and mobiles are vertical; a horizontal dipole loses a lot of signal to them.
- **Keep it clear of the trunk and foliage** — a few feet out on a branch or a separate rope line. Wet leaves absorb UHF.
- **Use a halyard (rope + pulley)** so you can lower it without climbing. Leave slack for sway so the coax isn't tugged.
- **Measure the coax run** from the antenna to where the repeater will sit. At 462 MHz, 50 ft of LMR-400 loses about 1.4 dB; the same length of RG-58 loses roughly 5 dB (most of your signal).

### 4.1 Shop gable-end wall mount (recommended permanent site)

Side view of the gable end of the 30×40 metal shop:

```
                    ┌┐
                    ││  Comet CA-GMRS antenna
                    ││  (keep within ~5 ft of the top bracket)
                    └┤
                     │
                     │  10 ft galvanized steel mast, 1.5" OD, 16-ga or heavier
                     │◄─── coax strapped down the mast every ~2 ft
        roof peak    │
          ~15 ft  /\ │
                 /  \╞══╗  UPPER bracket — high on the gable,
                /    \  ║  through-bolted with inside backing plate
               /      \ ║
              /        \║◄── 18" standoff clears roof overhang/trim
             /          ║    and keeps the antenna off the metal wall
            /           ║
           /            ║
   eave ──/─────────── ╞╝  LOWER bracket (+ brace leg) near the eave
         │              │
         │ heavy-gauge  │ ◄── butyl/EPDM sealing washers on every bolt
         │ steel wall   │
         │ panels       │       drip loop
         │              │        ╭─╮
         │       ┌──────┴──◄─────╯ │   coax enters wall below the loop
         │       │ ARRESTOR├───────┘
         │       └────┬────┘
         │  inside    │ #6 copper ───► GROUND ROD (8–10 ft) ◄─── mast bonded here too
         │            │ 3 ft jumper
         │       ┌────▼─────┐
         │       │  RT97S   │  + power supply, controller PC
         │       └──────────┘
   ──────┴──────────────────────────── ground
```

**How it's built**

The shop is heavy-gauge steel, and fastening into the internal beams isn't practical. Each bracket is therefore **through-bolted through the wall panel and clamped between an outside plate and an inside backing plate**. The plates spread the load across several ribs so it never rests on one small patch of sheet steel.

Bracket attachment detail (cross-section through the wall, one bracket):

```
      OUTSIDE                         │ │                        INSIDE
                                      │ │
   bracket foot ──► ┌──────┐ ┌────────┤ ├────────┐
                    │      │ │ outside│ │ inside │ ◄── backing plate, 3/16"–1/4" steel,
   mast ◄═══════════╡      ├─┤ plate  │ │ plate  │     wide enough to span 2–3 panel ribs
   (18" standoff)   │      │ │        │ │        │     (e.g. ~12" × 18")
                    └──────┘ └────────┤ ├────────┘
                                      │ │
   bolt: 3/8" galvanized grade 5 ═════╪═╪═════════ flat washer + lock nut
   (4 per bracket minimum)            │ │
       EPDM-bonded washer + butyl ───►│ │◄── steel spacer blocks fill rib/valley gaps
       tape under the outside plate   │ │    so tightening doesn't crush the ribs
                                      │ │
                               steel wall panel
```

- **Plates:** use an outside plate under each bracket and an inside backing plate behind it, both 3/16"–1/4" steel, sized to span at least 2–3 panel ribs. A welding or fab shop can cut and drill matching pairs cheaply.
- **Bolts:** at least 4 per bracket, 3/8" **hot-dip galvanized** grade 5, with lock nuts. Galvanized steel matches the panels and avoids the corrosion you can get when stainless touches galvanized. If you use stainless, put nylon or EPDM washers between the dissimilar metals.
- **Ribs:** if the panel is ribbed, fill the gaps behind the plates with steel spacer blocks so the plates sit flat and tightening doesn't crush the ribs.
- **Sealing:** lay butyl tape under the outside plate, put an EPDM-bonded washer on each bolt head, and seal the bolt holes.
- **Holes:** mark the outside plate's hole pattern from the inside backing plate so the two line up. The plates make the Rohn WM18D's wood-stud hole spacing irrelevant, because you drill whatever pattern the plates need.
- **Spacing:** spread the brackets as far apart vertically as possible, the lower one near the eave and the upper one high on the gable. The wider apart they are, the smaller the wind forces on each bracket. If the antenna would sit more than ~5 ft above the top bracket, use a heavier mast.

**Option: ground-supported mast (recommended if the panel feels thin when you drill it)**

Stand the mast on the ground, on a base plate on a small concrete pad or a pipe set in concrete, and use the two wall brackets only to hold it upright. The ground then carries the weight and most of the wind load, and the wall panels only resist side sway. You need a longer mast for this: two 10 ft sections joined, or a telescoping mast, about 25–30 ft from ground to antenna base.
- **18" standoff brackets** clear the overhang and keep the antenna away from the metal wall, which can distort its pattern.
- **Coax runs down the mast,** strapped every ~2 ft, with a drip loop before it enters the wall.
- **Arrestor at the wall entry**, bonded with the mast to one ground rod, which is bonded to the electrical service ground.
- The shop is also a good home for the repeater, power and controller PC: short coax run and one grounding point.

**Mount parts**

| Part | Pick | Price | Notes |
|---|---|---|---|
| Antenna | [Comet CA-GMRS](https://www.buytwowayradios.com/comet-ca-gmrs.html), 5.5 dBi, SO-239 | ~$116 | Pre-tuned 462–468 MHz. The Tram 1486 (~$100) has more gain but is wide-band "tunable" and needs an SWR meter. Confirm the CA-GMRS clamp fits a 1.5" mast. |
| Wall brackets | [Easy Up EZ30-18](https://www.solidsignal.com/easy-up-deluxe-18-inch-y-type-antenna-wall-mount-brace-pair-ez30-18) pair | est. $30–60 | 12-ga galvanized, third brace leg; fits 1"–1.75" mast (1.25"–1.5" ideal). Heavier alternative: [Rohn WM18D](https://www.solidsignal.com/rohn-pair-of-18-wall-mounts-for-tv-radio-and-cellular-antennas-wm18d) (up to 2¾"; holes spaced for 24" wood studs). |
| Mast | 10 ft galvanized steel, 1.5" OD, 16-ga or heavier | est. $40–70 | Avoid thin swaged TV masts. |
| Main coax | LMR-400 or equivalent, **PL-259 both ends**, measured run + ~10% | est. $70–120 (50 ft) | CA-GMRS and RT97S both have SO-239 sockets. Over ~75 ft → ½" hardline. |
| Protection | UHF arrestor, 3 ft jumper, ground rod, #6 copper, clamps | est. $80–160 | |
| Sealing | Self-amalgamating tape, butyl/EPDM washers, UV zip ties | est. $15 | |
| Mounting plates | 2 pairs (inside + outside) of 3/16"–1/4" steel plates, ~12" × 18", drilled to match; steel rib spacers | est. $40–100 (fab shop) | One pair per bracket. |
| Bolts | 3/8" hot-dip galvanized grade 5 bolts, flat washers, lock nuts, EPDM-bonded washers, butyl tape | est. $20–40 | At least 4 per bracket. |
| *Optional* ground base | Mast base plate on a concrete pad, or a pipe set in concrete; extra 10 ft mast section | est. $50–120 | For the ground-supported option. |

**Mount subtotal: ~$400–670** with plates and bolts (add the optional ground base if used).

### Height rules
Check the GMRS antenna-height rule in 47 CFR Part 95E and any FAA / local / HOA limits before going much higher than the shop mast. Both options above are modest heights.

---

## 5. Parts list

Prices are from listings found 2026-11-19; **est.** = estimate, not from a specific listing.

| # | Item | Suggested | Price | Notes |
|---|---|---|---|---|
| 1 | Repeater | [Retevis RT97S](https://www.retevis.com/products/rt97s-full-duplex-portable-gmrs-repeater), [Amazon B0CTKHTCWR](https://www.amazon.com/dp/B0CTKHTCWR), or [myGMRS bundle](https://shop.mygmrs.com/products/retevis-rt97s-repeater-bundle-with-antenna-and-coax-cable) | $339.99 (Retevis) | Choose the GMRS version. Antenna and controller cable are never included; check each listing for AC supply + programming cable. Bundle's RG-58 is for bench tests only. |
| 2a | Antenna — tree | Vertical UHF dipole or roll-up J-pole (GMRS-tuned) | est. $30–80 | Site A. |
| 2b | Antenna — shop | [Comet CA-GMRS](https://www.buytwowayradios.com/comet-ca-gmrs.html) | ~$116 | Site B, pre-tuned. Alternatives: [Tram 1486](https://reach.dog/shop/mygmrscom/products/tram-1486-uhf-fiberglass-base-antenna) (~$94–110, needs tuning), [Comet CA-712EFC](https://www.gigaparts.com/comet-ca-712efc-460-470mhz-base-repeater-antenna.html) (~$160–223). |
| 3 | Feedline | LMR-400 (or equivalent), PL-259 ends, cut to length | est. $1–2/ft | Over ~75 ft, consider ½" hardline. |
| 4 | Jumper | 3 ft LMR-240 / RG-8X | est. $15–25 | Arrestor → repeater. |
| 5 | Lightning arrestor | UHF-rated coax protector (PolyPhaser, Alpha Delta class) | est. $40–90 | At building entry, bonded to ground. |
| 6 | Grounding | 8–10 ft ground rod, #6 copper, clamps | est. $40–70 | Bond to electrical service ground — have an electrician check. |
| 7 | Mounting | Site A: rope, pulley, throw line. Site B: [EZ30-18](https://www.solidsignal.com/easy-up-deluxe-18-inch-y-type-antenna-wall-mount-brace-pair-ez30-18) brackets + 10 ft 1.5" steel mast (§4.1) | est. $20–130 | |
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
- Install the Comet CA-GMRS on the shop gable-end wall mount (§4.1), arrestor and grounding, battery backup.

---

## 7. Open questions
- Shop wall panel gauge and rib profile (sets backing-plate size and spacer blocks). Decision so far: through-bolt with backing plates, not into internal beams.
- Wall-only mount or ground-supported mast?
- Coax run length from the gable end to the repeater's spot inside (sets cable length and type).
- Does the Comet CA-GMRS mount clamp fit a 1.5" OD mast?
- Does the RT97L's DB-9 port support an external controller? (Decides the upgrade path.)
- Exact GMRS station-ID and remote-control requirements for repeaters (Part 95E).
- Retevis programming protocol — unknown until traced.
- URIxB GPIO mapping on our unit (confirm against its manual).

## 8. References
- [Retevis RT97S](https://www.retevis.com/products/rt97s-full-duplex-portable-gmrs-repeater) · [RT97S vs RT97L](https://www.retevis.com/blog/rt97s-vs-rt97l-which-gmrs-repeater-is-right-for-you-) · [RT97L test](https://www.retevis.com/blog/retevis-rt97l-repeater-review-real-world-testing-for-gmrs-enthusiasts)
- [Repeater Builder: RIM Lite to RT97S pinout (PDF)](https://www.repeater-builder.com/products/RIM_pdfs/RB_RIM_Lite_to_RT97S.pdf)
- [myGMRS RT97S-to-URIxB cable](https://shop.mygmrs.com/products/retevis-rt97s-to-dmk-urixb-custom-interface-cable) · [DMK URIxB](https://reach.dog/shop/mygmrscom/products/dmk-engineering-urixb)
- [RT97S manual (Amazon ASIN B0CTKHTCWR)](https://manuals.plus/asin/B0CTKHTCWR)
- [Comet CA-GMRS](https://www.buytwowayradios.com/comet-ca-gmrs.html) · [Easy Up EZ30-18 brackets](https://www.solidsignal.com/easy-up-deluxe-18-inch-y-type-antenna-wall-mount-brace-pair-ez30-18) · [Rohn WM18D brackets](https://www.solidsignal.com/rohn-pair-of-18-wall-mounts-for-tv-radio-and-cellular-antennas-wm18d)
- [RT97/RT97S USB driver notes](https://shop.mygmrs.com/pages/retevis-rt97-and-rt97s-usb-driver-installation)
- [FCC GMRS rules (47 CFR Part 95 Subpart E)](https://www.ecfr.gov/current/title-47/chapter-I/subchapter-D/part-95/subpart-E)
