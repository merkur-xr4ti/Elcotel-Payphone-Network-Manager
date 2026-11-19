//! Calling a payphone for real: a Hayes modem on a serial port (a real modem, or the
//! Virtual SIP Modem through a com0com pair) or on a TCP port (the Virtual SIP Modem's
//! network port), then the PNM Plus session with the phone.
//!
//! The PNM wire protocol, as PNM Plus 2.2 sends it (decompiled CommCtl module) and as the
//! Elcotel Series 5 firmware accepts it (checked against the firmware in the Elcotel emulator):
//!
//!   command   PC: 02 <cmd> 03
//!   logon     PC: 02 <station id digits as values 0-9> 03          phone: 07
//!   password  PC: 02 90 03                                         phone: 06
//!             PC: <len lo> <len hi> <ANI 7 digits> 01 <min hour wday day month year BCD>
//!                 <enc(len)> <enc(8 password chars)> <enc(sum)> <sum>   phone: 06 (15 = refused)
//!   queries   PC: 02 <cmd> 03     phone: 06 <len lo> <len hi> <data> <sum>
//!   rate file PC: 02 A3 03 / 06, frame(52-byte header) / 06, then per block of the packed
//!             body: 02 A3 03 / 06, frame(block) / 06; the last block ends 00 00
//!   store     PC: 02 29 03        phone: 06 ... 0B (stored in EEPROM) or 0C (refused)
//!   reload    PC: 02 08 03        phone: 06
//!   hang up   PC: 02 01 03
//!
//! Reply layouts (offsets into <data>) come from how PNM Plus reads them: versions
//! (CommCtlMfunbGetFonVers), status = cashbox, alarms, clock and totalizer (fn2132),
//! call counters (sub2033), SMDR call records (CommCtlMfunbRetrieveSMDR).

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const STX: u8 = 0x02;
pub const ETX: u8 = 0x03;
pub const ACK: u8 = 0x06;
pub const LOGON_OK: u8 = 0x07;
pub const STORED: u8 = 0x0B;
pub const NOT_STORED: u8 = 0x0C;
pub const NAK: u8 = 0x15;

/// The PNM command bytes. The first five are confirmed with the phone's firmware; the
/// others are filled in from probing the firmware and can be changed in Modem settings.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PnmCodes {
    pub password: u8,
    pub download: u8,
    pub store: u8,
    pub reload: u8,
    pub hangup: u8,
    pub versions: Option<u8>,
    pub status: Option<u8>,
    pub counters: Option<u8>,
    pub smdr_header: Option<u8>,
    pub smdr_block: Option<u8>,
    pub get_time: Option<u8>,
    pub set_time: Option<u8>,
    pub ram_image: Option<u8>,
}

/// Defaults from probing the stock Series 5 firmware (PCM 05.04.04) one command at a time:
/// 09 clock, 13 versions, 20 status, 21 call counters and 25 a 507-byte data block answer
/// with framed data; 0D and 0E answer 0F ("nothing to send"), which is how PNM's call-record
/// request reports none, so they are taken as the SMDR header and block requests (inferred).
/// The set-clock command wasn't identified and is left blank.
impl Default for PnmCodes {
    fn default() -> Self {
        PnmCodes {
            password: 0x90,
            download: 0xA3,
            store: 0x29,
            reload: 0x08,
            hangup: 0x01,
            versions: Some(0x13),
            status: Some(0x20),
            counters: Some(0x21),
            smdr_header: Some(0x0D),
            smdr_block: Some(0x0E),
            get_time: Some(0x09),
            set_time: None,
            ram_image: Some(0x25),
        }
    }
}

/// Pause between the modem's CONNECT and the logon (PNM Plus: 0.5 s + 3 s).
pub const CONNECT_SETTLE: Duration = Duration::from_millis(3500);

/// What the firmware answers to a command it doesn't have.
pub const UNKNOWN_CMD: u8 = 0x18;
/// "Nothing to send" (no call records).
pub const NOTHING: u8 = 0x0F;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
pub enum Connection {
    /// No longer offered; kept so older saved settings still load (treated as Serial).
    Simulated,
    /// A Hayes modem on a serial port (COMx).
    #[default]
    Serial,
    /// A modem reached over TCP (the Virtual SIP Modem's network port).
    Tcp,
    /// The built-in SIP modem (as in Telelink Rewritten): calls over SIP with modem tones.
    Sip,
}

impl Connection {
    /// Old saved settings may still say Simulated, which is no longer offered: use the serial modem.
    pub fn or_serial(self) -> Connection {
        if self == Connection::Simulated {
            Connection::Serial
        } else {
            self
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Connection::Simulated => "Simulated (no modem)",
            Connection::Serial => "Modem on a serial port",
            Connection::Tcp => "Modem on a network port (TCP)",
            Connection::Sip => "Built-in SIP modem",
        }
    }
}

/// Everything a call needs, copied from the settings and the phone's record.
#[derive(Clone, Debug)]
pub struct CallSetup {
    pub connection: Connection,
    pub serial_port: String,
    pub baud: u32,
    pub tcp_addr: String,
    pub init: Vec<String>,
    pub dial_command: String,
    pub target: String,
    pub connect_timeout: u32,
    pub station_id: String,
    pub password: String,
    pub ani: String,
    pub codes: PnmCodes,
    /// The SIP account, for the built-in SIP modem (or why there isn't one).
    pub sip: Result<crate::sip::SipConfig, String>,
}

/// One thing to do while connected.
#[derive(Clone, Debug)]
pub enum Op {
    Versions,
    Status,
    Counters,
    Smdr,
    GetTime,
    SetTime,
    SendRates { name: String, bytes: Vec<u8> },
    Store,
    RamImage,
    Raw(u8),
}

