//! Configuration: system settings, modem settings, phone macros, global phone changes, journal purge.

use crate::app::{at, App};
use crate::details::dual_list;
use crate::link::Connection;
use crate::model::*;
use crate::style;
use eframe::egui::{self, Vec2};

impl App {
    pub fn config_windows(&mut self, ctx: &egui::Context) {
        self.sysconfig_window(ctx);
        self.modem_window(ctx);
        self.macros_window(ctx);
        self.global_window(ctx);
        self.purge_window(ctx);
        self.files_window(ctx);
    }

    pub fn open_files(&mut self, kind: &str) {
        self.cfg.files_kind = kind.to_string();
        self.cfg.files_sel = None;
        self.w.files = true;
    }

    /// Rate files, speed dial, priority parsing, messages, rep/cards, registers & options and voice
    /// files: create, edit, rename, delete and assign them to the active phone.
    fn files_window(&mut self, ctx: &egui::Context) {
        if self.w.files {
            if self.cfg.files_kind.is_empty() {
                self.cfg.files_kind = FILE_KINDS[0].0.to_string();
            }
            let pos = at(self.area, 0.22, 0.06);
            let kind = self.cfg.files_kind.clone();
            let rows: Vec<(String, String)> = self
                .db
                .config_files
                .iter()
                .filter(|f| f.kind == kind)
                .map(|f| {
                    let users = self
                        .db
                        .phones
                        .iter()
                        .filter(|p| p.files.get(&kind) == Some(&f.name))
                        .count();
                    (
                        f.name.clone(),
                        format!("{:<12} {:<30} {:>3} phones", f.name, f.description, users),
                    )
                })
                .collect();
            let phone = self.active_ani.clone();
            let assigned = phone
                .as_ref()
                .and_then(|a| self.db.phone(a))
                .and_then(|p| p.files.get(&kind).cloned());
            let mut sel = self.cfg.files_sel.clone();
            let mut action = "";
            let c = &mut self.cfg;
            let close = style::child_window(ctx, "cfgfiles", "Configuration Files", self.area, pos, 470.0, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (k, ..) in FILE_KINDS {
                        let on = c.files_kind == *k;
                        let text = egui::RichText::new(*k).color(if on { style::WHITE } else { style::BLACK });
                        if ui.add(egui::SelectableLabel::new(on, text)).clicked() {
                            c.files_kind = k.to_string();
                            sel = None;
                        }
                    }
                });
                ui.separator();
                ui.label(format!(
                    "Phone To Assign File: {}   (currently {})",
                    phone.clone().unwrap_or_else(|| "no active phone".into()),
                    assigned.clone().unwrap_or_else(|| "none".into())
                ));
                style::header(ui, &format!("{:<12} {:<30} {}", "Name", "Description", "In use"));
                if style::list_box(ui, "cfgfilelist", &rows, &mut sel, Vec2::new(458.0, 150.0)).is_some() {
                    action = "edit";
                }
                ui.horizontal(|ui| {
                    if style::btn(ui, "New") {
                        action = "new";
                    }
                    if style::btn_enabled(ui, sel.is_some() && phone.is_some(), "Assign") {
                        action = "assign";
                    }
                    if style::btn_enabled(ui, sel.is_some(), "Delete") {
                        action = "delete";
                    }
                    if style::btn_enabled(ui, sel.is_some(), "Edit") {
                        action = "edit";
                    }
                    if style::btn(ui, "Close") {
                        action = "close";
                    }
                });
                ui.small("Assigning a file marks the phone as needing a download; send it with Dial Payphone or a poll list.");
            });
            self.cfg.files_sel = sel.clone();
            let kind = self.cfg.files_kind.clone();
            match action {
                "new" => {
                    self.cfg.file_edit = Some((
                        ConfigFile {
                            kind: kind.clone(),
                            name: "NEWFILE".into(),
                            ..Default::default()
                        },
                        String::new(),
                    ));
                }
                "edit" => {
                    if let Some(f) = sel
                        .as_ref()
                        .and_then(|n| self.db.config_files.iter().find(|f| f.kind == kind && &f.name == n))
                    {
                        self.cfg.file_edit = Some((f.clone(), f.name.clone()));
                    }
                }
                "delete" => {
                    let n = sel.unwrap();
                    self.db.config_files.retain(|f| !(f.kind == kind && f.name == n));
                    for p in self.db.phones.iter_mut() {
                        if p.files.get(&kind) == Some(&n) {
                            p.files.remove(&kind);
                        }
                    }
                    self.cfg.files_sel = None;
                    self.save();
                }
                "assign" => {
                    let (a, n) = (phone.unwrap(), sel.unwrap());
                    if let Some(p) = self.db.phone_mut(&a) {
                        p.files.insert(kind.clone(), n.clone());
                        p.needs_update = true;
                    }
                    self.message(format!("{kind} file {n} assigned to {a}; the phone needs a download"));
                    self.save();
                }
                "close" => self.w.files = false,
                _ => {}
            }
            if close {
                self.w.files = false;
            }
        }
        let pos2 = at(self.area, 0.3, 0.12);
        let Some((f, _)) = self.cfg.file_edit.as_mut() else {
            return;
        };
        let (h1, h2) = FILE_KINDS
            .iter()
            .find(|k| k.0 == f.kind)
            .map(|k| (k.1, k.2))
            .unwrap_or(("Item", "Value"));
        let mut ok = false;
        let mut cancel = false;
        let title = format!("{} File Editor", f.kind);
        let close = style::child_window(ctx, "cfgfileedit", &title, self.area, pos2, 480.0, |ui| {
            egui::Grid::new("cfgfe").num_columns(2).show(ui, |ui| {
                ui.label("File name:");
                ui.add(
                    egui::TextEdit::singleline(&mut f.name)
                        .desired_width(120.0)
                        .char_limit(8),
                );
                ui.end_row();
                style::field(ui, "Description:", &mut f.description, 340.0);
            });
            ui.small("File names are up to 8 characters, as on the phone.");
            if f.kind == "Rates" {
                ui.horizontal(|ui| {
                    ui.label("Rate file to send (.R94):");
                    ui.add(
                        egui::TextEdit::singleline(&mut f.binary_path)
                            .desired_width(300.0)
                            .hint_text("C:\\...\\517299.R94"),
                    );
                });
                let p = f.binary_path.trim();
                if !p.is_empty() {
                    match std::fs::metadata(p) {
                        Ok(m) => {
                            let blocks = std::fs::read(p)
                                .map(|b| crate::link::rate_blocks(&b).len())
                                .unwrap_or(0);
                            ui.small(format!(
                                "{} bytes: sent to the phone as a header and {} block(s), then stored in its EEPROM.",
                                m.len(),
                                blocks
                            ));
                        }
                        Err(_) => {
                            ui.colored_label(egui::Color32::from_rgb(170, 0, 0), "File not found");
                        }
                    }
                } else {
                    ui.small("Without a .R94 file, \"Download Rates\" can't send this file to a real phone.");
                }
            }
            egui::Frame::default()
                .fill(style::WHITE)
                .inner_margin(4.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("cfgrows")
                        .max_height(220.0)
                        .show(ui, |ui| {
                            egui::Grid::new("cfgrowsgrid")
                                .num_columns(3)
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.strong(h1);
                                    ui.strong(h2);
                                    ui.label("");
                                    ui.end_row();
                                    let mut remove = None;
                                    for (i, (a, b)) in f.rows.iter_mut().enumerate() {
                                        ui.add(egui::TextEdit::singleline(a).desired_width(150.0));
                                        ui.add(egui::TextEdit::singleline(b).desired_width(220.0));
                                        if ui.small_button("Delete").clicked() {
                                            remove = Some(i);
                                        }
                                        ui.end_row();
                                    }
                                    if let Some(i) = remove {
                                        f.rows.remove(i);
                                    }
                                });
                        });
                });
            ui.horizontal(|ui| {
                if style::btn(ui, "Add Row") {
                    f.rows.push((String::new(), String::new()));
                }
                ok = style::btn(ui, "OK");
                cancel = style::btn(ui, "Cancel");
            });
        });
        if ok {
            let (mut f, orig) = self.cfg.file_edit.take().unwrap();
            f.name = f.name.trim().to_uppercase();
            if f.name.is_empty() {
                f.name = "NEWFILE".into();
            }
            if let Some(x) = self
                .db
                .config_files
                .iter_mut()
                .find(|x| x.kind == f.kind && x.name == orig && !orig.is_empty())
            {
                *x = f.clone();
            } else if !self
                .db
                .config_files
                .iter()
                .any(|x| x.kind == f.kind && x.name == f.name)
            {
                self.db.config_files.push(f.clone());
            }
            if !orig.is_empty() && orig != f.name {
                for p in self.db.phones.iter_mut() {
                    if p.files.get(&f.kind) == Some(&orig) {
                        p.files.insert(f.kind.clone(), f.name.clone());
                    }
                }
            }
            // phones using a changed file must receive it again
            for p in self
                .db
                .phones
                .iter_mut()
                .filter(|p| p.files.get(&f.kind) == Some(&f.name))
            {
                p.needs_update = true;
            }
            self.cfg.files_sel = Some(f.name.clone());
            self.message(format!("{} file {} saved", f.kind, f.name));
            self.save();
        } else if cancel || close {
            self.cfg.file_edit = None;
        }
    }

    fn sysconfig_window(&mut self, ctx: &egui::Context) {
        if !self.w.sysconfig {
            return;
        }
        let pos = at(self.area, 0.2, 0.08);
        let mut ok = false;
        let mut cancel = false;
        let mut factory = false;
        let s = &mut self.cfg.settings;
        let close = style::child_window(ctx, "syscfg", "System Configuration", self.area, pos, 440.0, |ui| {
            style::group(ui, "Registration", |ui| {
                egui::Grid::new("sysg").num_columns(2).show(ui, |ui| {
                    style::field(ui, "Company", &mut s.company, 260.0);
                    style::field(ui, "User Name", &mut s.user_name, 200.0);
                    style::field(ui, "User ID", &mut s.user_id, 80.0);
                });
            });
            style::group(ui, "Defaults for new phones", |ui| {
                egui::Grid::new("sysg2").num_columns(2).show(ui, |ui| {
                    style::field(ui, "Password", &mut s.default_password, 80.0);
                    style::field(ui, "Bypass", &mut s.default_bypass, 80.0);
                });
                ui.checkbox(
                    &mut s.ten_digit_id,
                    "Use 10 digit phone IDs (needs 5.3.3 or higher firmware)",
                );
            });
            ui.horizontal(|ui| {
                ok = style::btn(ui, "OK");
                cancel = style::btn(ui, "Cancel");
                factory = style::btn(ui, "Factory");
            });
        });
        if factory {
            let keep = (
                self.cfg.settings.company.clone(),
                self.cfg.settings.user_name.clone(),
                self.cfg.settings.user_id.clone(),
            );
            self.cfg.settings = Settings::default();
            self.cfg.settings.company = keep.0;
            self.cfg.settings.user_name = keep.1;
            self.cfg.settings.user_id = keep.2;
        }
        if ok {
            let mut s = self.cfg.settings.clone();
            // keep modem values that this window doesn't show
            let cur = &self.db.settings;
            s.comm_port = cur.comm_port.clone();
            s.modem_type = cur.modem_type.clone();
            s.init_series5 = cur.init_series5.clone();
            s.init_kx = cur.init_kx.clone();
            s.dial_prefix = cur.dial_prefix.clone();
            s.timeout_secs = cur.timeout_secs;
            self.db.settings = s;
            self.message("System configuration saved");
            self.save();
        }
        if ok || cancel || close {
            self.w.sysconfig = false;
        }
    }

    fn modem_window(&mut self, ctx: &egui::Context) {
        if !self.w.modem {
            return;
        }
        let pos = at(self.area, 0.18, 0.02);
        let mut ok = false;
        let mut cancel = false;
        let mut test = false;
        let mut refresh = false;
        let mut factory_codes = false;
        if self.cfg.ports.is_empty() {
            refresh = true;
        }
        // "Simulated (no modem)" is no longer offered: a page opened on it shows the serial-port modem.
        if self.cfg.settings.connection == Connection::Simulated {
            self.cfg.settings.connection = Connection::Serial;
        }
        // command codes as editable hex text
        const CODE_NAMES: [&str; 13] = [
            "Password",
            "Download file",
            "Store in EEPROM",
            "Reload",
            "Hang up",
            "Software versions",
            "Status (cash, alarms)",
            "Call counters",
            "SMDR header",
            "SMDR block",
            "Read clock",
            "Set clock",
            "Data block (RAM image)",
        ];
        if self.cfg.code_text.len() != CODE_NAMES.len() {
            self.cfg.sip_port_text = self.cfg.settings.sip_port.to_string();
            let c = &self.cfg.settings.pnm_codes;
            let h = |v: u8| format!("{v:02X}");
            let o = |v: Option<u8>| v.map(|v| format!("{v:02X}")).unwrap_or_default();
            self.cfg.code_text = vec![
                h(c.password),
                h(c.download),
                h(c.store),
                h(c.reload),
                h(c.hangup),
                o(c.versions),
                o(c.status),
                o(c.counters),
                o(c.smdr_header),
                o(c.smdr_block),
                o(c.get_time),
                o(c.set_time),
                o(c.ram_image),
            ];
        }
        if let Some(rx) = &self.cfg.modem_test.running {
            while let Ok(l) = rx.try_recv() {
                if l == "done" {
                    self.cfg.modem_test.running = None;
                    break;
                }
                self.cfg.modem_test.lines.push(l);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let testing = self.cfg.modem_test.running.is_some();
        let test_lines = self.cfg.modem_test.lines.clone();
        let ports = self.cfg.ports.clone();
        let codes = &mut self.cfg.code_text;
        let port_text = &mut self.cfg.sip_port_text;
        let s = &mut self.cfg.settings;
        let types: Vec<String> = [
            "Hayes compatible",
            "Virtual SIP Modem",
            "US Robotics Sportster",
            "Other",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let close = style::child_window(
            ctx,
            "modem",
            "Modem/Communications Settings",
            self.area,
            pos,
            620.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Connection:");
                    for c in [Connection::Serial, Connection::Tcp, Connection::Sip] {
                        ui.radio_value(&mut s.connection, c, c.label());
                    }
                });
                if s.connection == Connection::Sip {
                    style::group(ui, "SIP account (built-in SIP modem)", |ui| {
                        egui::Grid::new("sipacct")
                            .num_columns(4)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("SIP Server:");
                                ui.add_sized(
                                    [200.0, 20.0],
                                    egui::TextEdit::singleline(&mut s.sip_server)
                                        .hint_text("server name or IP address"),
                                );
                                ui.label("Port:");
                                ui.add_sized(
                                    [70.0, 20.0],
                                    egui::TextEdit::singleline(port_text).char_limit(5).hint_text("5060"),
                                );
                                ui.end_row();
                                ui.label("User Name:");
                                ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(&mut s.sip_user));
                                ui.label("Password:");
                                ui.add_sized(
                                    [150.0, 20.0],
                                    egui::TextEdit::singleline(&mut s.sip_password).password(true),
                                );
                                ui.end_row();
                            });
                        let port_ok = port_text.trim().is_empty()
                            || port_text.trim().parse::<u16>().map(|p| p > 0).unwrap_or(false);
                        if !port_ok {
                            ui.colored_label(
                                egui::Color32::from_rgb(170, 0, 0),
                                "The port must be a number from 1 to 65535.",
                            );
                        }
                        ui.small("Calls go out over SIP with modem tones (1200 or 300 bit/s, characters 8N1). Comm port, speed and network address aren't used. A phone's Dial String can be a number or a sip: address.");
                    });
                }
                ui.horizontal_top(|ui| {
                    egui::Grid::new("modemg")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Comm Port:");
                            ui.horizontal(|ui| {
                                style::combo(ui, "mport", &mut s.comm_port, &ports, 90.0);
                                if ui.small_button("Refresh").clicked() {
                                    refresh = true;
                                }
                            });
                            ui.end_row();
                            ui.label("Speed (bit/s):");
                            egui::ComboBox::from_id_salt("mbaud")
                                .width(90.0)
                                .selected_text(s.baud.to_string())
                                .show_ui(ui, |ui| {
                                    for b in [1200u32, 2400, 9600, 19200, 38400, 57600, 115200] {
                                        ui.selectable_value(&mut s.baud, b, b.to_string());
                                    }
                                });
                            ui.end_row();
                            style::field(ui, "Network address:", &mut s.tcp_addr, 150.0);
                            ui.label("Type:");
                            style::combo(ui, "mtype", &mut s.modem_type, &types, 160.0);
                            ui.end_row();
                            style::field(ui, "Init String (Series 5):", &mut s.init_series5, 160.0);
                            style::field(ui, "Init String (KX-5502):", &mut s.init_kx, 160.0);
                            style::field(ui, "Dial Prefix:", &mut s.dial_prefix, 70.0);
                            ui.label("Time to Wait before Timeout");
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut s.timeout_secs).range(5..=300));
                                ui.label("secs");
                            });
                            ui.end_row();
                        });
                    style::group(ui, "Phone command codes (hex)", |ui| {
                        egui::Grid::new("mcodes")
                            .num_columns(2)
                            .spacing([8.0, 2.0])
                            .show(ui, |ui| {
                                for (i, n) in CODE_NAMES.iter().enumerate() {
                                    ui.label(*n);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut codes[i])
                                            .desired_width(32.0)
                                            .char_limit(2),
                                    );
                                    ui.end_row();
                                }
                            });
                        if ui.small_button("Series 5 defaults").clicked() {
                            factory_codes = true;
                        }
                        ui.small("Blank = not known; that request is skipped.");
                    });
                });
                ui.small("Init strings: separate several commands with ;  (for example ATZ;AT+SIPMODE=TONES)");
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !testing,
                            egui::Button::new("Test Modem").min_size(Vec2::new(78.0, 26.0)),
                        )
                        .clicked()
                    {
                        test = true;
                    }
                    ok = style::btn(ui, "OK");
                    cancel = style::btn(ui, "Cancel");
                });
                if !test_lines.is_empty() {
                    egui::Frame::default()
                        .fill(style::BLACK)
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            ui.set_width(600.0);
                            for l in &test_lines {
                                ui.label(
                                    egui::RichText::new(l)
                                        .monospace()
                                        .color(egui::Color32::from_rgb(0, 220, 0)),
                                );
                            }
                        });
                }
            },
        );
        if refresh {
            self.cfg.ports = crate::link::serial_ports();
            if self.cfg.ports.is_empty() {
                self.cfg.ports = (1..=16).map(|i| format!("COM{i}")).collect();
            }
        }
        // the typed port, for Test Modem and OK
        self.cfg.settings.sip_port = self
            .cfg
            .sip_port_text
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|p| *p > 0)
            .unwrap_or(5060);
        if factory_codes {
            self.cfg.settings.pnm_codes = crate::link::PnmCodes::default();
            self.cfg.code_text.clear();
        }
        if test {
            self.test_modem();
        }
        if ok {
            // read back the command codes
            let t = self.cfg.code_text.clone();
            let p = |i: usize| u8::from_str_radix(t.get(i).map(|s| s.trim()).unwrap_or(""), 16).ok();
            let c = &mut self.cfg.settings.pnm_codes;
            c.password = p(0).unwrap_or(0x90);
            c.download = p(1).unwrap_or(0xA3);
            c.store = p(2).unwrap_or(0x29);
            c.reload = p(3).unwrap_or(0x08);
            c.hangup = p(4).unwrap_or(0x01);
            c.versions = p(5);
            c.status = p(6);
            c.counters = p(7);
            c.smdr_header = p(8);
            c.smdr_block = p(9);
            c.get_time = p(10);
            c.set_time = p(11);
            c.ram_image = p(12);
            let s = self.cfg.settings.clone();
            let d = &mut self.db.settings;
            d.connection = s.connection;
            d.comm_port = s.comm_port;
            d.baud = s.baud;
            d.tcp_addr = s.tcp_addr;
            d.modem_type = s.modem_type;
            d.init_series5 = s.init_series5;
            d.init_kx = s.init_kx;
            d.dial_prefix = s.dial_prefix;
            d.timeout_secs = s.timeout_secs;
            d.pnm_codes = s.pnm_codes;
            d.sip_server = s.sip_server;
            d.sip_port = s.sip_port;
            d.sip_user = s.sip_user;
            d.sip_password = s.sip_password;
            let label = d.connection.label();
            self.message(format!("Modem settings saved ({label})"));
            self.save();
        }
        if ok || cancel || close {
            self.w.modem = false;
            self.cfg.code_text.clear();
            self.cfg.modem_test.lines.clear();
        }
    }

    fn macros_window(&mut self, ctx: &egui::Context) {
        if self.w.macros {
            let pos = at(self.area, 0.25, 0.1);
            let rows: Vec<(String, String)> = self
                .db
                .macros
                .iter()
                .map(|m| (m.name.clone(), format!("{:<18} {}", m.name, m.description)))
                .collect();
            let mut sel = self.cfg.macro_sel.clone();
            let mut action = "";
            let close = style::child_window(ctx, "macros", "Edit Phone Macros", self.area, pos, 400.0, |ui| {
                ui.label("Select Macro:");
                if style::list_box(ui, "macrolist", &rows, &mut sel, Vec2::new(388.0, 140.0)).is_some() {
                    action = "edit";
                }
                ui.horizontal(|ui| {
                    if style::btn(ui, "New") {
                        action = "new";
                    }
                    if style::btn_enabled(ui, sel.is_some(), "Edit") {
                        action = "edit";
                    }
                    if style::btn_enabled(ui, sel.is_some(), "Delete") {
                        action = "delete";
                    }
                    if style::btn(ui, "Close") {
                        action = "close";
                    }
                });
            });
            self.cfg.macro_sel = sel.clone();
            match action {
                "new" => {
                    self.cfg.macro_edit = Some((
                        Macro {
                            name: "New Macro".into(),
                            ..Default::default()
                        },
                        String::new(),
                    ))
                }
                "edit" => {
                    if let Some(m) = sel.as_ref().and_then(|n| self.db.macros.iter().find(|m| &m.name == n)) {
                        self.cfg.macro_edit = Some((m.clone(), m.name.clone()));
                    }
                }
                "delete" => {
                    let n = sel.unwrap();
                    self.db.macros.retain(|m| m.name != n);
                    self.cfg.macro_sel = None;
                    self.save();
                }
                "close" => self.w.macros = false,
                _ => {}
            }
            if close {
                self.w.macros = false;
            }
        }
        let pos2 = at(self.area, 0.18, 0.18);
        let Some((m, _)) = self.cfg.macro_edit.as_mut() else {
            return;
        };
        let right: Vec<(String, String)> = m
            .commands
            .iter()
            .enumerate()
            .map(|(i, c)| (format!("{i}|{c}"), c.clone()))
            .collect();
        let left: Vec<(String, String)> = COMMANDS.iter().map(|c| (c.to_string(), c.to_string())).collect();
        let pick = &mut self.cfg.macro_pick;
        let mut ok = false;
        let mut cancel = false;
        let close = style::child_window(ctx, "macroedit", "Create/Update Macro", self.area, pos2, 560.0, |ui| {
            egui::Grid::new("macg").num_columns(2).show(ui, |ui| {
                style::field(ui, "Name:", &mut m.name, 200.0);
                style::field(ui, "Description:", &mut m.description, 300.0);
            });
            let (a, r, aa, ra) = dual_list(
                ui,
                "macpick",
                "Low level command set",
                &left,
                "Commands in macro",
                &right,
                pick,
                220.0,
                170.0,
            );
            if let Some(a) = a {
                // "Insert Before" the selected command when one is selected
                let at_idx = pick
                    .right_sel
                    .as_ref()
                    .and_then(|k| k.split('|').next())
                    .and_then(|i| i.parse::<usize>().ok());
                match at_idx {
                    Some(i) if i <= m.commands.len() => m.commands.insert(i, a),
                    _ => m.commands.push(a),
                }
            }
            if let Some(r) = r {
                if let Some(i) = r.split('|').next().and_then(|i| i.parse::<usize>().ok()) {
                    if i < m.commands.len() {
                        m.commands.remove(i);
                    }
                }
            }
            if aa {
                m.commands = COMMANDS.iter().map(|c| c.to_string()).collect();
            }
            if ra {
                m.commands.clear();
            }
            ui.small("Select a command in the macro first to insert new ones before it.");
            ui.horizontal(|ui| {
                ok = style::btn(ui, "OK");
                cancel = style::btn(ui, "Cancel");
            });
        });
        if ok {
            let (m, orig) = self.cfg.macro_edit.take().unwrap();
            if let Some(x) = self.db.macros.iter_mut().find(|x| x.name == orig && !orig.is_empty()) {
                *x = m;
            } else {
                self.db.macros.push(m);
            }
            self.save();
        } else if cancel || close {
            self.cfg.macro_edit = None;
        }
    }

    fn global_window(&mut self, ctx: &egui::Context) {
        if !self.w.global {
            return;
        }
        let pos = at(self.area, 0.06, 0.04);
        let model = self.cfg.global_model.clone();
        let chosen = self.cfg.global_chosen.clone();
        let left: Vec<(String, String)> = self
            .db
            .phones
            .iter()
            .filter(|p| model.is_empty() || p.model == model)
            .filter(|p| !chosen.contains(&p.ani))
            .map(|p| (p.ani.clone(), format!("{:<13} {:<10} {}", p.ani, p.model, p.location)))
            .collect();
        let right: Vec<(String, String)> = chosen.iter().map(|a| (a.clone(), self.db.phone_label(a))).collect();
        let taxes: Vec<String> = self.db.tax_templates.iter().map(|t| t.name.clone()).collect();
        let rates: Vec<String> = self.db.rate_templates.iter().map(|t| t.name.clone()).collect();
        let macros: Vec<String> = self.db.macros.iter().map(|m| m.name.clone()).collect();
        let polls: Vec<(u32, String)> = self.db.poll_lists.iter().map(|l| (l.id, l.name.clone())).collect();
        let models: Vec<String> = MODELS.iter().map(|s| s.to_string()).collect();
        let mut apply = false;
        let mut done = false;
        let mut clear = false;
        let c = &mut self.cfg;
        let close = style::child_window(ctx, "global", "Global Phone Changes", self.area, pos, 640.0, |ui| {
            ui.horizontal(|ui| {
                ui.label("Model Type:");
                style::combo(ui, "gmodel", &mut c.global_model, &models, 120.0);
                ui.label("(none = every model)");
            });
            let (a, r, aa, ra) = dual_list(
                ui,
                "gpick",
                "Phones",
                &left,
                "Selected Phones",
                &right,
                &mut c.global_pick,
                250.0,
                160.0,
            );
            if let Some(a) = a {
                c.global_chosen.push(a);
            }
            if let Some(r) = r {
                c.global_chosen.retain(|x| *x != r);
            }
            if aa {
                c.global_chosen.extend(left.iter().map(|(k, _)| k.clone()));
            }
            if ra {
                c.global_chosen.clear();
            }
            style::group(ui, "Changes to apply to the selected phones", |ui| {
                egui::Grid::new("gch").num_columns(2).show(ui, |ui| {
                    ui.label("Tax rate template");
                    style::combo(ui, "gtax", &mut c.global_tax, &taxes, 180.0);
                    ui.end_row();
                    ui.label("Charged rate template");
                    style::combo(ui, "grate", &mut c.global_rate, &rates, 180.0);
                    ui.end_row();
                    ui.label("Add to poll list");
                    style::combo_id(ui, "gpoll", &mut c.global_poll, &polls, 180.0);
                    ui.end_row();
                    ui.label("Send macro (one-time poll list)");
                    style::combo(ui, "gmacro", &mut c.global_macro, &macros, 180.0);
                    ui.end_row();
                });
            });
            ui.horizontal(|ui| {
                if style::btn_enabled(ui, !c.global_chosen.is_empty(), "Apply Changes") {
                    apply = true;
                }
                if style::btn(ui, "Clear") {
                    clear = true;
                }
                if style::btn(ui, "Close") {
                    done = true;
                }
            });
        });
        if clear {
            let m = self.cfg.global_model.clone();
            self.cfg.global_chosen.clear();
            self.cfg.global_tax.clear();
            self.cfg.global_rate.clear();
            self.cfg.global_poll = None;
            self.cfg.global_macro.clear();
            self.cfg.global_model = m;
        }
        if apply {
            let chosen = self.cfg.global_chosen.clone();
            let tax = self.cfg.global_tax.clone();
            let rate = self.cfg.global_rate.clone();
            for p in self.db.phones.iter_mut().filter(|p| chosen.contains(&p.ani)) {
                if !tax.is_empty() {
                    p.tax_template = tax.clone();
                    p.needs_update = true;
                }
                if !rate.is_empty() {
                    p.rate_template = rate.clone();
                    p.needs_update = true;
                }
            }
            if let Some(pid) = self.cfg.global_poll {
                if let Some(l) = self.db.poll_lists.iter_mut().find(|l| l.id == pid) {
                    for a in &chosen {
                        if !l.phones.contains(a) {
                            l.phones.push(a.clone());
                        }
                    }
                }
            }
            let mac = self.cfg.global_macro.clone();
            if let Some(m) = self.db.macros.iter().find(|m| m.name == mac).cloned() {
                let id = self.db.new_id();
                self.db.poll_lists.push(PollList {
                    id,
                    name: format!("Global change - {}", m.name),
                    kind: "One-time".into(),
                    commands: m.commands,
                    phones: chosen.clone(),
                    schedule: vec!["0200".into()],
                    last_run: String::new(),
                });
            }
            self.message(format!("Global changes applied to {} phone(s)", chosen.len()));
            self.save();
        }
        if done || close {
            self.w.global = false;
        }
    }

    fn purge_window(&mut self, ctx: &egui::Context) {
        if !self.w.purge {
            return;
        }
        let pos = at(self.area, 0.28, 0.2);
        let codes: Vec<(String, String)> = self
            .db
            .account_codes
            .iter()
            .map(|a| (a.code.clone(), format!("{} {}", a.code, a.description)))
            .collect();
        let from = self.cfg.purge_from.clone();
        let to = self.cfg.purge_to.clone();
        let sel_codes = self.cfg.purge_codes.clone();
        let matching = self
            .db
            .journal
            .iter()
            .filter(|j| !from.is_empty() && j.date.as_str() >= from.as_str())
            .filter(|j| !to.is_empty() && j.date.as_str() <= to.as_str())
            .filter(|j| sel_codes.is_empty() || sel_codes.contains(&j.account_code))
            .count();
        let mut ok = false;
        let mut cancel = false;
        let c = &mut self.cfg;
        let close = style::child_window(
            ctx,
            "purge",
            "Purge Selected Journal Entries",
            self.area,
            pos,
            400.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Dates");
                    ui.add(
                        egui::TextEdit::singleline(&mut c.purge_from)
                            .desired_width(90.0)
                            .hint_text("YYYY-MM-DD"),
                    );
                    ui.label("to");
                    ui.add(
                        egui::TextEdit::singleline(&mut c.purge_to)
                            .desired_width(90.0)
                            .hint_text("YYYY-MM-DD"),
                    );
                });
                ui.label("Account codes (none ticked = all):");
                ui.horizontal_wrapped(|ui| {
                    for (code, label) in &codes {
                        let mut on = c.purge_codes.contains(code);
                        if ui.checkbox(&mut on, label).changed() {
                            if on {
                                c.purge_codes.push(code.clone());
                            } else {
                                c.purge_codes.retain(|x| x != code);
                            }
                        }
                    }
                });
                ui.colored_label(
                egui::Color32::from_rgb(160, 0, 0),
                "Warning! Journal entries that you purge cannot be restored. Make sure you have selected the proper date range and accounting codes before you press OK.",
            );
                ui.label(format!("{matching} entries will be purged."));
                ui.horizontal(|ui| {
                    ok = style::btn_enabled(ui, matching > 0, "OK");
                    cancel = style::btn(ui, "Cancel");
                });
            },
        );
        if ok {
            let before = self.db.journal.len();
            self.db.journal.retain(|j| {
                !(j.date.as_str() >= from.as_str()
                    && j.date.as_str() <= to.as_str()
                    && (sel_codes.is_empty() || sel_codes.contains(&j.account_code)))
            });
            let n = before - self.db.journal.len();
            self.message(format!("{n} journal entries purged"));
            self.save();
        }
        if ok || cancel || close {
            self.w.purge = false;
        }
    }
}
