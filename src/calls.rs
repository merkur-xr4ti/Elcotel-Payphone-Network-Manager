//! Real calls to payphones: turning the command list into PNM operations, running one call
//! at a time (poll lists queue behind), and saving what the phone reported.

use crate::app::App;
use crate::link::{self, CallSetup, Connection, LinkEvent, Op, Report};
use crate::model::*;
use crate::state::{CallJob, QueuedCall};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// PNM's call-count categories in the order of `CallCounts::rows`.
fn counts_from(cats: [u32; 8]) -> CallCounts {
    CallCounts {
        local_coin: cats[0],
        other_coin: cats[1],
        zero_plus: cats[2],
        zero_minus: cats[3],
        dial_around: cats[4],
        debit_card: cats[5],
        dataport: cats[6],
        other: cats[7],
    }
}

/// The SIP account the built-in modem calls from, as typed in Modem Settings.
pub fn sip_account(s: &Settings) -> Result<crate::sip::SipConfig, String> {
    if s.sip_server.trim().is_empty() || s.sip_user.trim().is_empty() {
        return Err("No SIP account: fill in the SIP server and user name in Modem Settings".into());
    }
    Ok(crate::sip::SipConfig {
        server: s.sip_server.trim().to_string(),
        port: if s.sip_port == 0 { 5060 } else { s.sip_port },
        user: s.sip_user.trim().to_string(),
        password: s.sip_password.clone(),
        ..Default::default()
    })
}

impl App {
    /// What the modem dials for a phone.
    pub fn dial_target(&self, p: &Phone) -> String {
        if !p.dial_target.trim().is_empty() {
            return p.dial_target.trim().to_string();
        }
        let prefix = self
            .db
            .site(p.site_id)
            .map(|s| s.dial_prefix.clone())
            .unwrap_or_default();
        let digits: String = p.ani.chars().filter(|c| c.is_ascii_digit()).collect();
        format!("{}{}", prefix.trim(), digits)
    }

    pub fn call_setup(&self, p: &Phone) -> CallSetup {
        let s = &self.db.settings;
        let kx = p.model.starts_with("KX");
        let init = if kx { &s.init_kx } else { &s.init_series5 };
        CallSetup {
            connection: s.connection.or_serial(),
            sip: sip_account(s),
            serial_port: s.comm_port.clone(),
            baud: s.baud,
            tcp_addr: s.tcp_addr.clone(),
            init: init
                .split(';')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect(),
            dial_command: s.dial_prefix.clone(),
            target: self.dial_target(p),
            connect_timeout: s.timeout_secs,
            station_id: p.phone_id.clone(),
            password: p.password.clone(),
            ani: p.ani.clone(),
            codes: s.pnm_codes.clone(),
        }
    }

    /// Turn the command names into the operations of one call, with notes for what can't be sent.
    pub fn ops_for(&self, p: &Phone, commands: &[String], raw: Option<u8>) -> (Vec<Op>, Vec<String>) {
        let has = |name: &str| commands.iter().any(|c| c.starts_with(name));
        let mut ops = Vec::new();
        let mut notes = Vec::new();
        if has("Get Software Versions") || has("Get RAM Image") {
            ops.push(Op::Versions);
        }
        if has("Get Alarms") || has("Get Cashbox") || has("Get Call Counts") || has("Get RAM Image") {
            ops.push(Op::Status);
        }
        if has("Get Call Counts") {
            ops.push(Op::Counters);
        }
        if has("Retrieve Call Records") {
            ops.push(Op::Smdr);
        }
        if has("Set Date/Time") {
            if self.db.settings.pnm_codes.set_time.is_some() {
                ops.push(Op::SetTime);
            } else {
                ops.push(Op::GetTime);
                notes.push(
                    "Set Date/Time: the set-clock command isn't known yet, so the phone's clock is read instead".into(),
                );
            }
        }
        let mut sent_file = false;
        if has("Download Rates") {
            match p
                .files
                .get("Rates")
                .and_then(|n| self.db.config_files.iter().find(|f| f.kind == "Rates" && &f.name == n))
            {
                Some(f) if !f.binary_path.trim().is_empty() => match std::fs::read(f.binary_path.trim()) {
                    Ok(bytes) => {
                        ops.push(Op::SendRates {
                            name: f.name.clone(),
                            bytes,
                        });
                        sent_file = true;
                    }
                    Err(e) => notes.push(format!("Rates: could not read {}: {e}", f.binary_path)),
                },
                Some(f) => notes.push(format!(
                    "Rates: file {} has no .R94 file attached (Configuration > Rate Files)",
                    f.name
                )),
                None => notes.push("Rates: no rate file is assigned to this phone".into()),
            }
        }
        for c in commands {
            if c.starts_with("Download") && c != "Download Rates" {
                notes.push(format!(
                    "{c}: not sent - this file's format for the phone isn't known yet"
                ));
            }
        }
        if has("Set Totalizer") {
            notes.push("Set Totalizer: not sent - the phone command for it isn't known yet".into());
        }
        if has("Clear Alarms") {
            notes.push("Clear Alarms: not sent - the phone command for it isn't known yet".into());
        }
        if has("Get RAM Image") {
            ops.push(Op::RamImage);
        }
        if sent_file || has("Store Settings") {
            ops.push(Op::Store);
        }
        if let Some(r) = raw {
            ops.push(Op::Raw(r));
        }
        (ops, notes)
    }