impl Op {
    pub fn label(&self) -> String {
        match self {
            Op::Versions => "software versions".into(),
            Op::Status => "status (cashbox, alarms, totalizer)".into(),
            Op::Counters => "call counters".into(),
            Op::Smdr => "call records (SMDR)".into(),
            Op::GetTime => "phone clock".into(),
            Op::SetTime => "set the phone's clock".into(),
            Op::SendRates { name, .. } => format!("rate file {name}"),
            Op::Store => "store settings in EEPROM".into(),
            Op::RamImage => "phone data block (RAM image)".into(),
            Op::Raw(c) => format!("command {c:02X}"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Versions {
    pub program: String,
    pub chip: String,
    pub serial: String,
    pub model_code: String,
}

#[derive(Clone, Debug, Default)]
pub struct PhoneStatus {
    pub cashbox: f64,
    pub last_collected: f64,
    pub totalizer: f64,
    pub alarms: Vec<u8>,
    pub clock: String,
}

#[derive(Clone, Debug, Default)]
pub struct SmdrRecord {
    pub dialed: String,
    pub date: String,
    pub duration_secs: u32,
    pub price: u32,
    pub call_type: u8,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub result: String,
    pub connected: bool,
    pub logged_on: bool,
    pub versions: Option<Versions>,
    pub status: Option<PhoneStatus>,
    /// The 101 raw call counters, by call type.
    pub counters: Option<Vec<u32>>,
    pub smdr: Vec<SmdrRecord>,
    pub smdr_done: bool,
    pub clock: Option<String>,
    pub clock_set: bool,
    pub rates_sent: Option<String>,
    pub stored: Option<bool>,
    pub raw: Vec<(u8, Vec<u8>)>,
    pub ram_image: Option<Vec<u8>>,
    pub skipped: Vec<String>,
}

pub enum LinkEvent {
    Log(String),
    Status(String),
    Done(Box<Report>),
}

// ------------------------------------------------------------------ wire

pub trait Wire: Send {
    fn send(&mut self, bytes: &[u8]) -> std::io::Result<()>;
    /// Whatever arrives within `wait` (empty when nothing does).
    fn recv(&mut self, wait: Duration) -> std::io::Result<Vec<u8>>;
    /// Drop the line (DTR off) where the connection can.
    fn drop_line(&mut self) {}
}

struct SerialWire {
    port: Box<dyn serialport::SerialPort>,
}

impl Wire for SerialWire {
    fn send(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.port.write_all(bytes)?;
        self.port.flush()
    }
    fn recv(&mut self, wait: Duration) -> std::io::Result<Vec<u8>> {
        let end = Instant::now() + wait;
        let mut out = Vec::new();
        let mut buf = [0u8; 512];
        loop {
            match self.port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    out.extend_from_slice(&buf[..n]);
                    // keep reading what is already there
                    continue;
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(e),
            }
            if !out.is_empty() || Instant::now() >= end {
                return Ok(out);
            }
        }
    }
    fn drop_line(&mut self) {
        let _ = self.port.write_data_terminal_ready(false);
        std::thread::sleep(Duration::from_millis(600));
        let _ = self.port.write_data_terminal_ready(true);
    }
}

struct TcpWire {
    s: std::net::TcpStream,
}

impl Wire for TcpWire {
    fn send(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.s.write_all(bytes)?;
        self.s.flush()
    }
    fn recv(&mut self, wait: Duration) -> std::io::Result<Vec<u8>> {
        let end = Instant::now() + wait;
        let mut out = Vec::new();
        let mut buf = [0u8; 512];
        loop {
            match self.s.read(&mut buf) {
                Ok(0) => {
                    if out.is_empty() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::ConnectionAborted,
                            "the modem closed the network connection",
                        ));
                    }
                    return Ok(out);
                }
                Ok(n) => {
                    out.extend_from_slice(&buf[..n]);
                    continue;
                }
                Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                Err(e) => return Err(e),
            }
            if !out.is_empty() || Instant::now() >= end {
                return Ok(out);
            }
        }
    }
}

/// The built-in SIP modem, driven with AT commands like the others.
struct SipWire {
    m: crate::sip::SipModem,
}

