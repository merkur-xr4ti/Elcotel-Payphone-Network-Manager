//! Service management: phone status and alarms, work orders, customer service, diagnostics, RAM image.

use crate::app::{at, App};
use crate::model::*;
use crate::style;
use eframe::egui::{self, Color32, Vec2};

impl App {
    pub fn service_windows(&mut self, ctx: &egui::Context) {
        self.status_window(ctx);
        self.handle_window(ctx);
        self.wo_window(ctx);
        self.wo_edit_window(ctx);
        self.complaints_window(ctx);
        self.complaint_edit_window(ctx);
        self.letter_window(ctx);
        self.diag_window(ctx);
        self.conhist_window(ctx);
        self.ram_window(ctx);
    }

    fn phone_picker(&self, ui: &mut egui::Ui, id: &str, value: &mut Option<String>, width: f32) {
        let text = value
            .clone()
            .map(|a| self.db.phone_label(&a))
            .unwrap_or_else(|| "(select a phone)".into());
        egui::ComboBox::from_id_salt(id)
            .width(width)
            .selected_text(text)
            .show_ui(ui, |ui| {
                for p in &self.db.phones {
                    ui.selectable_value(value, Some(p.ani.clone()), format!("{}  {}", p.ani, p.location));
                }
            });
    }

    fn status_window(&mut self, ctx: &egui::Context) {
        if !self.w.status {
            return;
        }
        let models = self.svc.status_models.clone();
        let rows: Vec<(u32, String)> = self
            .db
            .alarms
            .iter()
            .filter(|a| !a.handled)
            .filter(|a| models.is_empty() || self.db.phone(&a.ani).map(|p| models.contains(&p.model)).unwrap_or(true))
            .map(|a| {
                let loc = self.db.phone(&a.ani).map(|p| p.location.clone()).unwrap_or_default();
                (
                    a.id,
                    format!(
                        "{:<13} {:<18} {:<22} {}",
                        a.ani,
                        trunc(&loc, 18),
                        trunc(&a.description, 22),
                        a.time
                    ),
                )
            })
            .collect();
        let quiet: Vec<String> = self
            .db
            .phones
            .iter()
            .filter(|p| p.last_contact.is_empty() || p.needs_update)
            .map(|p| {
                format!(
                    "{}  {}",
                    p.ani,
                    if p.needs_update {
                        "needs a download"
                    } else {
                        "never contacted"
                    }
                )
            })
            .collect();
        let mut sel = self.svc.status_sel;
        let mut handle = false;
        let mut history = false;
        let mut redial = false;
        let mut done = false;
        let mut filter = false;
        let mut dbl = None;
        let close = style::child_window(
            ctx,
            "status",
            "Phone Management",
            self.area,
            at(self.area, 0.05, 0.08),
            600.0,
            |ui| {
                style::header(
                    ui,
                    &format!("{:<13} {:<18} {:<22} {}", "ANI", "Location", "Alarm", "Reported"),
                );
                dbl = style::list_box(ui, "statuslist", &rows, &mut sel, Vec2::new(588.0, 200.0));
                ui.label(format!("{} alarm(s) waiting to be handled", rows.len()));
                if !quiet.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("Attention: {}", quiet.join(";  ")))
                            .small()
                            .color(Color32::from_rgb(128, 0, 0)),
                    );
                }
                ui.horizontal(|ui| {
                    style::btn(ui, "Refresh"); // the list is rebuilt every frame
                    if style::btn(ui, "Filter") {
                        filter = true;
                    }
                    if style::btn_enabled(ui, !rows.is_empty(), "Redial All") {
                        redial = true;
                    }
                    if style::btn_enabled(ui, sel.is_some(), "Handle") {
                        handle = true;
                    }
                    if style::btn_enabled(ui, sel.is_some(), "History") {
                        history = true;
                    }
                    if style::btn(ui, "Close") {
                        done = true;
                    }
                });
                if !models.is_empty() {
                    ui.small(format!("Showing models: {}", models.join(", ")));
                }
            },
        );
        self.svc.status_sel = sel;
        if filter {
            self.svc.show_filter = true;
        }
        if handle || dbl.is_some() {
            self.svc.handle_alarm = dbl.or(sel);
            self.svc.handle_note.clear();
            self.w.handle = true;
        }
        if history {
            if let Some(a) = sel.and_then(|id| self.db.alarms.iter().find(|a| a.id == id)) {
                self.svc.conhist_ani = Some(a.ani.clone());
                self.w.conhist = true;
            }
        }
        if redial {
            let anis: Vec<String> = {
                let mut v: Vec<String> = self
                    .db
                    .alarms
                    .iter()
                    .filter(|a| !a.handled)
                    .map(|a| a.ani.clone())
                    .collect();
                v.dedup();
                v
            };
            let cmds: Vec<String> = vec!["Get Alarms".into(), "Get Call Counts".into()];
            for a in &anis {
                self.queue_call(a, cmds.clone(), None, "Redial All");
            }
            self.open_dial(None);
            self.message(format!("{} call(s) queued for the modem", anis.len()));
            return;
        }
        if close || done {
            self.w.status = false;
        }
        if self.svc.show_filter {
            let mut chosen = self.svc.status_models.clone();
            let mut ok = false;
            let c2 = style::child_window(
                ctx,
                "alarmfilter",
                "Select Category of Phones to Display",
                self.area,
                at(self.area, 0.3, 0.2),
                300.0,
                |ui| {
                    ui.label("Show alarms for these phone models (none ticked = all):");
                    for m in MODELS {
                        let mut on = chosen.iter().any(|x| x == m);
                        if ui.checkbox(&mut on, *m).changed() {
                            if on {
                                chosen.push(m.to_string());
                            } else {
                                chosen.retain(|x| x != m);
                            }
                        }
                    }
                    ui.horizontal(|ui| {
                        if style::btn(ui, "Add All") {
                            chosen = MODELS.iter().map(|s| s.to_string()).collect();
                        }
                        if style::btn(ui, "Remove All") {
                            chosen.clear();
                        }
                        if style::btn(ui, "OK") {
                            ok = true;
                        }
                    });
                },
            );
            self.svc.status_models = chosen;
            if ok || c2 {
                self.svc.show_filter = false;
            }
        }
    }

    fn handle_window(&mut self, ctx: &egui::Context) {
        if !self.w.handle {
            return;
        }
        let Some(alarm) = self
            .svc
            .handle_alarm
            .and_then(|id| self.db.alarms.iter().find(|a| a.id == id))
            .cloned()
        else {
            self.w.handle = false;
            return;
        };
        let phone = self.db.phone(&alarm.ani).cloned().unwrap_or_default();
        let site = self.db.site(phone.site_id).cloned().unwrap_or_default();
        let route = self
            .db
            .route(site.route_id)
            .map(|r| r.description.clone())
            .unwrap_or_else(|| "(none)".into());
        let tech = self.db.tech_of_route(site.route_id).cloned().unwrap_or_default();
        let mut ok = false;
        let mut cancel = false;
        let mut dial = false;
        let mut wo = false;
        let mut hist = false;
        let note = &mut self.svc.handle_note;
        let close = style::child_window(
            ctx,
            "handle",
            "Handle Reported Alarm",
            self.area,
            at(self.area, 0.2, 0.12),
            520.0,
            |ui| {
                ui.label(
                    egui::RichText::new(format!("{}  {} - {}", phone.ani, site.name, phone.location))
                        .strong()
                        .color(style::BLUE_TEXT),
                );
                egui::Grid::new("hgrid")
                    .num_columns(2)
                    .spacing([10.0, 3.0])
                    .show(ui, |ui| {
                        let row = |ui: &mut egui::Ui, k: &str, v: &str| {
                            ui.label(k);
                            ui.label(v);
                            ui.end_row();
                        };
                        row(ui, "Problem:", &alarm.description);
                        row(ui, "Reported:", &alarm.time);
                        row(ui, "Route:", &route);
                        row(ui, "Site Contact:", &site.contact);
                        row(ui, "Site Phone:", &site.phone);
                        row(
                            ui,
                            "Technician:",
                            if tech.name.is_empty() {
                                "(none assigned)"
                            } else {
                                &tech.name
                            },
                        );
                        row(ui, "Business:", &tech.bus_phone);
                        row(ui, "Cellular:", &tech.cellular);
                        row(ui, "FAX:", &tech.fax);
                        row(ui, "Beeper:", &tech.beeper);
                    });
                ui.label("How it was handled:");
                ui.add(egui::TextEdit::multiline(note).desired_width(500.0).desired_rows(2));
                ui.horizontal(|ui| {
                    if style::btn(ui, "OK") {
                        ok = true;
                    }
                    if style::btn(ui, "Cancel") {
                        cancel = true;
                    }
                    if style::btn(ui, "Work Orders") {
                        wo = true;
                    }
                    if style::btn(ui, "Dial Phone") {
                        dial = true;
                    }
                    if style::btn(ui, "View History") {
                        hist = true;
                    }
                });
            },
        );
        if ok {
            let n = self.svc.handle_note.clone();
            if let Some(a) = self.db.alarms.iter_mut().find(|a| a.id == alarm.id) {
                a.handled = true;
                a.handled_note = if n.is_empty() {
                    format!("Handled by {}", self.db.settings.user_name)
                } else {
                    n
                };
            }
            self.message(format!("Alarm on {} handled", alarm.ani));
            self.save();
        }
        if wo {
            let next = self.db.work_orders.iter().map(|w| w.number).max().unwrap_or(1000) + 1;
            self.svc.wo_edit = Some(WorkOrder {
                number: next,
                ani: alarm.ani.clone(),
                opened: today(),
                status: "Open".into(),
                problem: alarm.description.clone(),
                tech_id: if tech.id == 0 { None } else { Some(tech.id) },
                ..Default::default()
            });
            self.svc.wo_new = true;
        }
        if dial {
            self.open_dial(Some(alarm.ani.clone()));
        }
        if hist {
            self.svc.conhist_ani = Some(alarm.ani.clone());
            self.w.conhist = true;
        }
        if ok || cancel || close {
            self.w.handle = false;
        }
    }

    fn wo_window(&mut self, ctx: &egui::Context) {
        if !self.w.wo {
            return;
        }
        if self.svc.wo_status.is_empty() {
            self.svc.wo_status = "Open".into();
        }
        let from = self.svc.wo_from.clone();
        let to = self.svc.wo_to.clone();
        let status = self.svc.wo_status.clone();
        let mut list: Vec<&WorkOrder> = self
            .db
            .work_orders
            .iter()
            .filter(|w| status == "All" || w.status == status)
            .filter(|w| from.is_empty() || w.opened.as_str() >= from.as_str())
            .filter(|w| to.is_empty() || w.opened.as_str() <= to.as_str())
            .collect();
        list.sort_by_key(|w| std::cmp::Reverse(w.number));
        let rows: Vec<(u32, String)> = list
            .iter()
            .map(|w| {
                (
                    w.number,
                    format!(
                        "{:<6} {:<13} {:<10} {:<7} {:<16} {}",
                        w.number,
                        w.ani,
                        w.opened,
                        w.status,
                        trunc(&self.db.tech_name(w.tech_id), 16),
                        w.problem
                    ),
                )
            })
            .collect();
        let mut sel = self.svc.wo_sel;
        let mut newb = false;
        let mut upd = false;
        let mut done = false;
        let s = &mut self.svc;
        let close = style::child_window(
            ctx,
            "wo",
            "Work Order Management",
            self.area,
            at(self.area, 0.06, 0.1),
            640.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Opened from:");
                    ui.add(
                        egui::TextEdit::singleline(&mut s.wo_from)
                            .desired_width(90.0)
                            .hint_text("YYYY-MM-DD"),
                    );
                    ui.label("to:");
                    ui.add(
                        egui::TextEdit::singleline(&mut s.wo_to)
                            .desired_width(90.0)
                            .hint_text("YYYY-MM-DD"),
                    );
                    ui.label("Status:");
                    for st in ["Open", "Closed", "All"] {
                        ui.radio_value(&mut s.wo_status, st.to_string(), st);
                    }
                });
                style::header(
                    ui,
                    &format!(
                        "{:<6} {:<13} {:<10} {:<7} {:<16} {}",
                        "No.", "Payphone", "Opened", "Status", "Technician", "Problem"
                    ),
                );
                if let Some(d) = style::list_box(ui, "wolist", &rows, &mut sel, Vec2::new(628.0, 220.0)) {
                    sel = Some(d);
                    upd = true;
                }
                ui.horizontal(|ui| {
                    style::btn(ui, "Refresh View"); // the view is rebuilt every frame
                    if style::btn(ui, "New...") {
                        newb = true;
                    }
                    if style::btn_enabled(ui, sel.is_some(), "Update...") {
                        upd = true;
                    }
                    if style::btn(ui, "Close") {
                        done = true;
                    }
                });
            },
        );
        self.svc.wo_sel = sel;
        if newb {
            let next = self.db.work_orders.iter().map(|w| w.number).max().unwrap_or(1000) + 1;
            let ani = self.active_ani.clone().unwrap_or_default();
            let tech = self
                .db
                .phone(&ani)
                .and_then(|p| self.db.site(p.site_id))
                .and_then(|s| self.db.tech_of_route(s.route_id))
                .map(|t| t.id);
            self.svc.wo_edit = Some(WorkOrder {
                number: next,
                ani,
                opened: today(),
                status: "Open".into(),
                tech_id: tech,
                ..Default::default()
            });
            self.svc.wo_new = true;
        }
        if upd {
            if let Some(w) = sel.and_then(|n| self.db.work_orders.iter().find(|w| w.number == n)) {
                self.svc.wo_edit = Some(w.clone());
                self.svc.wo_new = false;
                self.svc.wo_action.clear();
                self.svc.wo_close_it = false;
            }
        }
        if close || done {
            self.w.wo = false;
        }
    }

    fn wo_edit_window(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.svc.wo_edit.as_mut() else { return };
        let is_new = self.svc.wo_new;
        let techs: Vec<(u32, String)> = self.db.techs.iter().map(|t| (t.id, t.name.clone())).collect();
        let codes: Vec<String> = self.db.account_codes.iter().map(|a| a.code.clone()).collect();
        let phones: Vec<String> = self.db.phones.iter().map(|p| p.ani.clone()).collect();
        let action = &mut self.svc.wo_action;
        let close_it = &mut self.svc.wo_close_it;
        let mut ok = false;
        let mut cancel = false;
        let title = if is_new {
            "New Work Order Entry"
        } else {
            "Work Order Status"
        };
        let close = style::child_window(ctx, "woedit", title, self.area, at(self.area, 0.2, 0.06), 480.0, |ui| {
            egui::Grid::new("wogrid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Number");
                    ui.label(ed.number.to_string());
                    ui.end_row();
                    ui.label("Payphone");
                    style::combo(ui, "woani", &mut ed.ani, &phones, 160.0);
                    ui.end_row();
                    ui.label("Opened");
                    ui.label(&ed.opened);
                    ui.end_row();
                    if !is_new {
                        ui.label("Status:");
                        ui.label(if ed.closed.is_empty() {
                            ed.status.clone()
                        } else {
                            format!("{}  (closed {})", ed.status, ed.closed)
                        });
                        ui.end_row();
                    }
                    style::field(ui, "Problem", &mut ed.problem, 320.0);
                    ui.label("Technician");
                    style::combo_id(ui, "wotech", &mut ed.tech_id, &techs, 200.0);
                    ui.end_row();
                    ui.label("Account Code");
                    style::combo(ui, "wocode", &mut ed.account_code, &codes, 100.0);
                    ui.end_row();
                    style::num_field(ui, "Amount $", &mut ed.amount, 90.0);
                    ui.label("Notes:");
                    ui.add(
                        egui::TextEdit::multiline(&mut ed.notes)
                            .desired_width(320.0)
                            .desired_rows(2),
                    );
                    ui.end_row();
                });
            if !ed.actions.is_empty() {
                ui.label("Action taken so far:");
                for a in &ed.actions {
                    ui.label(format!("  {}  {}", a.date, a.text));
                }
            }
            if ed.status != "Closed" {
                ui.label(if is_new { "Action Taken" } else { "New Action" });
                ui.add(egui::TextEdit::singleline(action).desired_width(460.0));
                if !is_new {
                    ui.checkbox(close_it, "Close this work order (problem solved)");
                }
            }
            ui.horizontal(|ui| {
                if style::btn(ui, "OK") {
                    ok = true;
                }
                if style::btn(ui, "Cancel") {
                    cancel = true;
                }
            });
        });
        if ok {
            let mut w = self.svc.wo_edit.take().unwrap();
            let a = self.svc.wo_action.trim().to_string();
            if !a.is_empty() {
                w.actions.push(WoAction { date: today(), text: a });
            }
            if self.svc.wo_close_it {
                w.status = "Closed".into();
                w.closed = today();
            }
            if is_new {
                self.db.work_orders.push(w.clone());
                // the work order's cost goes into the cost accounting journal
                if w.amount != 0.0 {
                    let id = self.db.new_id();
                    self.db.journal.push(JournalEntry {
                        id,
                        date: w.opened.clone(),
                        ani: w.ani.clone(),
                        account_code: w.account_code.clone(),
                        amount: w.amount,
                        tech_id: w.tech_id,
                        notes: format!("Work order {}", w.number),
                    });
                }
            } else if let Some(x) = self.db.work_orders.iter_mut().find(|x| x.number == w.number) {
                *x = w.clone();
            }
            self.message(format!("Work order {} saved", w.number));
            self.svc.wo_action.clear();
            self.svc.wo_close_it = false;
            self.save();
            self.w.wo = true;
        } else if cancel || close {
            self.svc.wo_edit = None;
        }
    }

    fn complaints_window(&mut self, ctx: &egui::Context) {
        if !self.w.complaints {
            return;
        }
        let mut list: Vec<&Complaint> = self.db.complaints.iter().collect();
        list.sort_by(|a, b| b.date.cmp(&a.date));
        let rows: Vec<(u32, String)> = list
            .iter()
            .map(|c| {
                (
                    c.id,
                    format!(
                        "{:<10} {:<13} {:<16} {:>7} {}",
                        c.date,
                        c.ani,
                        trunc(&c.name, 16),
                        money(c.amount),
                        c.reason
                    ),
                )
            })
            .collect();
        let mut sel = self.svc.complaint_sel;
        let mut newb = false;
        let mut open = None;
        let mut done = false;
        let close = style::child_window(
            ctx,
            "complaints",
            "Customer Service History",
            self.area,
            at(self.area, 0.08, 0.14),
            600.0,
            |ui| {
                style::header(
                    ui,
                    &format!(
                        "{:<10} {:<13} {:<16} {:>7} {}",
                        "Date", "ANI", "Name", "Amount", "Complaint Reason"
                    ),
                );
                if let Some(d) = style::list_box(ui, "complist", &rows, &mut sel, Vec2::new(588.0, 200.0)) {
                    open = Some(d);
                }
                let total: f64 = list.iter().map(|c| c.amount).sum();
                ui.label(format!("{} complaint(s), refunds {}", list.len(), money(total)));
                ui.horizontal(|ui| {
                    if style::btn(ui, "New") {
                        newb = true;
                    }
                    if style::btn_enabled(ui, sel.is_some(), "View") {
                        open = sel;
                    }
                    if style::btn(ui, "Close") {
                        done = true;
                    }
                });
            },
        );
        self.svc.complaint_sel = sel;
        if newb {
            let ani = self.active_ani.clone().unwrap_or_default();
            let loc = self.db.phone(&ani).map(|p| p.location.clone()).unwrap_or_default();
            self.svc.complaint_edit = Some(Complaint {
                id: 0,
                date: today(),
                time: chrono::Local::now().format("%H:%M").to_string(),
                ani,
                location: loc,
                account_code: "220".into(),
                ..Default::default()
            });
        }
        if let Some(id) = open {
            self.svc.complaint_edit = self.db.complaints.iter().find(|c| c.id == id).cloned();
        }
        if close || done {
            self.w.complaints = false;
        }
    }

    fn complaint_edit_window(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.svc.complaint_edit.as_mut() else {
            return;
        };
        let phones: Vec<String> = self.db.phones.iter().map(|p| p.ani.clone()).collect();
        let codes: Vec<String> = self.db.account_codes.iter().map(|a| a.code.clone()).collect();
        let mut ok = false;
        let mut cancel = false;
        let mut letter = false;
        let mut history = false;
        let close = style::child_window(
            ctx,
            "complaint",
            "Customer Service",
            self.area,
            at(self.area, 0.22, 0.05),
            470.0,
            |ui| {
                egui::Grid::new("cgrid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Date Entered:");
                        ui.label(format!("{}   TOD {}", ed.date, ed.time));
                        ui.end_row();
                        ui.label("ANI");
                        let before = ed.ani.clone();
                        style::combo(ui, "cani", &mut ed.ani, &phones, 150.0);
                        if before != ed.ani {
                            ed.location = self.db.phone(&ed.ani).map(|p| p.location.clone()).unwrap_or_default();
                        }
                        ui.end_row();
                        style::field(ui, "Location name", &mut ed.location, 250.0);
                        style::field(ui, "Name", &mut ed.name, 250.0);
                        style::field(ui, "Address", &mut ed.address, 250.0);
                        style::field(ui, "City", &mut ed.city, 160.0);
                        style::field(ui, "State", &mut ed.state, 40.0);
                        style::field(ui, "Zip", &mut ed.zip, 80.0);
                        style::field(ui, "Phone", &mut ed.phone, 120.0);
                        style::field(ui, "Complaint Reason", &mut ed.reason, 300.0);
                        ui.label("Accounting Code");
                        style::combo(ui, "ccode", &mut ed.account_code, &codes, 100.0);
                        ui.end_row();
                        style::num_field(ui, "Amount $", &mut ed.amount, 90.0);
                    });
                ui.horizontal(|ui| {
                    if style::btn(ui, "OK") {
                        ok = true;
                    }
                    if style::btn(ui, "Cancel") {
                        cancel = true;
                    }
                    if style::btn(ui, "Print Letter") {
                        letter = true;
                    }
                    if style::btn(ui, "Show History") {
                        history = true;
                    }
                });
            },
        );
        if letter {
            let c = ed.clone();
            let co = self.db.settings.company.clone();
            self.svc.letter = Some(format!(
                "{co}\n{}\n\n{}\n{}\n{}, {} {}\n\nDear {},\n\nThank you for letting us know about the problem you had on {} \
                 with the payphone at {} ({}).\n\nReason reported: {}\n\nPlease find enclosed a refund of {}.\nWe are sorry for the trouble.\n\nSincerely,\n\n{}\nCustomer Service",
                today(), c.name, c.address, c.city, c.state, c.zip, if c.name.is_empty() { "Customer" } else { &c.name }, c.date, c.location, c.ani, c.reason, money(c.amount), self.db.settings.user_name
            ));
        }
        if history {
            self.w.complaints = true;
        }
        if ok {
            let mut c = self.svc.complaint_edit.take().unwrap();
            if c.id == 0 {
                c.id = self.db.new_id();
                // refunds are recorded in the cost accounting journal
                if c.amount != 0.0 {
                    let jid = self.db.new_id();
                    self.db.journal.push(JournalEntry {
                        id: jid,
                        date: c.date.clone(),
                        ani: c.ani.clone(),
                        account_code: c.account_code.clone(),
                        amount: c.amount,
                        tech_id: None,
                        notes: format!("Customer service: {}", c.reason),
                    });
                }
                self.db.complaints.push(c.clone());
            } else if let Some(x) = self.db.complaints.iter_mut().find(|x| x.id == c.id) {
                *x = c.clone();
            }
            self.message(format!("Customer service record for {} saved", c.ani));
            self.save();
        } else if cancel || close {
            self.svc.complaint_edit = None;
        }
    }

    fn letter_window(&mut self, ctx: &egui::Context) {
        let Some(text) = self.svc.letter.as_mut() else { return };
        let mut done = false;
        let mut copy = false;
        let close = style::child_window(
            ctx,
            "letter",
            "Customer Letter",
            self.area,
            at(self.area, 0.3, 0.1),
            460.0,
            |ui| {
                egui::Frame::default()
                    .fill(style::WHITE)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(text)
                                .desired_width(430.0)
                                .desired_rows(16)
                                .frame(false),
                        );
                    });
                ui.horizontal(|ui| {
                    if style::btn(ui, "Copy") {
                        copy = true;
                    }
                    if style::btn(ui, "Close") {
                        done = true;
                    }
                });
            },
        );
        if copy {
            ctx.copy_text(text.clone());
            self.info = "Letter copied - paste it into your word processor to print".into();
        }
        if done || close {
            self.svc.letter = None;
        }
    }

    fn diag_window(&mut self, ctx: &egui::Context) {
        if !self.w.diag {
            return;
        }
        let mut ani = self.svc.diag_ani.clone();
        let ty = self.svc.diag_type.clone();
        let records: Vec<&CallRecord> = self
            .db
            .call_records
            .iter()
            .filter(|r| Some(&r.ani) == ani.as_ref())
            .filter(|r| ty.is_empty() || r.call_type == ty)
            .collect();
        let last = ani
            .as_ref()
            .and_then(|a| {
                self.db
                    .contacts
                    .iter()
                    .rev()
                    .find(|c| &c.ani == a && c.result == "Completed")
            })
            .cloned();
        let mut done = false;
        let mut pick_type = self.svc.diag_type.clone();
        let close = style::child_window(
            ctx,
            "diag",
            "Phone Diagnostic Information",
            self.area,
            at(self.area, 0.1, 0.05),
            600.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Phone:");
                    self.phone_picker(ui, "diagphone", &mut ani, 300.0);
                    ui.label("View by:");
                    egui::ComboBox::from_id_salt("diagtype")
                        .selected_text(if pick_type.is_empty() {
                            "All calls".to_string()
                        } else {
                            pick_type.clone()
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut pick_type, String::new(), "All calls");
                            for t in ["Local Coin", "Other Coin", "Zero Plus", "Dial Around"] {
                                ui.selectable_value(&mut pick_type, t.to_string(), t);
                            }
                        });
                });
                ui.label("Call List:");
                style::header(
                    ui,
                    &format!(
                        "{:<17} {:<12} {:<12} {:>9} {:>10}",
                        "Date", "Call Type", "User Dialed", "Time", "Total Coin"
                    ),
                );
                let rows: Vec<(usize, String)> = records
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        (
                            i,
                            format!(
                                "{:<17} {:<12} {:<12} {:>9} {:>10}",
                                r.date,
                                r.call_type,
                                r.dialed,
                                hms(r.duration_secs),
                                money(r.coin)
                            ),
                        )
                    })
                    .collect();
                let mut s = None;
                style::list_box(ui, "diaglist", &rows, &mut s, Vec2::new(588.0, 160.0));
                let secs: u32 = records.iter().map(|r| r.duration_secs).sum();
                let coin: f64 = records.iter().map(|r| r.coin).sum();
                ui.label(format!(
                    "{} call(s)   Total Time {}   Total Coin {}",
                    records.len(),
                    hms(secs),
                    money(coin)
                ));
                if let Some(c) = &last {
                    style::group(ui, &format!("Call counts at last contact ({})", c.time), |ui| {
                        egui::Grid::new("diagcounts")
                            .num_columns(4)
                            .spacing([18.0, 2.0])
                            .show(ui, |ui| {
                                for (i, (k, v)) in c.counts.rows().iter().enumerate() {
                                    ui.label(*k);
                                    ui.label(v.to_string());
                                    if i % 2 == 1 {
                                        ui.end_row();
                                    }
                                }
                                ui.label(egui::RichText::new("Total Calls").strong());
                                ui.label(egui::RichText::new(c.counts.total().to_string()).strong());
                                ui.end_row();
                            });
                        if !c.alarms.is_empty() {
                            ui.label(format!("Alarms: {}", c.alarms.join(", ")));
                        }
                    });
                } else {
                    ui.weak("No completed contact with this phone yet. Dial it with \"Retrieve Call Records\" to fill this in.");
                }
                ui.horizontal(|ui| {
                    if style::btn(ui, "Close") {
                        done = true;
                    }
                });
            },
        );
        self.svc.diag_ani = ani;
        self.svc.diag_type = pick_type;
        if done || close {
            self.w.diag = false;
        }
    }

    fn conhist_window(&mut self, ctx: &egui::Context) {
        if !self.w.conhist {
            return;
        }
        let mut ani = self.svc.conhist_ani.clone();
        let list: Vec<Contact> = self
            .db
            .contacts
            .iter()
            .filter(|c| Some(&c.ani) == ani.as_ref())
            .rev()
            .cloned()
            .collect();
        let rows: Vec<(u32, String)> = list
            .iter()
            .map(|c| {
                (
                    c.id,
                    format!(
                        "{:<17} {:<10} {:>6} {:>10} {:>10}",
                        c.time,
                        c.result,
                        c.counts.total(),
                        money(c.cashbox),
                        money(c.totalizer)
                    ),
                )
            })
            .collect();
        let mut sel = self.svc.conhist_sel;
        let mut done = false;
        let mut smdr = false;
        let close = style::child_window(
            ctx,
            "conhist",
            "Contact History",
            self.area,
            at(self.area, 0.15, 0.2),
            520.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Phone:");
                    self.phone_picker(ui, "chphone", &mut ani, 300.0);
                });
                style::header(
                    ui,
                    &format!(
                        "{:<17} {:<10} {:>6} {:>10} {:>10}",
                        "Contact Time", "Result", "Calls", "Cashbox", "Totalizer"
                    ),
                );
                style::list_box(ui, "chlist", &rows, &mut sel, Vec2::new(508.0, 150.0));
                if let Some(c) = sel.and_then(|id| list.iter().find(|c| c.id == id)) {
                    style::group(ui, "Call Counters", |ui| {
                        egui::Grid::new("chcounts")
                            .num_columns(4)
                            .spacing([18.0, 2.0])
                            .show(ui, |ui| {
                                for (i, (k, v)) in c.counts.rows().iter().enumerate() {
                                    ui.label(*k);
                                    ui.label(v.to_string());
                                    if i % 2 == 1 {
                                        ui.end_row();
                                    }
                                }
                            });
                        ui.label(format!("Commands: {}", c.commands.join(", ")));
                    });
                }
                ui.horizontal(|ui| {
                    if style::btn(ui, "SMDR") {
                        smdr = true;
                    }
                    if style::btn(ui, "Close") {
                        done = true;
                    }
                });
            },
        );
        self.svc.conhist_ani = ani.clone();
        self.svc.conhist_sel = sel;
        if smdr {
            self.svc.diag_ani = ani;
            self.w.diag = true;
        }
        if done || close {
            self.w.conhist = false;
        }
    }

    fn ram_window(&mut self, ctx: &egui::Context) {
        if !self.w.ram {
            return;
        }
        let mut ani = self.svc.ram_ani.clone();
        let phone = ani.as_ref().and_then(|a| self.db.phone(a)).cloned();
        let mut done = false;
        let mut fix = false;
        let mut read = false;
        let close = style::child_window(
            ctx,
            "ram",
            "Security Information (RAM Image)",
            self.area,
            at(self.area, 0.2, 0.18),
            520.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Phone:");
                    self.phone_picker(ui, "ramphone", &mut ani, 300.0);
                });
                if let Some(p) = &phone {
                    egui::Grid::new("ramgrid")
                        .num_columns(3)
                        .striped(true)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            ui.strong("Register");
                            ui.strong("Database Value");
                            ui.strong("RAM Image Value");
                            ui.end_row();
                            let rows = [
                                ("Register 400 -- Owner Bypass", &p.bypass, &p.ram_bypass),
                                ("Register 402 -- Phone ID", &p.phone_id, &p.ram_phone_id),
                                ("Register 403 -- Password", &p.password, &p.ram_password),
                                ("Register 404 -- NPA NXX XXXX", &p.ani, &p.ram_ani),
                            ];
                            for (k, a, b) in rows {
                                ui.label(k);
                                ui.label(a.as_str());
                                let differs = a != b;
                                ui.label(
                                    egui::RichText::new(if b.is_empty() { "(not read)" } else { b.as_str() }).color(
                                        if differs {
                                            Color32::from_rgb(180, 0, 0)
                                        } else {
                                            style::BLACK
                                        },
                                    ),
                                );
                                ui.end_row();
                            }
                        });
                    ui.small("Database values may be changed on the phone detail window. Values in red differ from the phone.");
                    if !p.last_contact.is_empty() {
                        ui.small(format!("RAM image last read {}", p.last_contact));
                    }
                    if !p.ram_block.is_empty() {
                        let bytes: Vec<u8> = p
                            .ram_block
                            .split_whitespace()
                            .filter_map(|h| u8::from_str_radix(h, 16).ok())
                            .collect();
                        ui.label(format!(
                            "Data block from the phone ({} bytes, read {}):",
                            bytes.len(),
                            p.ram_block_time
                        ));
                        egui::Frame::default()
                            .fill(style::WHITE)
                            .inner_margin(4.0)
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("ramdump")
                                    .max_height(160.0)
                                    .show(ui, |ui| {
                                        ui.set_width(490.0);
                                        for (i, row) in bytes.chunks(16).enumerate() {
                                            let hx: Vec<String> = row.iter().map(|b| format!("{b:02X}")).collect();
                                            let txt: String = row
                                                .iter()
                                                .map(|b| {
                                                    if b.is_ascii_graphic() || *b == b' ' {
                                                        *b as char
                                                    } else {
                                                        '.'
                                                    }
                                                })
                                                .collect();
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:04X}  {:<48} {}",
                                                    i * 16,
                                                    hx.join(" "),
                                                    txt
                                                ))
                                                .monospace()
                                                .size(11.5),
                                            );
                                        }
                                    });
                            });
                        ui.small(
                            "The fields of this block aren't mapped yet; it is kept as read so it can be compared.",
                        );
                    }
                } else {
                    ui.weak("Select a phone.");
                }
                ui.horizontal(|ui| {
                    if style::btn_enabled(ui, phone.is_some(), "Read RAM") {
                        read = true;
                    }
                    if style::btn_enabled(ui, phone.is_some(), "Set DB like RAM") {
                        fix = true;
                    }
                    if style::btn(ui, "Close") {
                        done = true;
                    }
                });
            },
        );
        self.svc.ram_ani = ani.clone();
        if let Some(a) = ani {
            if read {
                self.queue_call(
                    &a,
                    vec!["Get RAM Image".into(), "Get Software Versions".into()],
                    None,
                    "View RAM Image",
                );
                self.open_dial(Some(a.clone()));
            }
            if fix {
                if let Some(p) = self.db.phone_mut(&a) {
                    if !p.ram_password.is_empty() {
                        p.password = p.ram_password.clone();
                        p.bypass = p.ram_bypass.clone();
                        p.phone_id = p.ram_phone_id.clone();
                    }
                }
                self.message(format!("Database for {a} now matches its RAM image"));
                self.save();
            }
        }
        if done || close {
            self.w.ram = false;
        }
    }
}

pub fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n.saturating_sub(1)).collect::<String>() + "~"
    }
}

pub fn hms(secs: u32) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}