    /// Put a call in line; it starts when the modem is free.
    pub fn queue_call(&mut self, ani: &str, commands: Vec<String>, raw: Option<u8>, source: &str) {
        self.dial.queue.push_back(QueuedCall {
            ani: ani.to_string(),
            commands,
            raw,
            source: source.to_string(),
        });
        self.start_next_call();
    }

    pub fn start_next_call(&mut self) {
        if self.dial.job.is_some() {
            return;
        }
        let Some(q) = self.dial.queue.pop_front() else { return };
        let Some(p) = self.db.phone(&q.ani).cloned() else {
            self.message(format!("{}: no such phone", q.ani));
            return self.start_next_call();
        };
        let setup = self.call_setup(&p);
        let (ops, notes) = self.ops_for(&p, &q.commands, q.raw);
        self.dial
            .log
            .push(format!("--- {} {} ({}) ---", now_stamp(), p.ani, q.source));
        self.dial.log.push(format!(
            "Dialing {}{} through {}",
            setup.dial_command,
            setup.target,
            setup.connection.label()
        ));
        for n in &notes {
            self.dial.log.push(format!("Note: {n}"));
        }
        if ops.is_empty() {
            self.dial.log.push("Nothing to send to the phone".into());
            self.dial.status = "Nothing to send".into();
            return self.start_next_call();
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        link::spawn_call(setup, ops, tx, cancel.clone());
        self.dial.status = format!("Calling {}", p.ani);
        self.dial.job = Some(CallJob {
            ani: p.ani.clone(),
            commands: q.commands,
            source: q.source,
            rx,
            cancel,
            started: std::time::Instant::now(),
        });
        let _ = notes;
    }

    pub fn cancel_calls(&mut self) {
        self.dial.queue.clear();
        if let Some(j) = &self.dial.job {
            j.cancel.store(true, Ordering::Relaxed);
            self.dial.status = "Cancelling...".into();
        }
    }

    /// Called every frame: read what the call thread reports.
    pub fn poll_call(&mut self, ctx: &eframe::egui::Context) {
        let mut done = None;
        if let Some(j) = &self.dial.job {
            while let Ok(ev) = j.rx.try_recv() {
                match ev {
                    LinkEvent::Log(l) => self.dial.log.push(l),
                    LinkEvent::Status(s) => self.dial.status = s,
                    LinkEvent::Done(r) => done = Some(*r),
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.dial.log.len() > 3000 {
            let cut = self.dial.log.len() - 3000;
            self.dial.log.drain(..cut);
        }
        if let Some(rep) = done {
            let job = self.dial.job.take().unwrap();
            let secs = job.started.elapsed().as_secs();
            self.apply_report(&job.ani, &job.commands, &rep, secs);
            self.message(format!("Call to {} ({}): {}", job.ani, job.source, rep.result));
            self.dial.status = format!("Ready ({})", rep.result);
            self.dial.last_report = Some(rep);
            self.save();
            self.start_next_call();
        }
    }

    /// Save what a real call brought back, as PNM does after a contact.
    pub fn apply_report(&mut self, ani: &str, commands: &[String], rep: &Report, _secs: u64) {
        let id = self.db.new_id();
        let mut c = Contact {
            id,
            ani: ani.to_string(),
            time: now_stamp(),
            result: if rep.logged_on {
                rep.result.clone()
            } else if rep.connected {
                format!("Connected, not logged on: {}", rep.result)
            } else {
                rep.result.clone()
            },
            commands: commands.to_vec(),
            ..Default::default()
        };
        if let Some(cn) = &rep.counters {
            c.counts = counts_from(link::counter_categories(cn));
        }
        let today = today();
        let alarm_names: Vec<String> = rep
            .status
            .as_ref()
            .map(|s| {
                s.alarms
                    .iter()
                    .map(|a| format!("Alarm #{a} reported by the phone"))
                    .collect()
            })
            .unwrap_or_default();
        c.alarms = alarm_names.clone();
        let mut new_records = Vec::new();
        if let Some(p) = self.db.phone_mut(ani) {
            if rep.logged_on {
                p.last_contact = c.time.clone();
            }
            if let Some(v) = &rep.versions {
                if !v.program.is_empty() {
                    p.pgm_ver = v.program.clone();
                }
                if !v.chip.is_empty() {
                    p.rom_ver = v.chip.clone();
                }
                if !v.serial.is_empty() {
                    p.serial = v.serial.clone();
                }
            }
            if let Some(s) = &rep.status {
                p.cashbox = s.cashbox;
                p.last_collected = s.last_collected;
                p.totalizer = s.totalizer;
            }
            if let Some(b) = &rep.ram_image {
                p.ram_block = link::hex(b);
                p.ram_block_time = c.time.clone();
            }
            if let Some(name) = &rep.rates_sent {
                p.file_dates.insert("Rates".into(), today.clone());
                p.files.insert("Rates".into(), name.clone());
            }
            if rep.rates_sent.is_some() && rep.stored == Some(true) {
                p.needs_update = false;
            }
            c.totalizer = p.totalizer;
            c.cashbox = p.cashbox;
            for r in &rep.smdr {
                new_records.push(CallRecord {
                    ani: ani.to_string(),
                    date: r.date.clone(),
                    call_type: format!("Type {}", r.call_type),
                    dialed: r.dialed.clone(),
                    duration_secs: r.duration_secs,
                    coin: r.price as f64 / 100.0,
                });
            }
        }
        self.db.call_records.extend(new_records);
        for a in alarm_names {
            let open = self
                .db
                .alarms
                .iter()
                .any(|x| x.ani == ani && !x.handled && x.description == a);
            if !open {
                let aid = self.db.new_id();
                self.db.alarms.push(Alarm {
                    id: aid,
                    ani: ani.to_string(),
                    time: c.time.clone(),
                    description: a,
                    handled: false,
                    handled_note: String::new(),
                });
            }
        }
        self.db.contacts.push(c.clone());
        self.dial.result = Some(c);
    }

    /// Ask the modem who it is, without dialing (Modem settings > Test).
    pub fn test_modem(&mut self) {
        let s = &self.cfg.settings;
        let setup = CallSetup {
            connection: s.connection.or_serial(),
            sip: sip_account(s),
            serial_port: s.comm_port.clone(),
            baud: s.baud,
            tcp_addr: s.tcp_addr.clone(),
            init: vec![],
            dial_command: String::new(),
            target: String::new(),
            connect_timeout: 5,
            station_id: String::new(),
            password: String::new(),
            ani: String::new(),
            codes: s.pnm_codes.clone(),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        self.cfg.modem_test.lines = vec![format!("Testing {}...", setup.connection.label())];
        if setup.connection == Connection::Sip {
            self.cfg.modem_test.lines.push(match &setup.sip {
                Ok(c) => format!("SIP account {}@{}:{} (characters 8N1)", c.user, c.server, c.port),
                Err(e) => e.clone(),
            });
        }
        self.cfg.modem_test.running = Some(rx);
        std::thread::spawn(move || {
            let mut w = match link::open_wire(&setup) {
                Ok(w) => w,
                Err(e) => {
                    let _ = tx.send(e);
                    let _ = tx.send("done".into());
                    return;
                }
            };
            for cmd in ["AT", "ATI3", "AT+SIPREG?", "AT+SIPMODE?"] {
                let _ = tx.send(format!("> {cmd}"));
                if w.send(format!("{cmd}\r").as_bytes()).is_err() {
                    let _ = tx.send("could not write to the modem".into());
                    break;
                }
                let mut got = Vec::new();
                let end = std::time::Instant::now() + std::time::Duration::from_millis(1500);
                while std::time::Instant::now() < end {
                    match w.recv(std::time::Duration::from_millis(150)) {
                        Ok(b) => got.extend(b),
                        Err(e) => {
                            let _ = tx.send(e.to_string());
                            break;
                        }
                    }
                    let t = String::from_utf8_lossy(&got);
                    if t.contains("OK") || t.contains("ERROR") {
                        break;
                    }
                }
                let t = String::from_utf8_lossy(&got).replace('\r', "").trim().to_string();
                let _ = tx.send(if t.is_empty() { "(no answer)".into() } else { t });
            }
            let _ = tx.send("done".into());
        });
    }
}