impl Wire for SipWire {
    fn send(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.m.write(bytes);
        Ok(())
    }
    fn recv(&mut self, wait: Duration) -> std::io::Result<Vec<u8>> {
        let end = Instant::now() + wait;
        loop {
            let b = self.m.read();
            if !b.is_empty() || Instant::now() >= end {
                return Ok(b);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

pub fn open_wire(setup: &CallSetup) -> Result<Box<dyn Wire>, String> {
    match setup.connection {
        Connection::Sip => {
            let cfg = setup.sip.clone()?;
            if cfg.server.trim().is_empty() || cfg.user.trim().is_empty() {
                return Err("No SIP account: fill in the server and user name in Modem Settings".into());
            }
            Ok(Box::new(SipWire {
                m: crate::sip::SipModem::start(cfg),
            }))
        }
        Connection::Serial => {
            let port = serialport::new(setup.serial_port.trim(), setup.baud.max(300))
                .data_bits(serialport::DataBits::Eight)
                .parity(serialport::Parity::None)
                .stop_bits(serialport::StopBits::One)
                .flow_control(serialport::FlowControl::None)
                .timeout(Duration::from_millis(40))
                .open()
                .map_err(|e| format!("Could not open {}: {}", setup.serial_port, e))?;
            let mut w = SerialWire { port };
            let _ = w.port.write_data_terminal_ready(true);
            let _ = w.port.write_request_to_send(true);
            Ok(Box::new(w))
        }
        Connection::Tcp => {
            use std::net::ToSocketAddrs;
            let addr = setup
                .tcp_addr
                .trim()
                .to_socket_addrs()
                .map_err(|e| format!("Bad modem address {}: {}", setup.tcp_addr, e))?
                .next()
                .ok_or_else(|| format!("Bad modem address {}", setup.tcp_addr))?;
            let s = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(|e| {
                format!(
                    "Could not reach the modem at {}: {} (is its network port turned on?)",
                    setup.tcp_addr, e
                )
            })?;
            let _ = s.set_nodelay(true);
            let _ = s.set_read_timeout(Some(Duration::from_millis(40)));
            Ok(Box::new(TcpWire { s }))
        }
        Connection::Simulated => Err("No connection chosen: pick one in Modem Settings".into()),
    }
}

// ------------------------------------------------------------------ framing

pub fn frame(data: &[u8]) -> Vec<u8> {
    let n = data.len();
    let mut v = vec![(n % 256) as u8, (n / 256) as u8];
    v.extend_from_slice(data);
    let sum: u32 = data.iter().map(|b| *b as u32).sum();
    v.push((sum % 256) as u8);
    v
}

/// PNM's password byte coding (fn21DD): invert, then rotate right by one bit.
pub fn enc(x: u8) -> u8 {
    let v = 255 - x;
    (v >> 1) | ((v & 1) << 7)
}

pub fn bcd(x: u32) -> u8 {
    (((x / 10) % 10) * 16 + x % 10) as u8
}

fn unbcd(b: u8) -> u32 {
    (b >> 4) as u32 * 10 + (b & 15) as u32
}

/// The phone's clock as PNM sends it: minute, hour, weekday (Sunday = 1), day, month, year, BCD.
pub fn clock_bytes(t: chrono::NaiveDateTime) -> [u8; 6] {
    use chrono::{Datelike, Timelike};
    [
        bcd(t.minute()),
        bcd(t.hour()),
        t.weekday().number_from_sunday() as u8,
        bcd(t.day()),
        bcd(t.month()),
        bcd((t.year() % 100) as u32),
    ]
}

fn clock_text(d: &[u8]) -> String {
    if d.len() < 6 {
        return String::new();
    }
    let y = unbcd(d[5]);
    let year = if y > 50 { 1900 + y } else { 2000 + y };
    format!(
        "{year:04}-{:02}-{:02} {:02}:{:02}",
        unbcd(d[4]),
        unbcd(d[3]),
        unbcd(d[1]),
        unbcd(d[0])
    )
}

/// Logon message: the station id's digits as byte values (a 10-digit id is packed as PNM does).
pub fn logon_message(station_id: &str) -> Vec<u8> {
    let mut id: String = station_id.chars().filter(|c| c.is_ascii_digit()).collect();
    while id.len() < 4 {
        id.insert(0, '0');
    }
    let mut v = vec![STX];
    let digits: Vec<u8> = id.bytes().map(|c| c - b'0').collect();
    if digits.len() == 10 {
        for p in digits.chunks(2) {
            let hi = if v.len() == 1 {
                (p[0].wrapping_sub(2)) * 16 + 128
            } else {
                p[0] * 16
            };
            v.push(hi.wrapping_add(p[1]));
        }
    } else {
        v.extend(digits);
    }
    v.push(ETX);
    v
}

/// The password frame (CommCtlMfunbLogon2Fon).
pub fn password_frame(ani: &str, password: &str, now: chrono::NaiveDateTime) -> Vec<u8> {
    let digits: Vec<u8> = ani
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();
    let mut d: Vec<u8> = if digits.len() >= 7 {
        digits[digits.len() - 7..].to_vec()
    } else {
        let mut v = vec![0; 7 - digits.len()];
        v.extend(digits);
        v
    };
    d.push(1);
    d.extend_from_slice(&clock_bytes(now));
    let pwd: Vec<u8> = password.bytes().take(8).collect();
    d.push(enc(pwd.len() as u8));
    let mut s = pwd.len() as u32;
    for i in 0..8 {
        let c = pwd.get(i).copied().unwrap_or(0);
        s += c as u32;
        d.push(enc(c));
    }
    d.push(enc((s % 256) as u8));
    frame(&d)
}

/// Split a .R94 rate file's packed body into download blocks (CommCtlMfunnSendAFile).
pub const RATE_BODY_START: usize = 268;
pub fn rate_blocks(file: &[u8]) -> Vec<Vec<u8>> {
    let body = &file[RATE_BODY_START.min(file.len())..];
    let mut blocks = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    for &b in body {
        if cur.len() == 1024 || (cur.len() == 1023 && b == 0) {
            blocks.push(std::mem::take(&mut cur));
        }
        cur.push(b);
    }
    cur.push(0);
    cur.push(0);
    blocks.push(cur);
    blocks
}

// ------------------------------------------------------------------ decoding replies

pub fn decode_versions(d: &[u8]) -> Versions {
    let num = |s: &[u8]| s.iter().map(|b| format!("{b:02}")).collect::<Vec<_>>().join(".");
    let letter = |b: Option<&u8>| {
        b.filter(|b| **b != 0 && b.is_ascii_graphic())
            .map(|b| (*b as char).to_string())
            .unwrap_or_default()
    };
    let mut v = Versions::default();
    if d.len() >= 3 {
        v.program = num(&d[0..3]) + &letter(d.get(8));
    }
    if d.len() >= 8 {
        v.serial = d[3..8].iter().map(|b| format!("{}{}", b >> 4, b & 15)).collect();
    }
    if d.len() >= 12 {
        v.chip = num(&d[9..12]) + &letter(d.get(12));
    }
    if d.len() >= 15 {
        v.model_code = format!("{:02}{:02}", d[13], d[14]);
    }
    v
}

pub fn decode_status(d: &[u8]) -> Option<PhoneStatus> {
    if d.len() < 48 {
        return None;
    }
    let u16le = |i: usize| d[i] as u32 | (d[i + 1] as u32) << 8;
    let u32le = |i: usize| d[i] as u64 | (d[i + 1] as u64) << 8 | (d[i + 2] as u64) << 16 | (d[i + 3] as u64) << 24;
    Some(PhoneStatus {
        cashbox: u16le(0) as f64 * 0.05,
        last_collected: u16le(2) as f64 * 0.05,
        alarms: (0..32u8).filter(|k| d[4 + *k as usize] != 0).collect(),
        clock: clock_text(&d[36..42]),
        totalizer: u32le(44) as f64 * 0.05,
    })
}

pub fn decode_counters(d: &[u8]) -> Option<Vec<u32>> {
    if d.len() < 202 {
        return None;
    }
    Some((0..101).map(|i| d[2 * i] as u32 | (d[2 * i + 1] as u32) << 8).collect())
}

/// PNM's call-count categories (sub2033), from the 101 counters (1-based call types).
pub fn counter_categories(c: &[u32]) -> [u32; 8] {
    let g = |i: usize| c.get(i - 1).copied().unwrap_or(0);
    let total: u32 = c.iter().sum();
    let local = g(5) + g(20);
    let other_coin = g(6) + g(7) + g(8) + g(9);
    let zero_plus = (41..=56).map(g).sum::<u32>() + g(72) + g(73) + g(74) + g(75);
    let zero_minus = g(34) + g(37) + g(38);
    let dial_around = g(10) + g(11) + g(14);
    let debit = g(100);
    let dataport = g(99);
    let other = total.saturating_sub(local + other_coin + zero_plus + zero_minus + dial_around + debit + dataport);
    [
        local,
        other_coin,
        zero_plus,
        zero_minus,
        dial_around,
        debit,
        dataport,
        other,
    ]
}

fn digits_of(b: u8) -> String {
    let mut s = String::new();
    for n in [b >> 4, b & 15] {
        match n {
            0..=9 => s.push((b'0' + n) as char),
            11 => s.push('*'),
            12 => s.push('#'),
            _ => {}
        }
    }
    s
}

pub fn decode_smdr(data: &[u8]) -> Vec<SmdrRecord> {
    data.chunks(64)
        .filter(|r| r.len() == 64)
        .map(|r| {
            let y = unbcd(r[46]);
            let year = if y > 35 { 1900 + y } else { 2000 + y };
            SmdrRecord {
                dialed: r[12..20].iter().map(|b| digits_of(*b)).collect(),
                date: format!(
                    "{year:04}-{:02}-{:02} {:02}:{:02}",
                    unbcd(r[20]),
                    unbcd(r[21]),
                    unbcd(r[22]),
                    unbcd(r[23])
                ),
                duration_secs: r[25] as u32 | (r[26] as u32) << 8,
                price: r[27] as u32 | (r[28] as u32) << 8,
                call_type: r[29],
            }
        })
        .collect()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(" ")
}

// ------------------------------------------------------------------ the call

#[derive(Debug)]
enum Fail {
    Cancelled,
    Lost,
    Io(String),
    Timeout(&'static str),
    Refused(String),
}

impl std::fmt::Display for Fail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fail::Cancelled => write!(f, "Process canceled by user"),
            Fail::Lost => write!(f, "Lost carrier"),
            Fail::Io(e) => write!(f, "{e}"),
            Fail::Timeout(w) => write!(f, "No response to {w}"),
            Fail::Refused(w) => write!(f, "{w}"),
        }
    }
}

pub struct Session<'a> {
    wire: Box<dyn Wire + 'a>,
    buf: Vec<u8>,
    tx: Option<Sender<LinkEvent>>,
    cancel: Arc<AtomicBool>,
    pub log: Vec<String>,
    t0: Instant,
    data_mode: bool,
}

impl<'a> Session<'a> {
    pub fn new(wire: Box<dyn Wire + 'a>, tx: Option<Sender<LinkEvent>>, cancel: Arc<AtomicBool>) -> Self {
        Session {
            wire,
            buf: Vec::new(),
            tx,
            cancel,
            log: Vec::new(),
            t0: Instant::now(),
            data_mode: false,
        }
    }

