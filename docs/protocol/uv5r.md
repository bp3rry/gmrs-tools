# Baofeng UV-5R / UV-5RA — Clone Protocol & Memory Map

**Status:** Draft v0.1 (2026-11-19). Verified against a sample factory image; **not yet verified against our own radio** (pending cable + serial trace — see §8).

**Sources (clean-room):**
- `baoclone` by Serge Vakulenko, KK6ABQ — BSD-3-Clause license. Protocol and layout facts were read from `uv-5r.c`, `radio.c`, `util.c` and described here in our own words. No code is copied.
- `baoclone/examples/uv-5r-factory.img` — a real factory image (firmware BFB291) used to confirm every offset marked ✅.
- CHIRP (GPLv3) was **not** used as a source for this document.

Legend: ✅ confirmed in sample image · 🔶 from source code only, not yet confirmed · ❓ open question

---

## 1. The big picture

The UV-5R has no "remote control" protocol. The only thing a computer can do is **clone mode**:

1. Wake the radio up with a "magic" handshake.
2. **Read** its entire memory (~6 KB) in small blocks, or
3. **Write** blocks back.

The computer then edits the memory image offline. There's no checksum and no encryption — the radio trusts whatever bytes it's given, which is exactly why we must be careful (backup first, verify after).

```
   Computer                                   Radio (powered on, in normal mode)
      │                                            │
      │  1. HANDSHAKE                              │
      │──── magic (7 bytes) ──────────────────────►│
      │◄──────────────────────────────── 0x06 (ACK)│
      │──── 0x02 ("who are you?") ────────────────►│
      │◄──────────────────────── 8-byte identifier │
      │──── 0x06 ─────────────────────────────────►│
      │◄─────────────────────────────── 0x06 (ACK) │  radio is now in clone mode
      │                                            │  (display usually goes blank/frozen)
      │  2a. READ  (repeat for each 64-byte block) │
      │──── 'S' addrHi addrLo 0x40 ───────────────►│
      │◄─────────── 'X' addrHi addrLo 0x40 + 64 B  │
      │──── 0x06 ─────────────────────────────────►│
      │◄─────────────────────────────── 0x06 (ACK) │
      │                                            │
      │  2b. WRITE (repeat for each 16-byte block) │
      │──── 'X' addrHi addrLo 0x10 + 16 bytes ────►│
      │◄─────────────────────────────── 0x06 (ACK) │
      │                                            │
      │  3. Done — power-cycle the radio           │
```

---

## 2. Serial settings 🔶

| Setting | Value |
|---|---|
| Baud rate | 9600 |
| Data / parity / stop | 8 data bits, no parity, 1 stop bit (8N1) |
| Flow control | None |
| Read timeout | ~200 ms per read (baoclone) |
| Connector | Kenwood-style 2-pin "K1" plug (3.5 mm + 2.5 mm) |

