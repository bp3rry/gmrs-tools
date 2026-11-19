//! `gmrs` — command-line front-end. All radio logic lives in gmrs-core;
//! this file only handles arguments, the serial port, files and printing.

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gmrs_core::uv5r::{Image, Options, protocol};

const USAGE: &str = "\
gmrs — GMRS Toolkit command-line tool

USAGE:
  gmrs ports                     List serial ports (find your programming cable)
  gmrs detect <port> [--trace]   Check the radio answers (read-only)
  gmrs backup <port> [--trace] [--out <dir>]
                                 Save a full copy of the radio's memory (read-only)
  gmrs show <file.img>           Print what's inside a saved image

  --trace   print every byte sent/received (for troubleshooting)

Example:
  gmrs ports
  gmrs backup /dev/cu.usbserial-1420

After detect/backup, turn the radio off and on again to leave clone mode.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let trace = args.iter().any(|a| a == "--trace");
    let out_dir = flag_value(&args, "--out").map(PathBuf::from);
    let positional: Vec<&str> = positional(&args);

    let result = match positional.as_slice() {
        ["ports"] => cmd_ports(),
        ["detect", port] => cmd_detect(port, trace),
        ["backup", port] => cmd_backup(port, trace, out_dir),
        ["show", file] => cmd_show(Path::new(file)),
        _ => {
            print!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\nError: {e}");
            ExitCode::FAILURE
        }
    }
}

type CliResult = Result<(), Box<dyn std::error::Error>>;

// ---- commands -------------------------------------------------------------

fn cmd_ports() -> CliResult {
    let ports = serialport::available_ports()?;
    // On macOS, use /dev/cu.* (call-out) devices, not /dev/tty.*.
    let ports: Vec<_> = ports
        .into_iter()
        .filter(|p| !p.port_name.starts_with("/dev/tty."))
        .collect();
    if ports.is_empty() {
        println!("No serial ports found. Is the programming cable plugged in?");
        return Ok(());
    }
    for p in ports {
        let detail = match p.port_type {
            serialport::SerialPortType::UsbPort(u) => format!(
                "USB {:04x}:{:04x} {}{}",
                u.vid,
                u.pid,
                u.manufacturer.unwrap_or_default(),
                chip_hint(u.vid)
            ),
            _ => String::new(),
        };
        println!("{:<36} {detail}", p.port_name);
    }
    Ok(())
}

fn chip_hint(vid: u16) -> &'static str {
    match vid {
        0x0403 => "  (FTDI — good)",
        0x1a86 => "  (CH340 — good)",
        0x067b => "  (Prolific PL2303 — may be a counterfeit that fails on macOS)",
        0x10c4 => "  (Silicon Labs CP210x — good)",
        _ => "",
    }
}

fn cmd_detect(port_name: &str, trace: bool) -> CliResult {
    let mut port = open(port_name, trace)?;
    eprintln!("Contacting radio on {port_name} ...");
    let conn = protocol::connect(&mut port, &Options::default())?;
    println!("Radio answered.");
    println!(
        "  Handshake:  {:?}{}",
        conn.variant,
        if conn.byte_by_byte {
            " (byte-by-byte)"
        } else {
            ""
        }
    );
    println!("  Identifier: {}", hex(&conn.ident));
    println!("\nTurn the radio off and on again before using it.");
    Ok(())
}

fn cmd_backup(port_name: &str, trace: bool, out_dir: Option<PathBuf>) -> CliResult {
    let dir = out_dir.unwrap_or_else(default_backup_dir);
    fs::create_dir_all(&dir)?;

    let mut port = open(port_name, trace)?;
    eprintln!("Contacting radio on {port_name} ...");
    let conn = protocol::connect(&mut port, &Options::default())?;
    eprintln!("Connected ({:?}). Reading memory:", conn.variant);

    let img = protocol::read_image(&mut port, &conn, |done, total| {
        if !trace {
            eprint!("\r  [{:<40}] {done}/{total}", "#".repeat(done * 40 / total));
        }
    })?;
    eprintln!();

    let fw = img.firmware_version().unwrap_or_else(|| "aged".into());
    let path = dir.join(format!("uv5r-{}-{}.img", sanitize(&fw), timestamp()));
    // create_new: never overwrite an existing backup (spec §7, rule 2).
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?
        .write_all(&img.to_file_bytes())?;

    // Re-read the file to confirm it's intact.
    let check = Image::from_file_bytes(&fs::read(&path)?)?;
    if check != img {
        return Err("backup file verification failed".into());
    }

    println!("Backup saved and verified:\n  {}", path.display());
    println!(
        "  Firmware: {fw}   Channels in use: {}",
        img.channels().count()
    );
    println!("\nTurn the radio off and on again before using it.");
    Ok(())
}

