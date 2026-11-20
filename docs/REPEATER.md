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
| Antenna (permanent) | **Comet CA-GMRS** on a ground-supported 1-1/4" steel pipe mast at the shop gable end, antenna tip ~25 ft | Factory-tuned for 462–468 MHz, no adjustment needed. See §4.1. |

Upgrade path: **RT97L** (~$535, ~20 W measured vs ~4 W) if coverage is too small — but confirm its DB-9 port supports a controller first.

---

## 3. System diagram

### 3.1 Physical components

```
                 ┌─────────────────────┐
                 │   ANTENNA           │   tree dipole (~20 ft)  — or —
                 │   (vertical, UHF)   │   Comet CA-GMRS, shop ground-supported mast (~25 ft)
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
| Antenna height | ~20 ft | **~25 ft at the antenna tip** (about 10 ft above the peak) |
| Antenna | Vertical dipole / roll-up J-pole hung from a rope + pulley | Comet CA-GMRS on a ground-supported steel pipe mast |
| Cost | Low (~$30–80 antenna + rope) | ~$470–830 for antenna, mast, base, brackets, coax and grounding (§4.1) |
| Effort | An afternoon | A weekend: concrete pad, then mast raising (2–3 people) |
| Durability | Wind sway, branches, re-hoisting | Permanent |
| Best for | Bench-to-field testing, first coverage check | Long-term station |

### Tree dipole tips
- **Mount it vertically.** Handhelds and mobiles are vertical; a horizontal dipole loses a lot of signal to them.
- **Keep it clear of the trunk and foliage** — a few feet out on a branch or a separate rope line. Wet leaves absorb UHF.
- **Use a halyard (rope + pulley)** so you can lower it without climbing. Leave slack for sway so the coax isn't tugged.
- **Measure the coax run** from the antenna to where the repeater will sit. At 462 MHz, 50 ft of LMR-400 loses about 1.4 dB; the same length of RG-58 loses roughly 5 dB (most of your signal).

### 4.1 Shop: ground-supported mast (chosen permanent site)

**Decision:** the mast stands on its own concrete base at ground level. Two wall brackets on the gable end only hold it upright. The ground carries the weight and most of the wind load, so the steel wall panels only resist side sway.

**Target geometry**

| Point | Height above ground |
|---|---|
| Mast base (on concrete pad) | 0 ft |
| Lower wall bracket | ~7 ft |
| Upper wall bracket (just below the peak) | ~14 ft |
| Roof peak | ~15 ft |
| Mast top / antenna base | 25 ft **minus the antenna's length** (≈21 ft for an antenna of ~4 ft) |
| Antenna tip | **~25 ft** (about 10 ft above the peak) |

Front view of the gable end, mast centered at the peak:

```
                              ┌┐  ◄── antenna tip ≈ 25 ft
                              ││
                              ││  Comet CA-GMRS antenna
                              ││
                              └┤  ◄── mast top ≈ 21 ft  (25 ft − antenna length)
                               │
                               │  1-1/4" schedule 40 galvanized steel pipe
                               │  (1.66" OD), one 21 ft stick, no joints
                               │◄── coax strapped to the mast every ~2 ft
                               │
                    peak ~15 ft│
                             /\│
                            /  ╞═══  UPPER bracket ~14 ft (steadies the mast only)
                           /   │  \
                          /    │   \
                         /     │    \
                        /      │     \
                       /       │      \
                      /        │       \
                     /         ╞═══     \   LOWER bracket ~7 ft
                    /          │         \
          eave ────┤           │          ├──── eave
                   │           │          │
                   │  gable    │          │   heavy-gauge steel wall panels;
                   │  end wall │          │   brackets through-bolted with
                   │           │          │   inside backing plates (detail below)
                   │           │          │
                   │           │ drip loop│
                   │       ╭───┤          │
                   │  ┌────┴─┐ │          │
   inside: RT97S ◄─┼──┤ARRST.│ │          │
   power, PC       │  └──┬───┘ │          │
                   │     │     │          │
   ════════════════╧═════╪═════╪══════════╧════════ ground
                         │  ┌──┴──┐
             #6 copper ──┴──┤ PAD ├──► GROUND ROD (8–10 ft), mast base and
                            └─────┘    arrestor bonded to it, and to service ground
                  concrete pad with base plate
```

**Mast**
- **Use steel pipe, not TV mast.** 1-1/4" schedule 40 galvanized pipe (1.66" OD) is much stiffer than thin-wall swaged mast, which matters with ~7 ft of mast plus the antenna above the top bracket.
- **Pipe comes in 21 ft sticks**, which matches the ≈21 ft mast-top target, so there's no joint. Cut it to **25 ft minus the antenna's length** once you know the CA-GMRS's actual length. Supply houses can cut it to length. Plan a trailer or a long truck bed to haul it.
- **If it flexes too much in wind**, step up to 1-1/2" schedule 40 (1.90" OD) with Rohn WM18D brackets. The EZ30-18 brackets top out at 1.75".

**Base**
- Pour a small concrete pad, about 2 ft × 2 ft × 2 ft deep or deeper below your frost line, with anchor bolts for a **mast base plate**, or set a short pipe sleeve in it.
- Put the base **about 18" out from the wall**, so the mast lines up with the bracket standoff and stands plumb.
- Put the ground rod next to the pad and bond the mast base to it.

**Wall brackets (steadying only)**
- **Brackets:** two [Easy Up EZ30-18](https://www.solidsignal.com/easy-up-deluxe-18-inch-y-type-antenna-wall-mount-brace-pair-ez30-18) brackets, 18" standoff, fit 1"–1.75" mast. The 1.66" pipe is near their upper limit, so check the U-bolt fit.
- **Placement:** set the lower one at ~7 ft and the upper one just below the peak at ~14 ft, on the centerline of the gable end.
- **Fastening:** still through-bolt with backing plates (detail below). The loads are smaller now, so ~8" × 12" plates are enough.

**Raising it**
- Stand the mast with 2–3 people and the antenna and coax already attached, then clamp it into the brackets from a ladder.
- Plan raising it as if it could fall, and keep it well clear of power lines.

**Coax and protection**
- Run the coax down the mast, strapped every ~2 ft, with a drip loop just before it enters the wall.
- Put the arrestor at the wall entry, and bond it and the mast to the same ground rod. That rod is in turn bonded to the electrical service ground; have an electrician check this.
- The shop is also a good home for the repeater, power and controller PC: short coax run and one grounding point.

Bracket attachment detail (cross-section through the wall, one bracket):

```
      OUTSIDE                         │ │                        INSIDE
                                      │ │
   bracket foot ──► ┌──────┐ ┌────────┤ ├────────┐
                    │      │ │ outside│ │ inside │ ◄── backing plate, 3/16"–1/4" steel,
   mast ◄═══════════╡      ├─┤ plate  │ │ plate  │     spanning 2–3 panel ribs
   (18" standoff)   │      │ │        │ │        │     (~8" × 12" for a steadying bracket)
                    └──────┘ └────────┤ ├────────┘
                                      │ │
   bolt: 3/8" galvanized grade 5 ═════╪═╪═════════ flat washer + lock nut
   (4 per bracket minimum)            │ │
       EPDM-bonded washer + butyl ───►│ │◄── steel spacer blocks fill rib/valley gaps
       tape under the outside plate   │ │    so tightening doesn't crush the ribs
                                      │ │
                               steel wall panel
```

- **Bolts:** 3/8" **hot-dip galvanized** grade 5 with lock nuts, at least 4 per bracket. Galvanized matches the panels; if you use stainless, isolate it with nylon or EPDM washers.
- **Ribs:** fill the gaps behind the plates with steel spacers so the plates sit flat.
- **Sealing:** butyl tape under the outside plate, an EPDM-bonded washer on each bolt, and seal the holes.
- **Holes:** drill the inside and outside plates as a matched pair so the holes line up.

**Mount parts**

| Part | Pick | Price | Notes |
|---|---|---|---|
| Antenna | [Comet CA-GMRS](https://www.buytwowayradios.com/comet-ca-gmrs.html), 5.5 dBi, SO-239 | ~$116 | Pre-tuned 462–468 MHz. Its length sets the mast cut. Confirm its clamp fits 1.66" OD. Alternative: Tram 1486 (~$100, needs SWR tuning). |
| Mast | 1-1/4" schedule 40 galvanized steel pipe (1.66" OD), one 21 ft stick, cut to 25 ft − antenna length | est. $60–120 | Heavier option: 1-1/2" sch 40 (1.90" OD) with Rohn WM18D brackets. |
| Base | Mast base plate + anchor bolts, or a pipe sleeve; ~3–4 bags concrete mix | est. $50–120 | Pad ~2 ft × 2 ft × 2 ft+ (below frost line). |
| Wall brackets | [Easy Up EZ30-18](https://www.solidsignal.com/easy-up-deluxe-18-inch-y-type-antenna-wall-mount-brace-pair-ez30-18) pair | est. $30–60 | Steadying only. Alternative: [Rohn WM18D](https://www.solidsignal.com/rohn-pair-of-18-wall-mounts-for-tv-radio-and-cellular-antennas-wm18d). |
| Mounting plates | 2 pairs (inside + outside) of 3/16"–1/4" steel plates, ~8" × 12", drilled to match; steel rib spacers | est. $30–80 (fab shop) | One pair per bracket. |
| Bolts | 3/8" hot-dip galvanized grade 5 bolts, washers, lock nuts, EPDM-bonded washers, butyl tape | est. $20–40 | At least 4 per bracket. |
| Main coax | LMR-400 or equivalent, **PL-259 both ends**, measured run + ~10% | est. $70–120 (50 ft) | CA-GMRS and RT97S both have SO-239 sockets. Over ~75 ft → ½" hardline. |
| Protection | UHF arrestor, 3 ft jumper, ground rod, #6 copper, clamps | est. $80–160 | |
| Sealing | Self-amalgamating tape, UV zip ties / stainless straps | est. $15 | |

**Mount subtotal: ~$470–830.**

### 4.2 Alternative: DIY two-piece mast (conduit/pipe base + manufactured top mast)

Same base, brackets, heights and antenna as §4.1. Only the mast changes. It's built in two pieces:

- **Lower section (DIY):** about 15 ft of galvanized conduit or pipe, from the concrete base to just above the upper bracket. Both wall brackets clamp this piece, so it is braced along its whole length.
- **Upper section (manufactured):** a 10 ft heavy-wall antenna mast that **slides down inside** the lower section, overlapping about 3 ft, and is through-bolted. This piece holds the antenna above the roof, where the wind load is.

```
                  ┌┐ ◄── antenna tip ≈ 25 ft
                  └┤
                   │  UPPER: manufactured antenna mast,
                   │  1-1/4" OD heavy-wall (16-ga or heavier), 10 ft
                   │  ~7 ft exposed above the joint
                   │
       ┌───────────┼───────────┐ ◄── top of lower section ≈ 15 ft
       │  ┌─────┐  │  ║        │     (cap gap with sealant / boot)
       │  │bolt ├──┼──╫──┐     │
       │  └─────┘  │  ║  │     │     JOINT DETAIL
  peak │           │  ║  │ 3 ft│     • upper mast slides inside lower pipe
 ~15 ft│ ══════════╪══╬══╪═══  │ ◄── UPPER bracket (~14 ft) clamps the
       │           │  ║  │     │     lower pipe right at the overlap
       │  ┌─────┐  │  ║  │     │     • 2 through-bolts, 3/8" galvanized,
       │  │bolt ├──┼──╫──┘     │       ~2.5 ft apart, at 90° to each other
       │  └─────┘  │  ║        │     • shim any slop with galvanized sheet
       └───────────┼──╨────────┘       so it doesn't rattle
                   │  ▲ bottom of upper mast ≈ 12 ft
                   │
                   │  LOWER: DIY galvanized conduit/pipe, ~15 ft,
                   │  ID must fit the upper mast's OD snugly
     ══════════════╪══════  LOWER bracket (~7–10 ft; place it over any coupling)
                   │
                   │  ◄── 1/4" drain hole just above the base plate
                ┌──┴──┐
                │ PAD │  concrete base + base plate, ground rod
                └─────┘
```

**Choosing the two pieces (they must nest)**

| Lower section | OD / ID | Upper mast that fits inside | Notes |
|---|---|---|---|
| **1-1/2" EMT** (galvanized thin-wall conduit) | 1.74" / ~1.61" | 1-1/4" OD (or 1-1/2" OD if it measures ≤1.60") | Cheapest and lightest. **Only acceptable because both brackets brace it.** Never let EMT stick out above the top bracket. The 1.74" OD just fits EZ30-18 brackets (max 1.75"). |
| **1-1/2" rigid conduit (RMC)** or **1-1/2" schedule 40 pipe** | 1.90" / ~1.61" | 1-1/4" OD (or 1-1/2" OD if it measures ≤1.60") | Much stronger. Threaded couplings are sturdy. The 1.90" OD needs Rohn WM18D brackets. |

Nominal pipe and mast sizes aren't actual measurements. **Measure the upper mast's OD and the lower pipe's ID with calipers before buying both.** A sliding fit with less than ~1/16" of slop is ideal; shim anything looser.

**Build notes**
- **Lengths:** conduit comes in 10 ft sticks, so a 15 ft lower section needs a coupling. Put the coupling at the lower bracket so the bracket supports it. Threaded RMC couplings are better than EMT set-screw couplings, and for EMT use compression couplings, not set-screw ones.
- **Raising it in stages:** raise the lower section and clamp it into both brackets first. It's a short, light, one-or-two-person job. Then, from a ladder at the peak, lower the upper mast with the antenna and coax attached into the socket and bolt it. That's easier and safer than standing a 21 ft one-piece mast.
- **Grounding:** couplings and the bolted joint aren't reliable electrical paths, so run a #6 copper bonding wire from the upper mast, across the joint and couplings, to the ground rod.
- **Water:** seal the top of the joint (sealant or a rubber boot) and drill a 1/4" drain hole just above the base plate so the pipe can't fill with water and freeze.
- **Don't use PVC conduit.** It's too flexible and can't be grounded.

**Cost compared with §4.1**

| Mast option | Est. cost | Comment |
|---|---|---|
| §4.1 one 21 ft stick, 1-1/4" sch 40 pipe | $60–120 | One piece, but awkward to haul and raise. |
| §4.2 two 10 ft sticks of 1-1/2" EMT + coupling + 10 ft heavy-wall mast | $70–130 | Similar cost, much easier to haul and raise in stages. |
| §4.2 with 1-1/2" RMC/sch 40 lower + 10 ft mast | $90–170 | Strongest; Rohn WM18D brackets. |

The money saving is small; most of the mount cost is the antenna, coax and grounding. The real gains are **handling and safety**: everything fits in a pickup, and nothing longer than 10 ft has to be raised. Use any pipe or conduit you already have, provided it meets the fit and strength notes above, and the saving gets bigger.

All prices in this table are estimates, not from listings.

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
| 7 | Mounting | Site A: rope, pulley, throw line. Site B: [EZ30-18](https://www.solidsignal.com/easy-up-deluxe-18-inch-y-type-antenna-wall-mount-brace-pair-ez30-18) steadying brackets + 21 ft 1-1/4" sch 40 pipe mast on a concrete base (§4.1) | est. $20–300 | |
| 8 | Weatherproofing | Self-amalgamating tape + electrical tape, UV zip ties | est. $15 | |
| 9 | Power | 13.8 V supply (in bundle) + optional battery backup | $0–150 est. | |
| 10 | Controller interface | [DMK URIxB](https://reach.dog/shop/mygmrscom/products/dmk-engineering-urixb) | $119.95 | |
| 11 | Interface cable | [RT97S-to-URIxB cable](https://shop.mygmrs.com/products/retevis-rt97s-to-dmk-urixb-custom-interface-cable) | see listing | |
| 12 | Controller PC | Existing Mac mini / Linux PC + small UPS | $0 + est. $60–100 | Set "power on after power loss" in BIOS/settings. |
| 13 | License | FCC GMRS license | $35 / 10 yrs | Required. |

**Rough totals:** tree-dipole starter ≈ **$700–900**; shop ground-mast station ≈ **$1,000–1,500** (both including controller parts, excluding the PC you already own).

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
- Pour the mast base, raise the ground-supported mast with the Comet CA-GMRS at the shop gable end (§4.1), arrestor and grounding, battery backup.

---

## 7. Open questions
- Shop wall panel gauge and rib profile (sets backing-plate size and spacer blocks). Decision so far: through-bolt with backing plates, not into internal beams.
- Mast build: one-piece pipe (§4.1) or DIY two-piece (§4.2)? If §4.2, caliper-check that the upper mast nests in the lower pipe.
- Exact Comet CA-GMRS length (sets the mast cut: 25 ft − antenna length) and local frost depth (sets pad depth).
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
