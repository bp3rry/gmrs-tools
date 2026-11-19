//! Clone-mode protocol (spec §1–§4). Read-only for now (spec §7, rule 1).
//!
//! Works over anything that implements `Read + Write`: a real serial port,
//! a Bluetooth bridge, or a simulated radio in tests. Reads must time out
//! (returning `Ok(0)` or a `TimedOut` error) rather than block forever.

use std::io::{ErrorKind, Read, Write};
use std::thread::sleep;
use std::time::Duration;

use super::image::{IDENT_LEN, Image};
use crate::{Error, Result};

/// Magic for firmware BFB291 and later (spec §3.1).
pub const MAGIC_BFB291: [u8; 7] = [0x50, 0xBB, 0xFF, 0x20, 0x12, 0x07, 0x25];
/// Magic for firmware before BFB291 (spec §3.1).
pub const MAGIC_AGED: [u8; 7] = [0x50, 0xBB, 0xFF, 0x01, 0x25, 0x98, 0x4D];

const ACK: u8 = 0x06;
const READ_BLOCK: usize = 0x40;

/// Which handshake the radio answered to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// Firmware BFB291 or later: main + aux blocks.
    Bfb291OrLater,
    /// Older firmware: main block only.
    Aged,
}

/// Result of a successful handshake. The radio is now in clone mode and must
/// be power-cycled when we're done.
#[derive(Debug, Clone, Copy)]
pub struct Connected {
    pub variant: Variant,
    pub ident: [u8; IDENT_LEN],
    /// Whether the magic had to be sent one byte at a time (spec §3.1 ❓).
    pub byte_by_byte: bool,
}

/// Timing knobs. Defaults follow the spec; tests set them to zero.
#[derive(Debug, Clone)]
pub struct Options {
    pub attempts: u32,
    pub retry_delay: Duration,
    pub byte_delay: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            attempts: 3,
            retry_delay: Duration::from_millis(500),
            byte_delay: Duration::from_millis(10),
        }
    }
}

/// Perform the handshake (spec §3.2), trying each known magic both as a
/// single write and byte-by-byte.
pub fn connect<P: Read + Write>(port: &mut P, opts: &Options) -> Result<Connected> {
    let candidates = [
        (Variant::Bfb291OrLater, &MAGIC_BFB291),
        (Variant::Aged, &MAGIC_AGED),
    ];
    for _ in 0..opts.attempts {
        for (variant, magic) in candidates {
            for byte_by_byte in [false, true] {
                if let Some(ident) = try_magic(port, magic, byte_by_byte, opts)? {
                    return Ok(Connected {
                        variant,
                        ident,
                        byte_by_byte,
                    });
                }
                sleep(opts.retry_delay);
            }
        }
    }
    Err(Error::Timeout(
        "radio did not respond to any handshake. Check: radio on, volume up, cable fully \
         inserted, correct port"
            .into(),
    ))
}

fn try_magic<P: Read + Write>(
    port: &mut P,
    magic: &[u8],
    byte_by_byte: bool,
    opts: &Options,
) -> Result<Option<[u8; IDENT_LEN]>> {
    drain(port);
    if byte_by_byte {
        for b in magic {
            port.write_all(&[*b])?;
            port.flush()?;
            sleep(opts.byte_delay);
        }
    } else {
        port.write_all(magic)?;
        port.flush()?;
    }
    if read_byte(port)? != Some(ACK) {
        return Ok(None); // not this magic; caller tries the next one
    }

    port.write_all(&[0x02])?;
    let mut ident = [0u8; IDENT_LEN];
    read_exact(port, &mut ident, "identifier")?;

    port.write_all(&[ACK])?;
    match read_byte(port)? {
        Some(ACK) => Ok(Some(ident)),
        Some(b) => Err(Error::Protocol(format!(
            "radio refused clone mode (sent {b:02X})"
        ))),
        None => Err(Error::Timeout("radio did not confirm clone mode".into())),
    }
}

/// Read the whole radio memory into an image (spec §4.3).
/// `progress(done, total)` is called after each block.
pub fn read_image<P: Read + Write>(
    port: &mut P,
    conn: &Connected,
    mut progress: impl FnMut(usize, usize),
) -> Result<Image> {
    let mut img = Image::blank(conn.ident, conn.variant == Variant::Bfb291OrLater);
    let regions = img.regions();
    let total: usize = regions.iter().map(|(s, e)| (e - s) / READ_BLOCK).sum();
    let mut done = 0;
    for &(start, end) in regions {
        for addr in (start..end).step_by(READ_BLOCK) {
            read_block(port, addr as u16, &mut img.mem[addr..addr + READ_BLOCK])?;
            done += 1;
            progress(done, total);
        }
    }
    Ok(img)
}

/// Read one block with the `S` command (spec §4.1).
fn read_block<P: Read + Write>(port: &mut P, addr: u16, out: &mut [u8]) -> Result<()> {
    let [hi, lo] = addr.to_be_bytes();
    let len = out.len() as u8;
    port.write_all(&[b'S', hi, lo, len])?;

    let what = format!("block {addr:04X}");
    let mut hdr = [0u8; 4];
    read_exact(port, &mut hdr, &what)?;
    if hdr[0] == ACK {
        // Late ACK from the previous block (spec §4.1 quirk).
        hdr.copy_within(1.., 0);
        read_exact(port, &mut hdr[3..], &what)?;
    }
    if hdr != [b'X', hi, lo, len] {
        return Err(Error::Protocol(format!(
            "unexpected reply to {what}: {hdr:02X?}"
        )));
    }
    read_exact(port, out, &what)?;

    port.write_all(&[ACK])?;
    match read_byte(port)? {
        None | Some(ACK) => Ok(()), // missing final ACK is tolerated (spec §4.1)
        Some(b) => Err(Error::Protocol(format!(
            "bad acknowledge after {what}: {b:02X}"
        ))),
    }
}

// ---- low-level helpers -------------------------------------------------

fn read_exact<P: Read>(port: &mut P, buf: &mut [u8], what: &str) -> Result<()> {
    let mut got = 0;
    while got < buf.len() {
        match port.read(&mut buf[got..]) {
            Ok(0) => return Err(timeout(what, got, buf.len())),
            Ok(n) => got += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) => {
                return Err(timeout(what, got, buf.len()));
            }
            Err(e) => return Err(Error::Io(e)),
        }
    }
    Ok(())
}

fn timeout(what: &str, got: usize, want: usize) -> Error {
    Error::Timeout(format!(
        "{what}: radio stopped responding ({got} of {want} bytes)"
    ))
}

/// Read a single byte, returning `None` on timeout.
fn read_byte<P: Read>(port: &mut P) -> Result<Option<u8>> {
    let mut b = [0u8; 1];
    match read_exact(port, &mut b, "byte") {
        Ok(()) => Ok(Some(b[0])),
        Err(Error::Timeout(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Discard any stale input.
fn drain<P: Read>(port: &mut P) {
    let mut junk = [0u8; 64];
    while matches!(port.read(&mut junk), Ok(n) if n > 0) {}
}
