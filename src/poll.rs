//! Poll lists, their scheduler, and the Dial Payphone window.

use crate::app::{at, App};
use crate::details::dual_list;
use crate::model::*;
use crate::style;
use eframe::egui::{self, Color32, Vec2};

impl App {
    pub fn open_dial(&mut self, ani: Option<String>) {
        if self.dial.job.is_none() {
            self.dial.ani = ani.or_else(|| self.active_ani.clone());
            self.dial.result = None;
            if self.dial.selected.is_empty() {
                self.dial.selected = vec![
                    "Get Call Counts".into(),
                    "Get Alarms".into(),
                    "Get Cashbox Totals".into(),
                ];
            }
        }
        self.w.dial = true;
    }

    /// Runs every frame: follows the call in progress and starts scheduled poll lists.
    pub fn tick(&mut self, ctx: &egui::Context) {
        self.poll_call(ctx);
        if self.shot_path.is_none() {
            let minute = chrono::Local::now().format("%Y-%m-%d %H%M").to_string();
            if minute != self.last_schedule_check {
                self.last_schedule_check = minute;
                self.run_due_polls(false);
            }
        }
    }

    /// Calls the phones of every poll list scheduled for this half hour (or every list when forced).
    pub fn run_due_polls(&mut self, force: bool) {
        let now = chrono::Local::now();
        let hhmm = now.format("%H%M").to_string();
        let stamp = format!("{} {}", today(), hhmm);
        let mut ran = vec![];
        for i in 0..self.db.poll_lists.len() {
            let l = self.db.poll_lists[i].clone();
            let due = l.schedule.contains(&hhmm) && l.last_run != stamp;
            if !(force || due) || l.phones.is_empty() {
                continue;
            }
            if l.kind == "One-time" && !l.last_run.is_empty() && !force {
                continue;
            }
            self.db.poll_lists[i].last_run = stamp.clone();
            for a in &l.phones {
                self.queue_call(a, l.commands.clone(), None, &format!("poll list {}", l.name));
            }
            ran.push(format!(
                "Poll list {} started: {} phone(s) queued for the modem",
                l.name,
                l.phones.len()
            ));
        }
        if ran.is_empty() {
            if force {
                self.message("No poll lists have phones to call");
            }
        } else {
            for r in ran {
                self.message(r);
            }
            self.save();
        }
    }