    fn note(&mut self, s: impl Into<String>) {
        let line = format!("{:7.2}s  {}", self.t0.elapsed().as_secs_f64(), s.into());
        if let Some(tx) = &self.tx {
            let _ = tx.send(LinkEvent::Log(line.clone()));
        }
        self.log.push(line);
    }

    fn status(&self, s: &str) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(LinkEvent::Status(s.to_string()));
        }
    }

    fn check(&self) -> Result<(), Fail> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(Fail::Cancelled)
        } else {
            Ok(())
        }
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), Fail> {
        self.check()?;
        let shown = if self.data_mode {
            format!(
                "PC -> phone: {}{}",
                hex(&bytes[..bytes.len().min(24)]),
                if bytes.len() > 24 {
                    format!(" ... ({} bytes)", bytes.len())
                } else {
                    String::new()
                }
            )
        } else {
            format!("PC -> modem: {}", String::from_utf8_lossy(bytes).trim())
        };
        self.note(shown);
        self.wire.send(bytes).map_err(|e| Fail::Io(e.to_string()))
    }

    fn pull(&mut self, wait: Duration) -> Result<usize, Fail> {
        self.check()?;
        let got = self.wire.recv(wait).map_err(|e| Fail::Io(e.to_string()))?;
        if !got.is_empty() {
            let shown = if self.data_mode {
                format!(
                    "phone -> PC: {}",
                    hex(&got[..got.len().min(32)])
                        + &if got.len() > 32 {
                            format!(" ... ({} bytes)", got.len())
                        } else {
                            String::new()
                        }
                )
            } else {
                format!(
                    "modem -> PC: {}",
                    String::from_utf8_lossy(&got).replace(['\r', '\n'], " ").trim()
                )
            };
            self.note(shown);
            self.buf.extend_from_slice(&got);
            if self.data_mode && contains(&self.buf, b"NO CARRIER") {
                return Err(Fail::Lost);
            }
        }
        Ok(got.len())
    }

    /// Wait for one of the modem's result words; returns the line that matched.
    fn wait_result(&mut self, words: &[&str], secs: f64) -> Result<String, Fail> {
        let end = Instant::now() + Duration::from_secs_f64(secs);
        loop {
            let text = String::from_utf8_lossy(&self.buf).to_string();
            for line in text.split(['\r', '\n']) {
                let l = line.trim();
                if let Some(w) = words.iter().find(|w| l == **w || l.starts_with(&format!("{} ", w))) {
                    let _ = w;
                    let found = l.to_string();
                    // drop everything up to and including that line
                    if let Some(p) = text.find(line) {
                        let cut = (p + line.len()).min(self.buf.len());
                        self.buf.drain(..cut);
                    } else {
                        self.buf.clear();
                    }
                    return Ok(found);
                }
            }
            if Instant::now() >= end {
                return Err(Fail::Timeout("the modem"));
            }
            self.pull(Duration::from_millis(100))?;
        }
    }

    /// Wait for one of `bytes` from the phone; returns it (bytes before it are dropped).
    fn wait_byte(&mut self, bytes: &[u8], secs: f64, what: &'static str) -> Result<u8, Fail> {
        let end = Instant::now() + Duration::from_secs_f64(secs);
        loop {
            if let Some(p) = self.buf.iter().position(|b| bytes.contains(b)) {
                let b = self.buf[p];
                self.buf.drain(..=p);
                return Ok(b);
            }
            if Instant::now() >= end {
                return Err(Fail::Timeout(what));
            }
            self.pull(Duration::from_millis(100))?;
        }
    }

    fn need(&mut self, n: usize, secs: f64, what: &'static str) -> Result<(), Fail> {
        let end = Instant::now() + Duration::from_secs_f64(secs);
        while self.buf.len() < n {
            if Instant::now() >= end {
                return Err(Fail::Timeout(what));
            }
            self.pull(Duration::from_millis(100))?;
        }
        Ok(())
    }

    /// Send 02 cmd 03 and read the framed reply: 06 <len lo> <len hi> <data> <sum>.
    /// Ok(Err(code)) when the phone answers with another single byte (for example "no more").
    fn query(&mut self, cmd: u8, what: &'static str) -> Result<Result<Vec<u8>, u8>, Fail> {
        self.buf.clear();
        self.send(&[STX, cmd, ETX])?;
        let end = Instant::now() + Duration::from_secs(12);
        loop {
            self.need(
                1,
                end.saturating_duration_since(Instant::now()).as_secs_f64().max(0.1),
                what,
            )?;
            let first = self.buf[0];
            if first != ACK {
                self.buf.remove(0);
                if [NAK, 0x04, NOT_STORED, NOTHING, UNKNOWN_CMD, 0x19].contains(&first) {
                    if first == UNKNOWN_CMD {
                        self.note(format!("the phone doesn't know command {cmd:02X} ({what})"));
                    }
                    return Ok(Err(first));
                }
                continue; // line noise
            }
            self.need(3, 8.0, what)?;
            let len = self.buf[1] as usize | (self.buf[2] as usize) << 8;
            if len == 0 {
                self.buf.drain(..3);
                // a bare 06 00 00 (+ sum) means "nothing"
                let _ = self.need(1, 0.5, what);
                if !self.buf.is_empty() {
                    self.buf.remove(0);
                }
                return Ok(Ok(Vec::new()));
            }
            self.need(3 + len + 1, 20.0, what)?;
            let data = self.buf[3..3 + len].to_vec();
            let sum = self.buf[3 + len];
            self.buf.drain(..3 + len + 1);
            let want = (data.iter().map(|b| *b as u32).sum::<u32>() % 256) as u8;
            if sum != want {
                self.note(format!(
                    "checksum mismatch on {what} (got {sum:02X}, expected {want:02X})"
                ));
            }
            return Ok(Ok(data));
        }
    }

    fn command_ack(&mut self, cmd: u8, what: &'static str) -> Result<(), Fail> {
        self.buf.clear();
        self.send(&[STX, cmd, ETX])?;
        match self.wait_byte(&[ACK, NAK], 10.0, what)? {
            ACK => Ok(()),
            _ => Err(Fail::Refused(format!("Phone rejected {what}"))),
        }
    }

    fn data_ack(&mut self, data: &[u8], what: &'static str) -> Result<bool, Fail> {
        self.buf.clear();
        self.send(data)?;
        Ok(self.wait_byte(&[ACK, NAK], 35.0, what)? == ACK)
    }

    // ---------------- modem

    fn modem_command(&mut self, cmd: &str, secs: f64) -> Result<String, Fail> {
        self.buf.clear();
        self.send(format!("{cmd}\r").as_bytes())?;
        self.wait_result(&["OK", "ERROR"], secs)
    }

    fn dial(&mut self, setup: &CallSetup) -> Result<(), Fail> {
        self.status("Initializing modem");
        for c in ["AT"].iter().map(|s| s.to_string()).chain(setup.init.iter().cloned()) {
            let c = c.trim().to_string();
            if c.is_empty() {
                continue;
            }
            match self.modem_command(&c, 3.0) {
                Ok(r) if r == "OK" => {}
                Ok(r) => self.note(format!("modem answered {r} to {c}")),
                Err(Fail::Timeout(_)) => self.note(format!("no answer from the modem to {c}")),
                Err(e) => return Err(e),
            }
        }
        self.status("Dialing");
        self.buf.clear();
        let dial = format!("{}{}\r", setup.dial_command.trim(), setup.target.trim());
        self.send(dial.as_bytes())?;
        let secs = setup.connect_timeout.max(10);
        let r = match self.wait_result(
            &["CONNECT", "NO CARRIER", "BUSY", "NO ANSWER", "NO DIALTONE", "ERROR"],
            secs as f64,
        ) {
            Ok(r) => r,
            Err(Fail::Timeout(_)) => {
                return Err(Fail::Refused(format!(
                    "NO ANSWER (nothing answered within {secs} seconds)"
                )))
            }
            Err(e) => return Err(e),
        };
        if !r.starts_with("CONNECT") {
            return Err(Fail::Refused(r));
        }
        self.status(&r);
        self.data_mode = true;
        // Wait as PNM Plus does after CONNECT (half a second, then 3 seconds) so the phone is
        // listening before the station id arrives; a logon sent sooner is ignored. Then drop
        // anything left over.
        self.note(format!(
            "waiting {:.1} s for the phone to get ready",
            CONNECT_SETTLE.as_secs_f64()
        ));
        let settle_end = Instant::now() + CONNECT_SETTLE;
        while Instant::now() < settle_end {
            self.pull(Duration::from_millis(100).min(settle_end.saturating_duration_since(Instant::now())))?;
        }
        self.buf.clear();
        Ok(())
    }

    fn hang_up(&mut self, codes: &PnmCodes) {
        if self.data_mode {
            let _ = self.send(&[STX, codes.hangup, ETX]);
            let _ = self.pull(Duration::from_millis(1200));
            self.data_mode = false;
            // back to command mode and hang up, as PNM does (+++ then ATH)
            std::thread::sleep(Duration::from_millis(1100));
            self.buf.clear();
            let _ = self.wire.send(b"+++");
            self.note("PC -> modem: +++");
            std::thread::sleep(Duration::from_millis(1100));
            let _ = self.wait_result(&["OK", "NO CARRIER"], 2.0);
        }
        self.buf.clear();
        let _ = self.send(b"ATH\r");
        let _ = self.wait_result(&["OK", "NO CARRIER", "ERROR"], 3.0);
        self.wire.drop_line();
    }

    // ---------------- PNM

    fn logon(&mut self, setup: &CallSetup) -> Result<(), Fail> {
        self.status("Attempting logon");
        let msg = logon_message(&setup.station_id);
        for attempt in 0..3 {
            self.buf.clear();
            self.send(&msg)?;
            match self.wait_byte(&[LOGON_OK], 4.0, "the logon") {
                Ok(_) => break,
                Err(Fail::Timeout(_)) if attempt < 2 => continue,
                Err(Fail::Timeout(_)) => {
                    return Err(Fail::Refused(format!(
                        "Phone did not accept station id {} (check the Phone ID; a modem sending 7E1 instead of 8N1 also causes this)",
                        setup.station_id
                    )))
                }
                Err(e) => return Err(e),
            }
        }
        self.status("Sending password");
        self.command_ack(setup.codes.password, "the password command")?;
        let f = password_frame(&setup.ani, &setup.password, chrono::Local::now().naive_local());
        if !self.data_ack(&f, "the password")? {
            return Err(Fail::Refused("Phone did not accept password".into()));
        }
        self.note("Connected to phone");
        Ok(())
    }

    fn send_rates(&mut self, codes: &PnmCodes, name: &str, file: &[u8]) -> Result<(), Fail> {
        self.status(&format!("Downloading rate file {name}"));
        self.command_ack(codes.download, "the download command")?;
        let mut head = file[..52.min(file.len())].to_vec();
        head.resize(52, 0);
        if !self.data_ack(&frame(&head), "the rate file header")? {
            return Err(Fail::Refused("Phone refused the rate file header".into()));
        }
        let blocks = rate_blocks(file);
        for (i, b) in blocks.iter().enumerate() {
            let mut tries = 0;
            loop {
                self.status(&format!("Downloading {name}: block {} of {}", i + 1, blocks.len()));
                self.command_ack(codes.download, "the block command")?;
                if self.data_ack(&frame(b), "a rate file block")? {
                    break;
                }
                tries += 1;
                if tries > 3 {
                    return Err(Fail::Refused("Phone kept rejecting a rate file block".into()));
                }
                self.note("block rejected, sending it again");
            }
        }
        Ok(())
    }

    fn store(&mut self, codes: &PnmCodes) -> Result<bool, Fail> {
        self.status("Storing settings in the phone's EEPROM");
        self.buf.clear();
        self.send(&[STX, codes.store, ETX])?;
        let b = self.wait_byte(&[STORED, NOT_STORED, NAK], 45.0, "the store command")?;
        if b != STORED {
            self.note("phone refused to store (settings stay in battery RAM)");
            return Ok(false);
        }
        self.command_ack(codes.reload, "the reload command")?;
        Ok(true)
    }

    fn run_ops(&mut self, setup: &CallSetup, ops: &[Op], rep: &mut Report) -> Result<(), Fail> {
        let c = setup.codes.clone();
        for op in ops {
            self.check()?;
            let missing = |rep: &mut Report, s: &mut Self| {
                let t = format!(
                    "{}: the phone command for this isn't known yet (set it in Modem settings)",
                    op.label()
                );
                s.note(t.clone());
                rep.skipped.push(t);
            };
            match op {
                Op::Versions => match c.versions {
                    Some(cmd) => {
                        self.status("Getting software versions");
                        if let Ok(d) = self.query(cmd, "the version request")? {
                            rep.versions = Some(decode_versions(&d));
                        }
                    }
                    None => missing(rep, self),
                },
                Op::Status => match c.status {
                    Some(cmd) => {
                        self.status("Getting phone status");
                        if let Ok(d) = self.query(cmd, "the status request")? {
                            rep.status = decode_status(&d);
                            if rep.status.is_none() {
                                self.note(format!("status reply too short ({} bytes)", d.len()));
                            }
                        }
                    }
                    None => missing(rep, self),
                },
                Op::Counters => match c.counters {
                    Some(cmd) => {
                        self.status("Getting call counters");
                        if let Ok(d) = self.query(cmd, "the call counter request")? {
                            rep.counters = decode_counters(&d);
                        }
                    }
                    None => missing(rep, self),
                },
                Op::GetTime => match c.get_time {
                    Some(cmd) => {
                        if let Ok(d) = self.query(cmd, "the time request")? {
                            rep.clock = Some(clock_text(&d));
                        }
                    }
                    None => missing(rep, self),
                },
                Op::SetTime => match c.set_time {
                    Some(cmd) => {
                        self.status("Sending new time");
                        self.command_ack(cmd, "the set time command")?;
                        let t = chrono::Local::now().naive_local();
                        rep.clock_set = self.data_ack(&frame(&clock_bytes(t)), "the new time")?;
                    }
                    None => missing(rep, self),
                },
                Op::Smdr => match (c.smdr_header, c.smdr_block) {
                    (Some(h), Some(b)) => {
                        self.status("Getting SMDR records");
                        match self.query(h, "the SMDR request")? {
                            Ok(_) => {
                                let mut all = Vec::new();
                                for _ in 0..400 {
                                    match self.query(b, "an SMDR block")? {
                                        Ok(d) if !d.is_empty() => all.extend(d),
                                        _ => break,
                                    }
                                }
                                rep.smdr = decode_smdr(&all);
                                rep.smdr_done = true;
                            }
                            Err(_) => {
                                self.note("No SMDR to transmit");
                                rep.smdr_done = true;
                            }
                        }
                    }
                    _ => missing(rep, self),
                },
                Op::SendRates { name, bytes } => {
                    self.send_rates(&c, name, bytes)?;
                    rep.rates_sent = Some(name.clone());
                }
                Op::Store => {
                    rep.stored = Some(self.store(&c)?);
                }
                Op::RamImage => match c.ram_image {
                    Some(cmd) => {
                        self.status("Reading the phone's data block");
                        if let Ok(d) = self.query(cmd, "the data block request")? {
                            rep.ram_image = Some(d);
                        }
                    }
                    None => missing(rep, self),
                },
                Op::Raw(cmd) => {
                    self.buf.clear();
                    self.send(&[STX, *cmd, ETX])?;
                    let end = Instant::now() + Duration::from_secs(5);
                    while Instant::now() < end {
                        self.pull(Duration::from_millis(200))?;
                    }
                    rep.raw.push((*cmd, std::mem::take(&mut self.buf)));
                }
            }
        }
        Ok(())
    }

    /// The whole call: dial, log on, do the operations, hang up.
    pub fn call(&mut self, setup: &CallSetup, ops: &[Op]) -> Report {
        let mut rep = Report::default();
        let r = (|| -> Result<(), Fail> {
            self.dial(setup)?;
            rep.connected = true;
            self.logon(setup)?;
            rep.logged_on = true;
            self.run_ops(setup, ops, &mut rep)
        })();
        rep.result = match &r {
            Ok(()) => "Completed".into(),
            Err(e) => e.to_string(),
        };
        if let Err(e) = &r {
            self.note(format!("Terminating call: {e}"));
        }
        self.status("Hanging up");
        self.hang_up(&setup.codes);
        self.note(format!("Call ended: {}", rep.result));
        rep
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Run one call on a background thread; events arrive on `tx`, ending with `Done`.
pub fn spawn_call(setup: CallSetup, ops: Vec<Op>, tx: Sender<LinkEvent>, cancel: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let rep = match open_wire(&setup) {
            Ok(w) => {
                let mut s = Session::new(w, Some(tx.clone()), cancel);
                s.note(format!(
                    "Opened {}",
                    match setup.connection {
                        Connection::Serial => format!("{} at {} bit/s, 8N1", setup.serial_port, setup.baud),
                        Connection::Tcp => format!("modem at {}", setup.tcp_addr),
                        Connection::Sip => match &setup.sip {
                            Ok(c) => format!(
                                "built-in SIP modem, account {}@{}:{}, characters 8N1",
                                c.user.trim(),
                                c.server.trim(),
                                c.port
                            ),
                            Err(_) => "built-in SIP modem".into(),
                        },
                        Connection::Simulated => "simulator".into(),
                    }
                ));
                s.call(&setup, &ops)
            }
            Err(e) => {
                let _ = tx.send(LinkEvent::Log(e.clone()));
                Report {
                    result: e,
                    ..Default::default()
                }
            }
        };
        let _ = tx.send(LinkEvent::Done(Box::new(rep)));
    });
}

