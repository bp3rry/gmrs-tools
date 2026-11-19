//! The radio's memory image and the `.img` file format (spec §5, §6).

use super::channel::{self, Channel, NUM_CHANNELS};
use crate::{Error, Result};

pub const IDENT_LEN: usize = 8;
/// End (exclusive) of the main block.
pub const MAIN_END: usize = 0x1800;
/// Start of the aux block.
pub const AUX_START: usize = 0x1EC0;
/// Size of the radio's address space we model.
pub const MEM_SIZE: usize = 0x2000;

const FILE_SIZE_FULL: usize = IDENT_LEN + MAIN_END + (MEM_SIZE - AUX_START); // 6472
const FILE_SIZE_MAIN_ONLY: usize = IDENT_LEN + MAIN_END; // 6152

/// A complete copy of the radio's memory.
///
/// `mem` always spans the full 0x0000–0x1FFF address range so that offsets in
/// the spec can be used directly. The gap 0x1800–0x1EBF is never read from or
/// written to the radio and is left as 0xFF.
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    pub ident: [u8; IDENT_LEN],
    pub mem: Vec<u8>,
    /// False for old ("aged") firmware, which has no aux block.
    pub has_aux: bool,
}

/// One band's transmit/receive limits (spec §6.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BandLimit {
    pub enabled: bool,
    pub lower_mhz: u16,
    pub upper_mhz: u16,
}

impl Image {
    pub fn blank(ident: [u8; IDENT_LEN], has_aux: bool) -> Self {
        Image {
            ident,
            mem: vec![0xFF; MEM_SIZE],
            has_aux,
        }
    }

    /// Address ranges that exist on the radio for this image.
    pub fn regions(&self) -> &'static [(usize, usize)] {
        if self.has_aux {
            &[(0, MAIN_END), (AUX_START, MEM_SIZE)]
        } else {
            &[(0, MAIN_END)]
        }
    }

    /// Parse a `.img` file (spec §5).
    pub fn from_file_bytes(data: &[u8]) -> Result<Self> {
        let has_aux = match data.len() {
            FILE_SIZE_FULL => true,
            FILE_SIZE_MAIN_ONLY => false,
            n => {
                return Err(Error::Format(format!(
                    "expected {FILE_SIZE_FULL} or {FILE_SIZE_MAIN_ONLY} bytes, got {n}"
                )));
            }
        };
        let mut ident = [0u8; IDENT_LEN];
        ident.copy_from_slice(&data[..IDENT_LEN]);
        let mut img = Image::blank(ident, has_aux);
        let mut pos = IDENT_LEN;
        for &(start, end) in img.regions() {
            let len = end - start;
            img.mem[start..end].copy_from_slice(&data[pos..pos + len]);
            pos += len;
        }
        Ok(img)
    }

    /// Serialize to the `.img` file format (spec §5).
    pub fn to_file_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FILE_SIZE_FULL);
        out.extend_from_slice(&self.ident);
        for &(start, end) in self.regions() {
            out.extend_from_slice(&self.mem[start..end]);
        }
        out
    }

    /// Firmware version string, e.g. "BFB291" (aux + 0x30).
    pub fn firmware_version(&self) -> Option<String> {
        self.aux_text(0x30)
            .map(|s| s.trim_start_matches("Ver").trim().to_string())
    }

    /// Power-on message (aux + 0x20, two 7-character lines).
    pub fn power_on_message(&self) -> Option<String> {
        self.aux_text(0x20)
    }

    fn aux_text(&self, offset: usize) -> Option<String> {
        if !self.has_aux {
            return None;
        }
        let start = AUX_START + offset;
        let text: String = self.mem[start..start + 14]
            .iter()
            .map(|&b| {
                if (0x20..0x7F).contains(&b) {
                    b as char
                } else {
                    ' '
                }
            })
            .collect();
        Some(text.trim().to_string())
    }

    /// VHF and UHF limits, or `None` on aged firmware (spec §6.6).
    pub fn band_limits(&self) -> Option<[BandLimit; 2]> {
        if !self.has_aux {
            return None;
        }
        let read = |at: usize| {
            let b = &self.mem[at..at + 5];
            BandLimit {
                enabled: b[0] == 1,
                lower_mhz: bcd_be_u16(b[1], b[2]),
                upper_mhz: bcd_be_u16(b[3], b[4]),
            }
        };
        Some([read(AUX_START + 0x100), read(AUX_START + 0x105)])
    }

    /// Channel `n` (0–127), or `None` if that slot is empty.
    pub fn channel(&self, n: usize) -> Option<Channel> {
        assert!(n < NUM_CHANNELS, "channel index out of range");
        let rec = &self.mem[n * 16..n * 16 + 16];
        let name = &self.mem[0x1000 + n * 16..0x1000 + n * 16 + 7];
        channel::decode(rec, name)
    }

    /// All non-empty channels with their slot numbers.
    pub fn channels(&self) -> impl Iterator<Item = (usize, Channel)> + '_ {
        (0..NUM_CHANNELS).filter_map(|n| self.channel(n).map(|c| (n, c)))
    }
}

fn bcd_be_u16(hi: u8, lo: u8) -> u16 {
    let d = |b: u8| ((b >> 4) as u16) * 10 + (b & 0x0F) as u16;
    d(hi) * 100 + d(lo)
}