    pub fn dial_window(&mut self, ctx: &egui::Context) {
        if !self.w.dial {
            return;
        }
        let pos = at(self.area, 0.08, 0.02);
        let busy = self.dial.job.is_some();
        let online_secs = self.dial.job.as_ref().map(|j| j.started.elapsed().as_secs_f64());
        let mut ani = self.dial.ani.clone();
        let phone = ani.as_ref().and_then(|a| self.db.phone(a)).cloned();
        let target = phone.as_ref().map(|p| self.dial_target(p)).unwrap_or_default();
        let macros: Vec<Macro> = self.db.macros.clone();
        let result = if busy { None } else { self.dial.result.clone() };
        let phone_clock = self
            .dial
            .last_report
            .as_ref()
            .and_then(|r| r.status.as_ref().map(|s| s.clock.clone()).or(r.clock.clone()));
        let queued = self.dial.queue.len();
        let conn_label = self.db.settings.connection.label();
        let mut go = false;
        let mut cancel = false;
        let mut send_raw = None;
        let mut done = false;
        let mut modem = false;
        let mut apply_macro = None;
        let d = &mut self.dial;
        let db = &self.db;
        let close = style::child_window(ctx, "dial", "Dial Payphone", self.area, pos, 680.0, |ui| {
            ui.horizontal(|ui| {
                ui.label("Phone Selected:");
                let text = ani
                    .clone()
                    .map(|a| db.phone_label(&a))
                    .unwrap_or_else(|| "(select a phone)".into());
                ui.add_enabled_ui(!busy, |ui| {
                    egui::ComboBox::from_id_salt("dialphone")
                        .width(300.0)
                        .selected_text(text)
                        .show_ui(ui, |ui| {
                            for p in &db.phones {
                                ui.selectable_value(
                                    &mut ani,
                                    Some(p.ani.clone()),
                                    format!("{}  {}", p.ani, p.location),
                                );
                            }
                        });
                });
                if let Some(p) = &phone {
                    ui.label(format!("{}  {}", p.model, db.site_name(p.site_id)));
                }
            });
            ui.horizontal(|ui| {
                ui.label(format!("Connection: {conn_label}"));
                if phone.is_some() {
                    ui.label(format!("   Dials: {target}"));
                }
            });
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    style::group(ui, "Commands", |ui| {
                        ui.small("You may select more than one command. Tick all the commands you wish to send, then click the button below.");
                        egui::Frame::default().fill(style::WHITE).inner_margin(3.0).show(ui, |ui| {
                            egui::ScrollArea::vertical().id_salt("dialcmds").max_height(230.0).show(ui, |ui| {
                                ui.set_width(220.0);
                                for c in COMMANDS {
                                    let mut on = d.selected.iter().any(|x| x == c);
                                    if ui.add_enabled(!busy, egui::Checkbox::new(&mut on, *c)).changed() {
                                        if on {
                                            d.selected.push(c.to_string());
                                        } else {
                                            d.selected.retain(|x| x != c);
                                        }
                                    }
                                }
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Macro:");
                            egui::ComboBox::from_id_salt("dialmacro").width(130.0).selected_text("add commands...").show_ui(ui, |ui| {
                                for m in &macros {
                                    if ui.selectable_label(false, &m.name).clicked() {
                                        apply_macro = Some(m.commands.clone());
                                    }
                                }
                            });
                        });
                    });
                    ui.add_space(4.0);
                    let can = ani.is_some() && !busy && !d.selected.is_empty();
                    if ui.add_enabled(can, egui::Button::new("Dial Phone && Execute Commands".replace("&&", "&")).min_size(Vec2::new(240.0, 30.0))).clicked() {
                        go = true;
                    }
                    if (busy || queued > 0)
                        && ui.add(egui::Button::new("Cancel Call").min_size(Vec2::new(240.0, 26.0))).clicked()
                    {
                        cancel = true;
                    }
                    ui.horizontal(|ui| {
                        ui.label("Send command (hex):");
                        ui.add(egui::TextEdit::singleline(&mut d.raw_cmd).desired_width(36.0).char_limit(2));
                        let code = u8::from_str_radix(d.raw_cmd.trim(), 16).ok();
                        if ui
                            .add_enabled(code.is_some() && ani.is_some() && !busy, egui::Button::new("Send"))
                            .on_hover_text("Log on and send one PNM command byte; the reply is shown in Results")
                            .clicked()
                        {
                            send_raw = code;
                        }
                    });
                    ui.horizontal(|ui| {
                        if style::btn_enabled(ui, !busy, "Modem Settings") {
                            modem = true;
                        }
                        if style::btn_enabled(ui, !busy, "Close") {
                            done = true;
                        }
                    });
                });
                ui.vertical(|ui| {
                    style::group(ui, "Results", |ui| {
                        let frame = egui::Frame::default().fill(style::BLACK).inner_margin(4.0);
                        frame.show(ui, |ui| {
                            ui.set_width(390.0);
                            egui::ScrollArea::vertical().id_salt("diallog").max_height(110.0).stick_to_bottom(true).show(ui, |ui| {
                                ui.set_width(380.0);
                                if d.log.is_empty() {
                                    ui.label(egui::RichText::new("Ready.").monospace().color(Color32::from_rgb(0, 220, 0)));
                                }
                                for l in &d.log {
                                    ui.label(egui::RichText::new(l).monospace().color(Color32::from_rgb(0, 220, 0)));
                                }
                            });
                        });
                        if let Some(s) = online_secs {
                            ui.label(format!("On Line: {:.1} mins", s / 60.0));
                        }
                        if !d.status.is_empty() {
                            ui.label(format!("Status: {}{}", d.status, if queued > 0 { format!("   ({queued} more call(s) waiting)") } else { String::new() }));
                        }
                    });
                    style::group(ui, "Phone Reports", |ui| {
                        let p = phone.clone().unwrap_or_default();
                        egui::Grid::new("dialrep").num_columns(4).spacing([12.0, 2.0]).show(ui, |ui| {
                            ui.label("Software Version:");
                            ui.label(&p.pgm_ver);
                            ui.label("ROM Chip Version:");
                            ui.label(&p.rom_ver);
                            ui.end_row();
                            ui.label("Serial No.");
                            ui.label(&p.serial);
                            ui.label("Zone:");
                            ui.label(db.site(p.site_id).map(|s| s.time_zone.clone()).unwrap_or_default());
                            ui.end_row();
                            ui.label("Date/Time:");
                            ui.label(result.as_ref().map(|r| r.time.clone()).unwrap_or_default());
                            ui.label("Phone clock:");
                            ui.label(phone_clock.clone().unwrap_or_default());
                            ui.end_row();
                        });
                    });
                    ui.horizontal_top(|ui| {
                        style::group(ui, "Call Counts", |ui| {
                            egui::Grid::new("dialcounts").num_columns(2).spacing([12.0, 1.0]).show(ui, |ui| {
                                let counts = result.as_ref().map(|r| r.counts).unwrap_or_default();
                                for (k, v) in counts.rows() {
                                    ui.label(k);
                                    ui.label(v.to_string());
                                    ui.end_row();
                                }
                                ui.strong("Total Calls");
                                ui.strong(counts.total().to_string());
                                ui.end_row();
                            });
                        });
                        ui.vertical(|ui| {
                            style::group(ui, "Alarms Reported", |ui| {
                                ui.set_width(170.0);
                                match &result {
                                    Some(r) if !r.alarms.is_empty() => {
                                        for a in &r.alarms {
                                            ui.colored_label(Color32::from_rgb(180, 0, 0), a);
                                        }
                                    }
                                    Some(_) => {
                                        ui.label("No Alarms reported");
                                    }
                                    None => {
                                        ui.label("");
                                    }
                                }
                            });
                            let p = phone.clone().unwrap_or_default();
                            egui::Grid::new("dialmoney").num_columns(2).show(ui, |ui| {
                                ui.label("Cashbox $");
                                ui.label(money(p.cashbox));
                                ui.end_row();
                                ui.label("Last Collected $");
                                ui.label(money(p.last_collected));
                                ui.end_row();
                                ui.label("Totalizer $");
                                ui.label(money(p.totalizer));
                                ui.end_row();
                            });
                        });
                    });
                });
            });
        });
        if let Some(cmds) = apply_macro {
            for c in cmds {
                if !self.dial.selected.contains(&c) {
                    self.dial.selected.push(c);
                }
            }
        }
        if ani != self.dial.ani {
            self.dial.result = None;
            self.dial.log.clear();
            if let Some(a) = &ani {
                self.set_active(a);
            }
        }
        self.dial.ani = ani;
        if go {
            if let Some(a) = self.dial.ani.clone() {
                self.dial.result = None;
                let cmds = self.dial.selected.clone();
                self.queue_call(&a, cmds, None, "Dial Payphone");
            }
        }
        if let (Some(code), Some(a)) = (send_raw, self.dial.ani.clone()) {
            self.queue_call(&a, vec![], Some(code), &format!("command {code:02X}"));
        }
        if cancel {
            self.cancel_calls();
        }
        if modem {
            self.cfg.settings = self.db.settings.clone();
            self.w.modem = true;
        }
        if (done || close) && !busy {
            self.w.dial = false;
        }
    }

    pub fn poll_windows(&mut self, ctx: &egui::Context) {
        self.poll_main(ctx);
        self.poll_phones(ctx);
        self.poll_cmds(ctx);
        self.poll_sched(ctx);
        self.poll_new(ctx);
    }

    fn poll_main(&mut self, ctx: &egui::Context) {
        if !self.w.poll {
            return;
        }
        let pos = at(self.area, 0.12, 0.06);
        let rows: Vec<(u32, String)> = self
            .db
            .poll_lists
            .iter()
            .map(|l| (l.id, format!("{:<24} {}", l.name, l.kind)))
            .collect();
        let mut sel = self.poll.sel;
        if sel.is_none() {
            sel = rows.first().map(|r| r.0);
        }
        let cur = sel
            .and_then(|id| self.db.poll_lists.iter().find(|l| l.id == id))
            .cloned();
        if let Some(c) = &cur {
            if self.poll.sel != sel {
                self.poll.edit_name = c.name.clone();
            }
        }
        let phones: Vec<(String, String)> = cur
            .as_ref()
            .map(|c| {
                c.phones
                    .iter()
                    .map(|a| {
                        (
                            a.clone(),
                            format!(
                                "{:<13} {}",
                                a,
                                self.db.phone(a).map(|p| p.location.clone()).unwrap_or_default()
                            ),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let cmds: Vec<(String, String)> = cur
            .as_ref()
            .map(|c| c.commands.iter().map(|x| (x.clone(), x.clone())).collect())
            .unwrap_or_default();
        let mut action = "";
        let name = &mut self.poll.edit_name;
        let close = style::child_window(ctx, "poll", "Poll List Maintenance", self.area, pos, 620.0, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.label("Poll Lists");
                    style::list_box(ui, "polllists", &rows, &mut sel, Vec2::new(220.0, 250.0));
                });
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add_enabled(cur.is_some(), egui::TextEdit::singleline(name).desired_width(220.0));
                        if style::btn_enabled(ui, cur.is_some(), "Update") {
                            action = "rename";
                        }
                    });
                    if let Some(c) = &cur {
                        ui.label(format!(
                            "{}   Schedule: {}   Last run: {}",
                            c.kind,
                            if c.schedule.is_empty() {
                                "none".into()
                            } else {
                                c.schedule.join(", ")
                            },
                            if c.last_run.is_empty() { "never" } else { &c.last_run }
                        ));
                    }
                    style::header(ui, &format!("{:<13} {}", "Phone", "Description"));
                    let mut s = None;
                    style::list_box(ui, "pollphones", &phones, &mut s, Vec2::new(370.0, 120.0));
                    ui.label("Commands");
                    let mut s2 = None;
                    style::list_box(ui, "pollcmds", &cmds, &mut s2, Vec2::new(370.0, 80.0));
                });
            });
            ui.horizontal(|ui| {
                if style::btn(ui, "New") {
                    action = "new";
                }
                if style::btn_enabled(ui, cur.is_some(), "Delete") {
                    action = "delete";
                }
                if style::btn_enabled(ui, cur.is_some(), "Phones...") {
                    action = "phones";
                }
                if style::btn_enabled(ui, cur.is_some(), "Commands...") {
                    action = "cmds";
                }
                if style::btn_enabled(ui, cur.is_some(), "Schedule...") {
                    action = "sched";
                }
                if style::btn_enabled(
                    ui,
                    cur.as_ref().map(|c| !c.phones.is_empty()).unwrap_or(false),
                    "Run Now",
                ) {
                    action = "run";
                }
                if style::btn(ui, "Close") {
                    action = "close";
                }
            });
        });
        if self.poll.sel != sel {
            self.poll.edit_name = sel
                .and_then(|id| self.db.poll_lists.iter().find(|l| l.id == id))
                .map(|l| l.name.clone())
                .unwrap_or_default();
        }
        self.poll.sel = sel;
        let id = sel.unwrap_or(0);
        match action {
            "new" => {
                self.poll.new_list = Some(PollList {
                    kind: "Recurring".into(),
                    ..Default::default()
                })
            }
            "rename" => {
                let n = self.poll.edit_name.trim().to_string();
                if !n.is_empty() {
                    if let Some(l) = self.db.poll_lists.iter_mut().find(|l| l.id == id) {
                        l.name = n;
                    }
                    self.save();
                }
            }
            "delete" => {
                self.db.poll_lists.retain(|l| l.id != id);
                self.poll.sel = None;
                self.save();
            }
            "phones" => self.poll.phones_for = Some(id),
            "cmds" => self.poll.cmds_for = Some(id),
            "sched" => {
                self.poll.sched_for = Some(id);
                self.poll.sched_hour.clear();
            }
            "run" => {
                if let Some(l) = self.db.poll_lists.iter().find(|l| l.id == id).cloned() {
                    for a in &l.phones {
                        self.queue_call(a, l.commands.clone(), None, &format!("poll list {}", l.name));
                    }
                    if let Some(x) = self.db.poll_lists.iter_mut().find(|x| x.id == id) {
                        x.last_run = now_stamp();
                    }
                    self.open_dial(None);
                    self.message(format!(
                        "Poll list {}: {} call(s) queued for the modem",
                        l.name,
                        l.phones.len()
                    ));
                    self.save();
                }
            }
            "close" => self.w.poll = false,
            _ => {}
        }
        if close {
            self.w.poll = false;
        }
    }

    fn poll_phones(&mut self, ctx: &egui::Context) {
        let Some(id) = self.poll.phones_for else { return };
        let Some(list) = self.db.poll_lists.iter().find(|l| l.id == id).cloned() else {
            self.poll.phones_for = None;
            return;
        };
        let pos = at(self.area, 0.2, 0.15);
        let right: Vec<(String, String)> = list
            .phones
            .iter()
            .map(|a| (a.clone(), self.db.phone_label(a)))
            .collect();
        let left: Vec<(String, String)> = self
            .db
            .phones
            .iter()
            .filter(|p| !list.phones.contains(&p.ani))
            .map(|p| (p.ani.clone(), self.db.phone_label(&p.ani)))
            .collect();
        let mut out = list.phones.clone();
        let mut ok = false;
        let pick = &mut self.poll.phones_pick;
        let close = style::child_window(
            ctx,
            "pollph",
            "Assign Phones To Poll List",
            self.area,
            pos,
            560.0,
            |ui| {
                ui.label(format!("Phones on Poll List: {}", list.name));
                let (a, r, aa, ra) = dual_list(
                    ui,
                    "pph",
                    "All Phones",
                    &left,
                    "Phones on Poll List",
                    &right,
                    pick,
                    220.0,
                    180.0,
                );
                if let Some(a) = a {
                    out.push(a);
                }
                if let Some(r) = r {
                    out.retain(|x| *x != r);
                }
                if aa {
                    out.extend(left.iter().map(|(k, _)| k.clone()));
                }
                if ra {
                    out.clear();
                }
                ok = style::btn(ui, "OK");
            },
        );
        if out != list.phones {
            if let Some(l) = self.db.poll_lists.iter_mut().find(|l| l.id == id) {
                l.phones = out;
            }
            self.save();
        }
        if ok || close {
            self.poll.phones_for = None;
        }
    }

    fn poll_cmds(&mut self, ctx: &egui::Context) {
        let Some(id) = self.poll.cmds_for else { return };
        let Some(list) = self.db.poll_lists.iter().find(|l| l.id == id).cloned() else {
            self.poll.cmds_for = None;
            return;
        };
        let pos = at(self.area, 0.22, 0.1);
        let right: Vec<(String, String)> = list.commands.iter().map(|c| (c.clone(), c.clone())).collect();
        let left: Vec<(String, String)> = COMMANDS
            .iter()
            .filter(|c| !list.commands.iter().any(|x| x == *c))
            .map(|c| (c.to_string(), c.to_string()))
            .collect();
        let mut out = list.commands.clone();
        let mut ok = false;
        let mut cancel = false;
        let pick = &mut self.poll.cmd_pick;
        let close = style::child_window(ctx, "pollcmd", "Poll List Commands", self.area, pos, 560.0, |ui| {
            ui.label(format!("Poll List Commands for: {}", list.name));
            let (a, r, aa, ra) = dual_list(
                ui,
                "pcm",
                "Available Commands",
                &left,
                "Commands to send",
                &right,
                pick,
                220.0,
                200.0,
            );
            if let Some(a) = a {
                out.push(a);
            }
            if let Some(r) = r {
                out.retain(|x| *x != r);
            }
            if aa {
                out.extend(left.iter().map(|(k, _)| k.clone()));
            }
            if ra {
                out.clear();
            }
            ui.horizontal(|ui| {
                ok = style::btn(ui, "OK");
                cancel = style::btn(ui, "Cancel");
            });
        });
        if out != list.commands {
            if let Some(l) = self.db.poll_lists.iter_mut().find(|l| l.id == id) {
                l.commands = out;
            }
            self.save();
        }
        if ok || cancel || close {
            self.poll.cmds_for = None;
        }
    }

    fn poll_sched(&mut self, ctx: &egui::Context) {
        let Some(id) = self.poll.sched_for else { return };
        let Some(list) = self.db.poll_lists.iter().find(|l| l.id == id).cloned() else {
            self.poll.sched_for = None;
            return;
        };
        let pos = at(self.area, 0.3, 0.2);
        let rows: Vec<(String, String)> = list.schedule.iter().map(|s| (s.clone(), s.clone())).collect();
        let mut sel = None;
        let mut add = false;
        let mut rem = None;
        let mut ok = false;
        let hour = &mut self.poll.sched_hour;
        let mut err = String::new();
        let close = style::child_window(ctx, "pollsched", "Poll Schedule", self.area, pos, 320.0, |ui| {
            ui.label(format!("Poll List Name: {}", list.name));
            ui.label("Enter time from 0000 to 2330 in 30 minute increments.");
            ui.horizontal(|ui| {
                ui.label("Hour:");
                ui.add(egui::TextEdit::singleline(hour).desired_width(60.0));
                if style::btn(ui, "Add") {
                    add = true;
                }
            });
            if let Some(d) = style::list_box(ui, "schedlist", &rows, &mut sel, Vec2::new(300.0, 110.0)) {
                rem = Some(d);
            }
            ui.small("Double-click a time to remove it.");
            ok = style::btn(ui, "OK");
        });
        if add {
            let h = self.poll.sched_hour.trim().replace(':', "");
            let valid = h.len() == 4
                && h.chars().all(|c| c.is_ascii_digit())
                && h[..2].parse::<u32>().map(|v| v < 24).unwrap_or(false)
                && (h[2..] == *"00" || h[2..] == *"30");
            if valid {
                if let Some(l) = self.db.poll_lists.iter_mut().find(|l| l.id == id) {
                    if !l.schedule.contains(&h) {
                        l.schedule.push(h);
                        l.schedule.sort();
                    }
                }
                self.poll.sched_hour.clear();
                self.save();
            } else {
                err = "Enter a time like 0200 or 1430 (on the hour or half hour).".into();
            }
        }
        if !err.is_empty() {
            self.info = err;
        }
        if let Some(r) = rem {
            if let Some(l) = self.db.poll_lists.iter_mut().find(|l| l.id == id) {
                l.schedule.retain(|s| *s != r);
            }
            self.save();
        }
        if ok || close {
            self.poll.sched_for = None;
        }
    }

    fn poll_new(&mut self, ctx: &egui::Context) {
        let pos = at(self.area, 0.3, 0.25);
        let Some(l) = self.poll.new_list.as_mut() else { return };
        let mut ok = false;
        let mut cancel = false;
        let close = style::child_window(
            ctx,
            "pollnew",
            "Enter New Poll List Information",
            self.area,
            pos,
            330.0,
            |ui| {
                ui.label("Enter Poll List Name:");
                ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(300.0));
                ui.label("Set Poll List Type:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut l.kind, "Recurring".to_string(), "Recurring");
                    ui.radio_value(&mut l.kind, "One-time".to_string(), "One-time");
                });
                ui.horizontal(|ui| {
                    ok = style::btn(ui, "OK");
                    cancel = style::btn(ui, "Cancel");
                });
            },
        );
        if ok {
            let mut l = self.poll.new_list.take().unwrap();
            if l.name.trim().is_empty() {
                l.name = "New Poll List".into();
            }
            l.id = self.db.new_id();
            l.commands = vec!["Get Call Counts".into(), "Get Alarms".into()];
            self.poll.sel = Some(l.id);
            self.poll.edit_name = l.name.clone();
            self.db.poll_lists.push(l);
            self.save();
        } else if cancel || close {
            self.poll.new_list = None;
        }
    }
}