/// Command-line call, for checking a modem and a phone without the window:
///   payphone-manager --call <COMx | host:port> <dial target> <station id> <password> [what...]
/// what: versions status counters smdr clock ram store raw:XX rates:<file.R94>  (default: versions status counters)
pub fn cli_call(args: &[String]) -> i32 {
    if args.len() < 4 {
        eprintln!("usage: --call <COMx | host:port | sip> <dial target> <station id> <password> [versions status counters smdr clock ram store raw:XX rates:FILE]");
        return 2;
    }
    let port = &args[0];
    let sip = port.eq_ignore_ascii_case("sip");
    let tcp = !sip && port.contains(':') && !port.to_ascii_uppercase().starts_with("COM");
    let mut ops = Vec::new();
    for w in &args[4..] {
        match w.as_str() {
            "versions" => ops.push(Op::Versions),
            "status" => ops.push(Op::Status),
            "counters" => ops.push(Op::Counters),
            "smdr" => ops.push(Op::Smdr),
            "clock" => ops.push(Op::GetTime),
            "ram" => ops.push(Op::RamImage),
            "store" => ops.push(Op::Store),
            w if w.starts_with("raw:") => {
                if let Ok(c) = u8::from_str_radix(&w[4..], 16) {
                    ops.push(Op::Raw(c));
                }
            }
            w if w.starts_with("rates:") => match std::fs::read(&w[6..]) {
                Ok(bytes) => ops.push(Op::SendRates {
                    name: w[6..].to_string(),
                    bytes,
                }),
                Err(e) => {
                    eprintln!("can't read {}: {e}", &w[6..]);
                    return 2;
                }
            },
            other => eprintln!("ignoring {other}"),
        }
    }
    if ops.is_empty() {
        ops = vec![Op::Versions, Op::Status, Op::Counters];
    }
    let setup = CallSetup {
        connection: if sip {
            Connection::Sip
        } else if tcp {
            Connection::Tcp
        } else {
            Connection::Serial
        },
        sip: crate::calls::sip_account(&crate::model::Db::load_or_sample().settings),
        serial_port: port.clone(),
        baud: 9600,
        tcp_addr: port.clone(),
        init: std::env::var("MODEM_INIT")
            .map(|s| s.split(';').map(|x| x.to_string()).collect())
            .unwrap_or_default(),
        dial_command: std::env::var("DIAL_CMD").unwrap_or_else(|_| "ATDT".into()),
        target: args[1].clone(),
        connect_timeout: std::env::var("CONNECT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60),
        station_id: args[2].clone(),
        password: args[3].clone(),
        ani: std::env::var("ANI").unwrap_or_else(|_| "5550100".into()),
        codes: PnmCodes::default(),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    spawn_call(setup, ops, tx, Arc::new(AtomicBool::new(false)));
    for ev in rx {
        match ev {
            LinkEvent::Log(l) => println!("{l}"),
            LinkEvent::Status(s) => println!("          [{s}]"),
            LinkEvent::Done(r) => {
                println!("RESULT: {}", r.result);
                if let Some(v) = &r.versions {
                    println!(
                        "versions: program {} chip {} serial {} model {}",
                        v.program, v.chip, v.serial, v.model_code
                    );
                }
                if let Some(s) = &r.status {
                    println!(
                        "status: cashbox ${:.2} last ${:.2} totalizer ${:.2} alarms {:?} clock {}",
                        s.cashbox, s.last_collected, s.totalizer, s.alarms, s.clock
                    );
                }
                if let Some(c) = &r.counters {
                    println!(
                        "call counts (local, other coin, 0+, 0-, dial around, debit, dataport, other): {:?}",
                        counter_categories(c)
                    );
                }
                if let Some(c) = &r.clock {
                    println!("phone clock: {c}");
                }
                if r.smdr_done {
                    println!("SMDR records: {}", r.smdr.len());
                }
                if let Some(b) = &r.ram_image {
                    println!("data block: {} bytes", b.len());
                }
                for (c, b) in &r.raw {
                    println!("command {c:02X} answered: {}", hex(b));
                }
                return if r.result == "Completed" { 0 } else { 1 };
            }
        }
    }
    1
}

/// Serial ports this computer has.
pub fn serial_ports() -> Vec<String> {
    serialport::available_ports()
        .map(|v| v.into_iter().map(|p| p.port_name).collect())
        .unwrap_or_default()
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// A pretend modem + Series 5 phone that follows the protocol above.
    struct FakePhone {
        inbox: Vec<u8>,
        out: Arc<Mutex<VecDeque<u8>>>,
        connected: bool,
        logged_on: bool,
        pwd_cmd: bool,
        expect_frame: Option<&'static str>,
        station: Vec<u8>,
        pub received_blocks: Arc<Mutex<Vec<Vec<u8>>>>,
        smdr_left: usize,
    }

    impl FakePhone {
        fn reply(&self, b: &[u8]) {
            self.out.lock().unwrap().extend(b.iter().copied());
        }
        fn framed(&self, data: &[u8]) {
            let mut v = vec![ACK];
            v.extend(frame(data));
            self.reply(&v);
        }
        fn take_frame(&mut self) -> Option<Vec<u8>> {
            if self.inbox.len() < 3 {
                return None;
            }
            let len = self.inbox[0] as usize | (self.inbox[1] as usize) << 8;
            if self.inbox.len() < len + 3 {
                return None;
            }
            let d = self.inbox[2..2 + len].to_vec();
            self.inbox.drain(..len + 3);
            Some(d)
        }
        fn process(&mut self) {
            loop {
                if !self.connected {
                    let text = String::from_utf8_lossy(&self.inbox).to_string();
                    if let Some(p) = text.find('\r') {
                        let line = text[..p].trim().to_string();
                        self.inbox.drain(..=p);
                        if line.starts_with("ATD") {
                            self.connected = true;
                            self.reply(b"\r\nCONNECT 1200\r\n");
                        } else {
                            self.reply(b"\r\nOK\r\n");
                        }
                        continue;
                    }
                    return;
                }
                if let Some(kind) = self.expect_frame {
                    let Some(d) = self.take_frame() else { return };
                    self.expect_frame = None;
                    match kind {
                        "pwd" => {
                            // ANI(7) 01 clock(6) enc(len) enc(8) enc(sum)
                            let ok = d.len() == 7 + 1 + 6 + 1 + 8 + 1 && d[14] == enc(4) && d[15] == enc(b'1');
                            self.reply(&[if ok { ACK } else { NAK }]);
                            self.logged_on = ok;
                        }
                        "block" => {
                            self.received_blocks.lock().unwrap().push(d);
                            self.reply(&[ACK]);
                        }
                        "time" => self.reply(&[if d.len() == 6 { ACK } else { NAK }]),
                        _ => {}
                    }
                    continue;
                }
                if self.inbox.first() == Some(&b'+') {
                    self.inbox.clear();
                    self.connected = false;
                    self.reply(b"\r\nOK\r\n");
                    return;
                }
                let Some(e) = self.inbox.iter().position(|b| *b == ETX) else {
                    return;
                };
                let msg: Vec<u8> = self.inbox.drain(..=e).collect();
                if msg.first() != Some(&STX) {
                    continue;
                }
                let body = &msg[1..msg.len() - 1];
                if !self.logged_on && !self.pwd_cmd {
                    if body == self.station.as_slice() {
                        self.reply(&[LOGON_OK]);
                        self.pwd_cmd = true;
                    }
                    continue;
                }
                match body.first().copied() {
                    Some(0x90) => {
                        self.reply(&[ACK]);
                        self.expect_frame = Some("pwd");
                    }
                    Some(0xA3) => {
                        self.reply(&[ACK]);
                        self.expect_frame = Some("block");
                    }
                    Some(0x29) => self.reply(&[ACK, STORED]),
                    Some(0x08) => self.reply(&[ACK]),
                    Some(0x01) => {}
                    Some(0x40) => {
                        // versions
                        self.framed(&[5, 4, 4, 0x12, 0x34, 0x56, 0x78, 0x90, 0, 1, 2, 3, 0, 0, 1]);
                    }
                    Some(0x41) => {
                        let mut d = vec![0u8; 48];
                        d[0] = 100; // $5.00 cashbox
                        d[2] = 40; // $2.00 last
                        d[4 + 3] = 1; // alarm 3
                        d[36..42].copy_from_slice(&[0x30, 0x14, 3, 0x17, 0x11, 0x26]);
                        d[44] = 0x10;
                        d[45] = 0x27; // 10000 * 0.05 = $500
                        self.framed(&d);
                    }
                    Some(0x42) => {
                        let mut d = vec![0u8; 202];
                        d[2 * 4] = 7; // counter 5 = local coin
                        d[2 * 98] = 2; // counter 99 = dataport
                        self.framed(&d);
                    }
                    Some(0x43) => self.framed(&[0u8; 30]),
                    Some(0x44) => {
                        if self.smdr_left > 0 {
                            self.smdr_left -= 1;
                            let mut r = vec![0u8; 64];
                            for i in (12..20).chain(47..57) {
                                r[i] = 0xFF;
                            }
                            r[12] = 0x55;
                            r[13] = 0x51;
                            r[14] = 0x21;
                            r[15] = 0x2F;
                            r[20] = 0x11;
                            r[21] = 0x17;
                            r[22] = 0x09;
                            r[23] = 0x05;
                            r[46] = 0x26;
                            r[25] = 60;
                            r[27] = 50;
                            r[29] = 5;
                            self.framed(&r);
                        } else {
                            self.reply(&[0x04]);
                        }
                    }
                    Some(0x45) => {
                        self.reply(&[ACK]);
                        self.expect_frame = Some("time");
                    }
                    _ => self.reply(&[NAK]),
                }
            }
        }
    }

    struct FakeWire(Arc<Mutex<FakePhone>>);
    impl Wire for FakeWire {
        fn send(&mut self, bytes: &[u8]) -> std::io::Result<()> {
            let mut p = self.0.lock().unwrap();
            p.inbox.extend_from_slice(bytes);
            p.process();
            Ok(())
        }
        fn recv(&mut self, _wait: Duration) -> std::io::Result<Vec<u8>> {
            let p = self.0.lock().unwrap();
            let mut o = p.out.lock().unwrap();
            Ok(o.drain(..).collect())
        }
    }

    fn setup() -> CallSetup {
        CallSetup {
            connection: Connection::Tcp,
            sip: Err("not used".into()),
            serial_port: String::new(),
            baud: 1200,
            tcp_addr: String::new(),
            init: vec!["ATZ".into()],
            dial_command: "ATDT".into(),
            target: "5551234".into(),
            connect_timeout: 20,
            station_id: "100217".into(),
            password: "1234".into(),
            ani: "217-555-2011".into(),
            codes: PnmCodes {
                versions: Some(0x40),
                status: Some(0x41),
                counters: Some(0x42),
                smdr_header: Some(0x43),
                smdr_block: Some(0x44),
                set_time: Some(0x45),
                ..Default::default()
            },
        }
    }

    fn fake(station: &str) -> (Arc<Mutex<FakePhone>>, Arc<Mutex<Vec<Vec<u8>>>>) {
        let blocks = Arc::new(Mutex::new(Vec::new()));
        let p = FakePhone {
            inbox: Vec::new(),
            out: Arc::new(Mutex::new(VecDeque::new())),
            connected: false,
            logged_on: false,
            pwd_cmd: false,
            expect_frame: None,
            station: logon_message(station)[1..logon_message(station).len() - 1].to_vec(),
            received_blocks: blocks.clone(),
            smdr_left: 2,
        };
        (Arc::new(Mutex::new(p)), blocks)
    }

    #[test]
    fn password_frame_matches_pnm() {
        let t = chrono::NaiveDate::from_ymd_opt(2026, 11, 17)
            .unwrap()
            .and_hms_opt(14, 30, 0)
            .unwrap();
        let f = password_frame("2175552011", "1234", t);
        // len 24, ANI 5552011, 01, clock, enc(4), enc('1'..'4'), 4 x enc(0), enc(sum), sum
        assert_eq!(f[0], 24);
        assert_eq!(f[1], 0);
        assert_eq!(&f[2..9], &[5, 5, 5, 2, 0, 1, 1]);
        assert_eq!(f[9], 1);
        assert_eq!(&f[10..16], &[0x30, 0x14, 3, 0x17, 0x11, 0x26]);
        assert_eq!(f[16], enc(4));
        assert_eq!(enc(0), 0xFF >> 1 | 0x80); // 255 rotated
        let sum: u32 = f[2..26].iter().map(|b| *b as u32).sum();
        assert_eq!(f[26], (sum % 256) as u8);
    }

    #[test]
    fn logon_encodes_digits() {
        assert_eq!(logon_message("217"), vec![2, 0, 2, 1, 7, 3]);
        assert_eq!(logon_message("100217"), vec![2, 1, 0, 0, 2, 1, 7, 3]);
        assert_eq!(logon_message("5175551234").len(), 7);
    }

    #[test]
    fn full_call_against_a_pretend_phone() {
        let (p, blocks) = fake("100217");
        let mut file = vec![0u8; RATE_BODY_START];
        file.extend((0..1500u32).map(|i| (i % 250 + 1) as u8));
        let ops = vec![
            Op::Versions,
            Op::Status,
            Op::Counters,
            Op::Smdr,
            Op::SetTime,
            Op::SendRates {
                name: "TEST".into(),
                bytes: file,
            },
            Op::Store,
        ];
        let mut s = Session::new(Box::new(FakeWire(p.clone())), None, Arc::new(AtomicBool::new(false)));
        let rep = s.call(&setup(), &ops);
        assert_eq!(rep.result, "Completed", "{}", s.log.join("\n"));
        assert!(rep.logged_on);
        let v = rep.versions.unwrap();
        assert_eq!(v.program, "05.04.04");
        assert_eq!(v.serial, "1234567890");
        let st = rep.status.unwrap();
        assert!((st.cashbox - 5.0).abs() < 1e-9);
        assert!((st.totalizer - 500.0).abs() < 1e-9);
        assert_eq!(st.alarms, vec![3]);
        assert_eq!(st.clock, "2026-11-17 14:30");
        let cats = counter_categories(&rep.counters.unwrap());
        assert_eq!(cats[0], 7);
        assert_eq!(cats[6], 2);
        assert_eq!(rep.smdr.len(), 2);
        assert_eq!(rep.smdr[0].dialed, "5551212");
        assert_eq!(rep.smdr[0].date, "2026-11-17 09:05");
        assert!(rep.clock_set);
        assert_eq!(rep.rates_sent.as_deref(), Some("TEST"));
        assert_eq!(rep.stored, Some(true));
        let b = blocks.lock().unwrap();
        // header + 2 body blocks
        assert_eq!(b.len(), 3);
        assert_eq!(b[0].len(), 52);
        assert!(b[2].ends_with(&[0, 0]));
    }

    #[test]
    fn wrong_password_is_reported() {
        let (p, _) = fake("100217");
        let mut st = setup();
        st.password = "9999".into();
        let mut s = Session::new(Box::new(FakeWire(p)), None, Arc::new(AtomicBool::new(false)));
        let rep = s.call(&st, &[Op::Status]);
        assert!(rep.connected);
        assert!(!rep.logged_on);
        assert_eq!(rep.result, "Phone did not accept password");
    }

    #[test]
    fn unknown_command_is_skipped_not_guessed() {
        let (p, _) = fake("100217");
        let mut st = setup();
        st.codes.counters = None;
        let mut s = Session::new(Box::new(FakeWire(p)), None, Arc::new(AtomicBool::new(false)));
        let rep = s.call(&st, &[Op::Counters]);
        assert_eq!(rep.result, "Completed");
        assert_eq!(rep.skipped.len(), 1);
        assert!(rep.counters.is_none());
    }
}
