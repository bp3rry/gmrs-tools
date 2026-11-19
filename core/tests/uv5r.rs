//! Tests against the sample factory image and a simulated radio.

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::time::Duration;

use gmrs_core::uv5r::{Image, Options, Tone, Variant, protocol};

const FACTORY: &[u8] = include_bytes!("../../tests/fixtures/uv5r/baoclone-factory-bfb291.img");

fn factory() -> Image {
    Image::from_file_bytes(FACTORY).expect("fixture parses")
}

// ---- image decoding ------------------------------------------------------

#[test]
fn file_round_trip_is_byte_identical() {
    assert_eq!(factory().to_file_bytes(), FACTORY);
}

#[test]
fn rejects_wrong_size() {
    assert!(Image::from_file_bytes(&FACTORY[..100]).is_err());
}

#[test]
fn decodes_header_fields() {
    let img = factory();
    assert_eq!(img.ident, [0xAA, 0x36, 0x74, 0x04, 0x00, 0x05, 0x20, 0xDD]);
    assert_eq!(img.firmware_version().as_deref(), Some("BFB291"));
    assert_eq!(img.power_on_message().as_deref(), Some("AAAAAAABBBBBBB"));
    let [vhf, uhf] = img.band_limits().unwrap();
    assert_eq!((vhf.lower_mhz, vhf.upper_mhz), (136, 174));
    assert_eq!((uhf.lower_mhz, uhf.upper_mhz), (400, 520));
}

#[test]
fn decodes_factory_channels() {
    let chans: Vec<_> = factory().channels().collect();
    assert_eq!(chans.len(), 2);

    let (n, c) = &chans[0];
    assert_eq!(*n, 0);
    assert_eq!(c.rx_hz, 136_025_000);
    assert_eq!(c.tx_hz, Some(136_025_000));
    assert_eq!((c.rx_tone, c.tx_tone), (Tone::None, Tone::None));
    assert!(c.wide && c.scan && !c.low_power && !c.busy_lockout);

    assert_eq!(chans[1].0, 127);
    assert_eq!(chans[1].1.rx_hz, 470_625_000);
}

// ---- simulated radio -------------------------------------------------------

/// Minimal state machine that behaves like a UV-5R in clone mode.
struct SimRadio {
    img: Image,
    input: Vec<u8>,
    output: VecDeque<u8>,
    state: State,
}

#[derive(PartialEq)]
enum State {
    Idle,
    MagicOk,
    IdentSent,
    Clone,
}

impl SimRadio {
    fn new(img: Image) -> Self {
        SimRadio {
            img,
            input: vec![],
            output: VecDeque::new(),
            state: State::Idle,
        }
    }

    fn process(&mut self) {
        loop {
            match self.state {
                State::Idle => {
                    if self.input.ends_with(&protocol::MAGIC_BFB291) {
                        self.input.clear();
                        self.output.push_back(0x06);
                        self.state = State::MagicOk;
                    }
                    return;
                }
                State::MagicOk if self.input.first() == Some(&0x02) => {
                    self.input.remove(0);
                    self.output.extend(self.img.ident);
                    self.state = State::IdentSent;
                }
                State::IdentSent if self.input.first() == Some(&0x06) => {
                    self.input.remove(0);
                    self.output.push_back(0x06);
                    self.state = State::Clone;
                }
                State::Clone if self.input.first() == Some(&0x06) => {
                    self.input.remove(0);
                    self.output.push_back(0x06);
                }
                State::Clone if self.input.len() >= 4 && self.input[0] == b'S' => {
                    let cmd: Vec<u8> = self.input.drain(..4).collect();
                    let addr = u16::from_be_bytes([cmd[1], cmd[2]]) as usize;
                    let len = cmd[3] as usize;
                    self.output.extend([b'X', cmd[1], cmd[2], cmd[3]]);
                    self.output.extend(&self.img.mem[addr..addr + len]);
                }
                _ => return,
            }
        }
    }
}

impl Read for SimRadio {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.output.is_empty() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let n = buf.len().min(self.output.len());
        for b in buf.iter_mut().take(n) {
            *b = self.output.pop_front().unwrap();
        }
        Ok(n)
    }
}

impl Write for SimRadio {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.input.extend_from_slice(buf);
        self.process();
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn fast() -> Options {
    Options {
        attempts: 1,
        retry_delay: Duration::ZERO,
        byte_delay: Duration::ZERO,
    }
}

#[test]
fn backup_from_simulated_radio_matches_source_image() {
    let mut radio = SimRadio::new(factory());
    let conn = protocol::connect(&mut radio, &fast()).expect("handshake");
    assert_eq!(conn.variant, Variant::Bfb291OrLater);
    assert_eq!(conn.ident, factory().ident);

    let mut last = (0, 0);
    let img = protocol::read_image(&mut radio, &conn, |d, t| last = (d, t)).expect("read");
    assert_eq!(last, (101, 101)); // 96 main + 5 aux blocks
    assert_eq!(img.to_file_bytes(), FACTORY);
}

#[test]
fn silent_radio_times_out_cleanly() {
    struct Silent;
    impl Read for Silent {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::TimedOut.into())
        }
    }
    impl Write for Silent {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let err = protocol::connect(&mut Silent, &fast()).unwrap_err();
    assert!(err.to_string().contains("did not respond"));
}