Radio must be **on**, volume up, on a quiet channel. The cable must be pushed in fully (a common failure is a plug that isn't fully seated).

---

## 3. Handshake 🔶

### 3.1 Magic strings
Two known magic strings for this family. Try the newer one first, then the older one; retry the whole sequence up to ~10 times with ~500 ms between attempts.

| Name | Bytes (hex) | Used by |
|---|---|---|
| "291" | `50 BB FF 20 12 07 25` | Firmware BFB291 and later (most UV-5R / UV-5RA in circulation) |
| "aged" | `50 BB FF 01 25 98 4D` | Old firmware (before BFB291) |

❓ **Timing:** baoclone sends the 7 magic bytes in one write. Some other tools are reported to send them one byte at a time with a ~10 ms gap, which some radios require. Our implementation should support both and we'll determine what our radio needs from a trace.

### 3.2 Steps
1. Flush any pending input.
2. Send the magic → expect `06`.
3. Send `02` → expect 8 bytes: the **identifier**.
4. Send `06` → expect `06`. Radio is now in clone mode.

### 3.3 Identifier ✅
Sample identifier: `AA 36 74 04 00 05 20 DD`

It begins with `AA` and ends with `DD`. The middle bytes appear to encode band limits (`36 74` ≈ 136–174 MHz, `04 00 05 20` ≈ 400–520 MHz). ❓ Exact meaning unconfirmed; we store it but don't rely on it.

---

## 4. Read and write commands 🔶

### 4.1 Read block — `S`
```
Send:     53  AH  AL  LEN          ('S', address high, address low, length = 0x40)
Receive:  58  AH  AL  LEN  [LEN bytes of data]
Send:     06
Receive:  06
```
- Reply header must echo `X` (0x58), the same address, and the same length — otherwise abort.
- Quirk: some radios (BF-F8HP) send the *previous* block's `06` ACK late, so the reply may start with a stray `06`. If the first byte is `06`, discard it and read one more byte.
- Quirk: the final `06` after a block may arrive late or not at all on some models; treat a missing final ACK as OK, a *wrong* byte as an error.

### 4.2 Write block — `X`
```
Send:     58  AH  AL  10  [16 bytes of data]
Receive:  06
```

### 4.3 What gets read and written

```
Address  0x0000 ┌──────────────────────────────┐ ─┐
                │                              │  │
                │       MAIN BLOCK             │  │ read in 96 × 64-byte blocks
                │  (channels, names, VFOs,     │  │ write in 384 × 16-byte blocks
                │   settings)                  │  │
         0x1800 ├──────────────────────────────┤ ─┘
                │  (gap — never read/written)  │
         0x1EC0 ├──────────────────────────────┤ ─┐
                │       AUX BLOCK              │  │ read in 5 × 64-byte blocks
                │  (messages, firmware string, │  │ write in 20 × 16-byte blocks
                │   band limits)               │  │ (only on firmware ≥ BFB291)
         0x2000 └──────────────────────────────┘ ─┘
```

Old ("aged") firmware: main block only.

---

## 5. Image file format (`.img`) ✅

Compatible with baoclone (and, by design, the common community format):

| File offset | Length | Contents |
|---|---|---|
| 0 | 8 | Radio identifier (from handshake) |
| 8 | 0x1800 (6144) | Main block, radio addresses 0x0000–0x17FF |
| 0x1808 | 0x140 (320) | Aux block, radio addresses 0x1EC0–0x1FFF (absent for aged firmware) |

Total: **6472 bytes** (6152 for aged firmware).

---

## 6. Memory map

### 6.1 Overview

| Address | Size | Contents | Status |
|---|---|---|---|
| 0x0000 | 128 × 16 | Channel records 0–127 | ✅ |
| 0x0CAA | 5 | ANI code (one hex digit per byte, low nibble) | 🔶 |
| 0x0E20 | ~0x28 | General settings | ✅ |
| 0x0E4A | 1 | Bit 7: active VFO (0 = A, 1 = B) | 🔶 |
| 0x0E4C | 1 | 0 = VFO (frequency) mode | 🔶 |
| 0x0F08 | 32 | VFO A | ✅ |
| 0x0F28 | 32 | VFO B | 🔶 |
| 0x1000 | 128 × 16 | Channel names (7 chars used, per channel) | 🔶 |
| 0x1EC0 + 0x10 | 14 | Text string (baoclone calls it "serial"; ❓ may be power-on message line) | ✅ present |
| 0x1EC0 + 0x20 | 14 | Power-on message (2 lines × 7 chars) | ✅ |
| 0x1EC0 + 0x30 | 14 | Firmware version string, e.g. `  Ver  BFB291  ` | ✅ |
| 0x1EC0 + 0x100 = 0x1FC0 | 5 | VHF TX/RX limits | ✅ |
| 0x1EC0 + 0x105 = 0x1FC5 | 5 | UHF TX/RX limits | ✅ |

### 6.2 Channel record (16 bytes at `0x0000 + n × 16`) ✅

```
 byte:  0   1   2   3 │ 4   5   6   7 │ 8   9 │10  11 │ 12  │ 13 │ 14  │ 15
       ┌──────────────┼───────────────┼───────┼───────┼─────┼────┼─────┼─────┐
       │  RX freq     │  TX freq      │RX tone│TX tone│scode│ -- │power│flags│
       │  BCD, LE     │  BCD, LE      │ u16LE │ u16LE │     │    │     │     │
       └──────────────┴───────────────┴───────┴───────┴─────┴────┴─────┴─────┘
```

| Byte(s) | Field | Encoding |
|---|---|---|
| 0–3 | RX frequency | 8 BCD digits, **little-endian** (last byte = most significant), units of **10 Hz**. Example: bytes `00 25 60 13` → `13602500` → 136.025 00 MHz ✅ |
| 4–7 | TX frequency | Same. `FF FF FF FF` = TX disabled |
| 8–9 | RX tone | See §6.3 |
| 10–11 | TX tone | See §6.3 |
| 12 | Signal code (PTT-ID group) | Low 4 bits; high 4 bits unknown |
| 13 | — | Unknown, write 0 |
| 14 | Power | Bit 0: 1 = low power, 0 = high |
| 15 | Flags | Bit 0: PTT-ID at beginning · Bit 1: PTT-ID at end · Bit 2: **scan add** · Bit 3: **busy-channel lockout** · Bits 4–5: unknown · Bit 6: **wide** (1 = 25 kHz, 0 = 12.5 kHz narrow) · Bit 7: unknown |

Example ✅: factory channel 0 has byte 15 = `0x44` → bits 2 and 6 → scan on, wide.

**Empty channel:** all 16 bytes `FF` (and the name bytes `FF`). A record with RX frequency `00000000` or non-BCD digits is also treated as empty.

**Channel name** (at `0x1000 + n × 16`): 7 bytes ASCII, uppercase; unused trailing bytes `FF`; all-`FF` = no name.

### 6.3 Tone (CTCSS / DCS) encoding 🔶

16-bit little-endian value:

| Value | Meaning |
|---|---|
| `0x0000` or `0xFFFF` | No tone |
| ≥ `0x0258` (600) | CTCSS, in tenths of Hz. e.g. 100.0 Hz → 1000 → `0x03E8` |
| 1 – 104 | DCS code, **normal** polarity: index into the 104-entry DCS table (1-based) |
| 105 – 208 (approx.) | DCS code, **inverted** polarity |

❓ **Discrepancy found:** baoclone's *decoder* treats value `104 + k` as inverted code *k*, but its *encoder* writes `105 + k`. One of them is off by one. We will settle this with a trace from a real radio (set a channel to D023I on the keypad, read it back).

**DCS table (104 codes, in index order):**
```
023 025 026 031 032 036 043 047 051 053 054 065 071 072 073 074 114 115
116 122 125 131 132 134 143 145 152 155 156 162 165 172 174 205 212 223
225 226 243 244 245 246 251 252 255 261 263 265 266 271 274 306 311 315
325 331 332 343 346 351 356 364 365 371 411 412 413 423 431 432 445 446
452 454 455 462 464 465 466 503 506 516 523 526 532 546 565 606 612 624
627 631 632 654 662 664 703 712 723 731 732 734 743 754
```

### 6.4 General settings (at `0x0E20`) ✅ offsets / 🔶 meanings

| Offset | Setting | Factory value (sample) |
|---|---|---|
| +0x00 | Squelch level (0–9) | 4 |
| +0x01 | Tuning step (index: 2.5, 5, 6.25, 10, 12.5, 20, 25, 50 kHz) | 1 (5 kHz) |
| +0x03 | Battery saver (0 = off, 1–4) | 3 |
| +0x04 | VOX level (0 = off, 1–10) | 0 |
| +0x06 | Backlight timeout (0 = off, 1–5) | 5 |
| +0x07 | Dual watch | 0 |
| +0x08 | Beep | 1 |
| +0x09 | Transmit timeout timer | 3 |
| +0x0E | Voice prompts (0 off, 1 English, 2 Chinese) | 1 |
| +0x10 | DTMF sidetone | 3 |
| +0x12 | Scan resume (TO / CO / SE) | 0 |
| +0x13 | PTT-ID mode | 0 |
| +0x14 | PTT-ID length | 5 |
| +0x15 | Display mode A (0 ch, 1 name, 2 freq) | 2 |
| +0x16 | Display mode B | 2 |
| +0x17 | Busy channel lockout | 0 |
| +0x18 | Auto key lock | 0 |
| +0x19 | Shift direction | 0 |
| +0x1D | Standby LED color (off/blue/orange/purple) | 3 |
| +0x1E | RX LED color | 1 |
| +0x1F | TX LED color | 2 |
| +0x20 | Alarm mode | 0 |
| +0x21 | Band | 0 |
| +0x22 | Dual watch priority | 0 |
| +0x23 | Squelch tail elimination | 0 |
| +0x24 | Repeater STE | 0 |
| +0x25 | Repeater STE delay | 0 |
| +0x26 | Power-on message type | 0 |
| +0x27 | Roger beep | 0 |

Offsets +0x02, +0x05, +0x0A–0x0D, +0x0F, +0x11, +0x1A–0x1C are unknown — **preserve as read**.

### 6.5 VFO record (32 bytes at 0x0F08 = A, 0x0F28 = B) ✅ partial

| Byte(s) | Field | Encoding |
|---|---|---|
| 0–7 | Frequency | One decimal digit per byte, most significant first, units of 10 Hz. Sample: `01 03 06 00 02 05 00 00` → 136.02500 MHz ✅ |
| 8 | — | unknown |
| 9–12 | Offset | One digit per byte, units of 100 kHz |
| 13 | — | unknown |
| 14–15 | RX tone | §6.3 |
| 16–17 | TX tone | §6.3 |
| 18 | Band | Bit 0: 0 = VHF, 1 = UHF |
| 20 | Signal code | Low 4 bits |
| 22 | Step | Bits 4–6 |
| 23 | Width / power | Bit 6: narrow · Bit 7: low power |

### 6.6 Band limits (5 bytes each) ✅

| Byte | Meaning |
|---|---|
| 0 | Enabled (1) |
| 1–2 | Lower limit, MHz, 4 BCD digits (big-endian here!) |
| 3–4 | Upper limit, MHz, 4 BCD digits |

Sample: VHF `01 01 36 01 74` → 136–174 MHz; UHF `01 04 00 05 20` → 400–520 MHz. Limits only work on firmware ≥ BFB291.

> ⚠️ Changing band limits to unlock transmit outside the radio's certified range is not something this project will offer. GMRS (462–467 MHz) is already inside the factory UHF range.

---

## 7. Safety rules for our implementation

1. **First contact is read-only.** Phase 1 begins with `detect` and `backup` only.
2. **Never overwrite a backup.** Backups are timestamped and kept forever by default.
3. **Write only what we understand.** Initial write support covers the **main block** only (channels, names, settings). The aux block (firmware string, band limits) is not written until we have trace-verified it.
4. **Preserve unknown bytes.** Edits change only known fields; everything else is written back exactly as read.
5. **Verify every write** by reading the image back and comparing byte-for-byte.
6. **Detect firmware** from the version string and refuse to write to unrecognized versions until tested.

---

## 8. Open questions / verification plan

| # | Question | How we'll answer it |
|---|---|---|
| 1 | Does our UV-5RA use the "291" magic? Single write or byte-by-byte? | First `gmrs detect <port> --trace` run. Expected: "291" magic, since our firmware is BFB297 |
| 2 | ~~Our firmware version~~ | **Answered: BFB297** (owner's radio, 2026-11-19). Still to confirm in the aux block after first backup |
| 3 | DCS inverted off-by-one (§6.3) | Program D023I on the keypad, read image, inspect |
| 4 | Is 0x1EC0+0x10 a serial number or a message line? | Compare two radios' images |
| 5 | Meaning of identifier middle bytes | Compare identifiers across radios |
| 6 | Does reading leave the radio in a sane state? | Confirm radio works normally after power-cycle |

### Known variants that may differ
Newer radios sold under similar names (e.g. **UV-5R Mini, UV-5RM, UV-5G/UV-5X**) may use different handshakes or memory layouts entirely. Each gets its own spec file in `docs/protocol/` when we add it.

---

## Change log

| Date | Change |
|---|---|
| 2026-11-19 | v0.1 — initial draft from baoclone source + sample factory image |