fn cmd_show(path: &Path) -> CliResult {
    let img = Image::from_file_bytes(&fs::read(path)?)?;
    println!("File:       {}", path.display());
    println!("Identifier: {}", hex(&img.ident));
    println!(
        "Firmware:   {}",
        img.firmware_version()
            .unwrap_or_else(|| "(aged — not stored)".into())
    );
    if let Some(msg) = img.power_on_message() {
        println!("Power-on:   {msg}");
    }
    if let Some([vhf, uhf]) = img.band_limits() {
        println!(
            "Limits:     VHF {}–{} MHz, UHF {}–{} MHz",
            vhf.lower_mhz, vhf.upper_mhz, uhf.lower_mhz, uhf.upper_mhz
        );
    }

    println!();
    println!(
        "{:>4}  {:<7}  {:>10}  {:>10}  {:>7}  {:>7}  {:<5} {:<6} Scan",
        "Ch", "Name", "RX MHz", "TX MHz", "RX tone", "TX tone", "Power", "Width"
    );
    for (n, c) in img.channels() {
        let tx = c.tx_hz.map(mhz).unwrap_or_else(|| "off".into());
        println!(
            "{:>4}  {:<7}  {:>10}  {:>10}  {:>7}  {:>7}  {:<5} {:<6} {}",
            n,
            if c.name.is_empty() { "-" } else { &c.name },
            mhz(c.rx_hz),
            tx,
            c.rx_tone.to_string(),
            c.tx_tone.to_string(),
            if c.low_power { "Low" } else { "High" },
            if c.wide { "Wide" } else { "Narrow" },
            if c.scan { "yes" } else { "-" },
        );
    }
    Ok(())
}

// ---- serial port ----------------------------------------------------------

/// Open the port with the UV-5R settings (spec §2), optionally tracing.
fn open(name: &str, trace: bool) -> io::Result<Port> {
    let inner = serialport::new(name, 9600)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(200))
        .open()?;
    Ok(Port { inner, trace })
}

/// Serial port wrapper that can print traffic (spec §8: verification traces).
struct Port {
    inner: Box<dyn serialport::SerialPort>,
    trace: bool,
}

impl Read for Port {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if self.trace && n > 0 {
            eprintln!("RX {}", hex(&buf[..n]));
        }
        Ok(n)
    }
}

impl Write for Port {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.trace {
            eprintln!("TX {}", hex(buf));
        }
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

// ---- small helpers --------------------------------------------------------

fn positional(args: &[String]) -> Vec<&str> {
    let mut out = vec![];
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
        } else if a == "--out" {
            skip = true;
        } else if !a.starts_with("--") {
            out.push(a.as_str());
        }
    }
    out
}

fn flag_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

fn hex(b: &[u8]) -> String {
    b.iter()
        .map(|x| format!("{x:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn mhz(hz: u64) -> String {
    format!("{}.{:05}", hz / 1_000_000, (hz % 1_000_000) / 10)
}

fn sanitize(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// Per-OS data folder (docs/ARCHITECTURE.md §3).
fn default_backup_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/GMRS Toolkit/backups")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join("GMRS Toolkit\\backups")
    } else {
        home.join(".local/share/gmrs-toolkit/backups")
    }
}

/// UTC timestamp like 20261119-104512Z, without a date library.
fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs as i64)
}

fn format_utc(secs: i64) -> String {
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn utc_formatting() {
        assert_eq!(super::format_utc(0), "19700101-000000Z");
        assert_eq!(super::format_utc(951_782_400), "20000229-000000Z"); // leap day
        assert_eq!(super::format_utc(1_795_107_628), "20261119-170028Z");
    }
}
