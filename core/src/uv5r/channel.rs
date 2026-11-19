//! Channel records and tone encoding (spec §6.2, §6.3).

use std::fmt;

pub const NUM_CHANNELS: usize = 128;

/// DCS codes in the order the radio indexes them (spec §6.3).
/// ❓ Unverified on real hardware — see spec §8, question 3.
pub const DCS_CODES: [u16; 104] = [
    23, 25, 26, 31, 32, 36, 43, 47, 51, 53, 54, 65, 71, 72, 73, 74, 114, 115, 116, 122, 125, 131,
    132, 134, 143, 145, 152, 155, 156, 162, 165, 172, 174, 205, 212, 223, 225, 226, 243, 244, 245,
    246, 251, 252, 255, 261, 263, 265, 266, 271, 274, 306, 311, 315, 325, 331, 332, 343, 346, 351,
    356, 364, 365, 371, 411, 412, 413, 423, 431, 432, 445, 446, 452, 454, 455, 462, 464, 465, 466,
    503, 506, 516, 523, 526, 532, 546, 565, 606, 612, 624, 627, 631, 632, 654, 662, 664, 703, 712,
    723, 731, 732, 734, 743, 754,
];

/// A squelch tone ("privacy code").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    None,
    /// CTCSS tone in tenths of a hertz (e.g. 1000 = 100.0 Hz).
    Ctcss(u16),
    /// DCS code with normal polarity, e.g. 23 = D023N.
    Dcs(u16),
    /// A raw value we can't decode with confidence yet (e.g. inverted DCS,
    /// whose mapping is unverified). Preserved exactly.
    Unknown(u16),
}

impl Tone {
    pub fn decode(v: u16) -> Tone {
        match v {
            0 | 0xFFFF => Tone::None,
            600.. => Tone::Ctcss(v),
            1..=104 => Tone::Dcs(DCS_CODES[(v - 1) as usize]),
            _ => Tone::Unknown(v),
        }
    }
}

impl fmt::Display for Tone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tone::None => write!(f, "-"),
            Tone::Ctcss(t) => write!(f, "{}.{}", t / 10, t % 10),
            Tone::Dcs(c) => write!(f, "D{c:03}N"),
            Tone::Unknown(v) => write!(f, "?{v}"),
        }
    }
}

/// One memory channel, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub name: String,
    pub rx_hz: u64,
    /// `None` means transmit is disabled on this channel.
    pub tx_hz: Option<u64>,
    pub rx_tone: Tone,
    pub tx_tone: Tone,
    pub low_power: bool,
    /// true = wide (25 kHz), false = narrow (12.5 kHz).
    pub wide: bool,
    pub scan: bool,
    pub busy_lockout: bool,
}

/// Decode a 16-byte channel record plus its 7-byte name.
/// Returns `None` for an empty slot.
pub fn decode(rec: &[u8], name: &[u8]) -> Option<Channel> {
    let rx_hz = bcd_le_freq(&rec[0..4])?;
    if rx_hz == 0 {
        return None;
    }
    let tx_hz = bcd_le_freq(&rec[4..8]).filter(|&hz| hz != 0);
    let u16le = |i: usize| u16::from_le_bytes([rec[i], rec[i + 1]]);
    let flags = rec[15];
    Some(Channel {
        name: decode_name(name),
        rx_hz,
        tx_hz,
        rx_tone: Tone::decode(u16le(8)),
        tx_tone: Tone::decode(u16le(10)),
        low_power: rec[14] & 0x01 != 0,
        wide: flags & 0x40 != 0,
        scan: flags & 0x04 != 0,
        busy_lockout: flags & 0x08 != 0,
    })
}

/// 4 bytes, 8 BCD digits, little-endian, units of 10 Hz. `None` if any nibble
/// isn't a decimal digit (which includes the all-0xFF "empty" pattern).
fn bcd_le_freq(b: &[u8]) -> Option<u64> {
    let mut v: u64 = 0;
    for &byte in b.iter().rev() {
        let (hi, lo) = (byte >> 4, byte & 0x0F);
        if hi > 9 || lo > 9 {
            return None;
        }
        v = v * 100 + (hi as u64) * 10 + lo as u64;
    }
    Some(v * 10)
}

fn decode_name(b: &[u8]) -> String {
    b.iter()
        .take_while(|&&c| c != 0xFF && c != 0x00)
        .map(|&c| c as char)
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_bcd_little_endian() {
        // spec §6.2 example: 00 25 60 13 -> 136.025 MHz
        assert_eq!(bcd_le_freq(&[0x00, 0x25, 0x60, 0x13]), Some(136_025_000));
        assert_eq!(bcd_le_freq(&[0xFF; 4]), None);
    }

    #[test]
    fn tones() {
        assert_eq!(Tone::decode(0), Tone::None);
        assert_eq!(Tone::decode(0xFFFF), Tone::None);
        assert_eq!(Tone::decode(1000).to_string(), "100.0");
        assert_eq!(Tone::decode(1).to_string(), "D023N");
        assert_eq!(Tone::decode(104).to_string(), "D754N");
        assert!(matches!(Tone::decode(105), Tone::Unknown(105)));
    }
}
