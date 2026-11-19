//! Business administration: cost accounting journal, commissions, rate and tax templates, account codes.

use crate::app::{at, App};
use crate::details::dual_list;
use crate::model::*;
use crate::service::trunc;
use crate::style;
use eframe::egui::{self, Vec2};

impl App {
    pub fn business_windows(&mut self, ctx: &egui::Context) {
        self.journal_window(ctx);
        self.journal_edit_window(ctx);
        self.commission_window(ctx);
        self.tax_window(ctx);
        self.rate_window(ctx);
        self.accounts_window(ctx);
    }

    fn journal_window(&mut self, ctx: &egui::Context) {
        if !self.w.journal {
            return;
        }
        let from = self.biz.j_from.clone();
        let to = self.biz.j_to.clone();
        let mut list: Vec<&JournalEntry> = self
            .db
            .journal
            .iter()
            .filter(|j| from.is_empty() || j.date.as_str() >= from.as_str())
            .filter(|j| to.is_empty() || j.date.as_str() <= to.as_str())
            .collect();
        list.sort_by(|a, b| b.date.cmp(&a.date));
        let mut income = 0.0;
        let mut expense = 0.0;
        for j in &list {
            if self.db.account_kind(&j.account_code) == "Income" {
                income += j.amount;
            } else {
                expense += j.amount;
            }
        }
        let rows: Vec<(u32, String)> = list
            .iter()
            .map(|j| {
                let site = self
                    .db
                    .phone(&j.ani)
                    .map(|p| self.db.site_name(p.site_id))
                    .unwrap_or_default();
                (
                    j.id,
                    format!(
                        "{:<10} {:<13} {:<5} {:>9}  {:<20} {}",
                        j.date,
                        j.ani,
                        j.account_code,
                        money(j.amount),
                        trunc(&site, 20),
                        j.notes
                    ),
                )
            })
            .collect();
        let mut summary: Vec<(String, String, f64, usize)> = vec![];
        if self.biz.j_summary {
            for a in &self.db.account_codes {
                let items: Vec<&&JournalEntry> = list.iter().filter(|j| j.account_code == a.code).collect();
                if !items.is_empty() {
                    summary.push((
                        a.code.clone(),
                        a.description.clone(),
                        items.iter().map(|j| j.amount).sum(),
                        items.len(),
                    ));
                }
            }
        }
        let count = list.len();
        let mut sel = self.biz.j_sel;
        let mut newb = false;
        let mut edit = None;
        let mut done = false;
        let pos = at(self.area, 0.04, 0.12);
        let b = &mut self.biz;
        let close = style::child_window(ctx, "journal", "Cost Accounting Journal", self.area, pos, 640.0, |ui| {
            ui.horizontal(|ui| {
                ui.label("Dates from");
                ui.add(
                    egui::TextEdit::singleline(&mut b.j_from)
                        .desired_width(90.0)
                        .hint_text("YYYY-MM-DD"),
                );
                ui.label("to");
                ui.add(
                    egui::TextEdit::singleline(&mut b.j_to)
                        .desired_width(90.0)
                        .hint_text("YYYY-MM-DD"),
                );
            });
            style::header(
                ui,
                &format!(
                    "{:<10} {:<13} {:<5} {:>9}  {:<20} {}",
                    "Date", "Phone ANI", "Acct", "Amount", "Site Name", "Notes"
                ),
            );
            if let Some(d) = style::list_box(ui, "jlist", &rows, &mut sel, Vec2::new(628.0, 200.0)) {
                edit = Some(d);
            }
            ui.horizontal(|ui| {
                for (k, v) in [("Income", income), ("Expenses", expense), ("Net", income - expense)] {
                    ui.label(format!("{k}:"));
                    ui.label(egui::RichText::new(money(v)).strong());
                    ui.add_space(10.0);
                }
                ui.label(format!("Count: {count}"));
            });
            if b.j_summary {
                style::group(ui, "Summary by account code", |ui| {
                    egui::Grid::new("jsum").num_columns(4).striped(true).show(ui, |ui| {
                        for (c, d, amt, n) in &summary {
                            ui.label(c);
                            ui.label(d);
                            ui.label(money(*amt));
                            ui.label(format!("{n} entries"));
                            ui.end_row();
                        }
                    });
                });
            }
            ui.horizontal(|ui| {
                if style::btn(ui, "New") {
                    newb = true;
                }
                if style::btn_enabled(ui, sel.is_some(), "Edit") {
                    edit = sel;
                }
                if style::btn(ui, if b.j_summary { "Hide Summary" } else { "Summary" }) {
                    b.j_summary = !b.j_summary;
                }
                if style::btn(ui, "Close") {
                    done = true;
                }
            });
        });
        self.biz.j_sel = sel;
        if newb {
            self.biz.j_edit = Some(JournalEntry {
                id: 0,
                date: today(),
                ani: self.active_ani.clone().unwrap_or_default(),
                account_code: "100".into(),
                ..Default::default()
            });
        }
        if let Some(id) = edit {
            self.biz.j_edit = self.db.journal.iter().find(|j| j.id == id).cloned();
        }
        if done || close {
            self.w.journal = false;
        }
    }

    fn journal_edit_window(&mut self, ctx: &egui::Context) {
        let pos = at(self.area, 0.28, 0.2);
        let Some(ed) = self.biz.j_edit.as_mut() else { return };
        let phones: Vec<String> = self.db.phones.iter().map(|p| p.ani.clone()).collect();
        let codes: Vec<(String, String)> = self
            .db
            .account_codes
            .iter()
            .map(|a| (a.code.clone(), format!("{}  {}", a.code, a.description)))
            .collect();
        let techs: Vec<(u32, String)> = self.db.techs.iter().map(|t| (t.id, t.name.clone())).collect();
        let site = self
            .db
            .phone(&ed.ani)
            .map(|p| self.db.site_name(p.site_id))
            .unwrap_or_default();
        let mut ok = false;
        let mut cancel = false;
        let mut del = false;
        let title = if ed.id == 0 {
            "Cost Accounting Journal Entry"
        } else {
            "Cost Accounting Journal Entry Edit"
        };
        let close = style::child_window(ctx, "jedit", title, self.area, pos, 420.0, |ui| {
            egui::Grid::new("jgrid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    style::field(ui, "Date", &mut ed.date, 100.0);
                    ui.label("Phone ANI");
                    style::combo(ui, "jani", &mut ed.ani, &phones, 150.0);
                    ui.end_row();
                    ui.label("Site Name");
                    ui.label(&site);
                    ui.end_row();
                    ui.label("Acct Code");
                    let txt = codes
                        .iter()
                        .find(|(c, _)| *c == ed.account_code)
                        .map(|(_, t)| t.clone())
                        .unwrap_or_else(|| ed.account_code.clone());
                    let mut chosen = None;
                    egui::ComboBox::from_id_salt("jcode")
                        .width(250.0)
                        .selected_text(txt)
                        .show_ui(ui, |ui| {
                            for (c, t) in &codes {
                                if ui.selectable_label(*c == ed.account_code, t).clicked() {
                                    chosen = Some(c.clone());
                                }
                            }
                        });
                    if let Some(c) = chosen {
                        if ed.amount == 0.0 {
                            ed.amount = self
                                .db
                                .account_codes
                                .iter()
                                .find(|a| a.code == c)
                                .map(|a| a.amount)
                                .unwrap_or(0.0);
                        }
                        ed.account_code = c;
                    }
                    ui.end_row();
                    style::num_field(ui, "Amount    $", &mut ed.amount, 90.0);
                    ui.label("Technician");
                    style::combo_id(ui, "jtech", &mut ed.tech_id, &techs, 200.0);
                    ui.end_row();
                    ui.label("Notes");
                    ui.add(
                        egui::TextEdit::multiline(&mut ed.notes)
                            .desired_width(260.0)
                            .desired_rows(2),
                    );
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if style::btn(ui, "OK") {
                    ok = true;
                }
                if style::btn_enabled(ui, ed.id != 0, "Delete") {
                    del = true;
                }
                if style::btn(ui, "Cancel") {
                    cancel = true;
                }
            });
        });
        if ok {
            let mut j = self.biz.j_edit.take().unwrap();
            if j.id == 0 {
                j.id = self.db.new_id();
                self.db.journal.push(j);
            } else if let Some(x) = self.db.journal.iter_mut().find(|x| x.id == j.id) {
                *x = j;
            }
            self.message("Journal entry saved");
            self.save();
        } else if del {
            let j = self.biz.j_edit.take().unwrap();
            self.db.journal.retain(|x| x.id != j.id);
            self.message("Journal entry deleted");
            self.save();
        } else if cancel || close {
            self.biz.j_edit = None;
        }
    }

    fn commission_window(&mut self, ctx: &egui::Context) {
        if !self.w.commission {
            return;
        }
        let pos = at(self.area, 0.1, 0.08);
        let label = |id: &u32| {
            self.db
                .contract(Some(*id))
                .map(|c| c.description.clone())
                .unwrap_or_default()
        };
        let right: Vec<(u32, String)> = self.biz.comm_chosen.iter().map(|i| (*i, label(i))).collect();
        let left: Vec<(u32, String)> = self
            .db
            .contracts
            .iter()
            .filter(|c| !self.biz.comm_chosen.contains(&c.id))
            .map(|c| (c.id, c.description.clone()))
            .collect();
        let mut run = false;
        let mut done = false;
        let b = &mut self.biz;
        let close = style::child_window(
            ctx,
            "comm",
            "Commission Calculation Controls",
            self.area,
            pos,
            560.0,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Revenue dates from");
                    ui.add(
                        egui::TextEdit::singleline(&mut b.comm_from)
                            .desired_width(90.0)
                            .hint_text("YYYY-MM-DD"),
                    );
                    ui.label("to");
                    ui.add(
                        egui::TextEdit::singleline(&mut b.comm_to)
                            .desired_width(90.0)
                            .hint_text("YYYY-MM-DD"),
                    );
                });
                let (a, r, aa, ra) = dual_list(
                    ui,
                    "commpick",
                    "Contracts available to process",
                    &left,
                    "Contracts to process",
                    &right,
                    &mut b.comm_pick,
                    210.0,
                    110.0,
                );
                if let Some(a) = a {
                    b.comm_chosen.push(a);
                }
                if let Some(r) = r {
                    b.comm_chosen.retain(|x| *x != r);
                }
                if aa {
                    b.comm_chosen.extend(left.iter().map(|(k, _)| *k));
                }
                if ra {
                    b.comm_chosen.clear();
                }
                if !b.comm_result.is_empty() {
                    style::group(ui, "Commission Processing", |ui| {
                        egui::Grid::new("commres")
                            .num_columns(5)
                            .striped(true)
                            .spacing([14.0, 3.0])
                            .show(ui, |ui| {
                                for h in ["Contract Description", "Revenue", "Tot. Recs", "Rate", "Amount"] {
                                    ui.strong(h);
                                }
                                ui.end_row();
                                for (d, rev, n, pct, amt) in &b.comm_result {
                                    ui.label(d);
                                    ui.label(money(*rev));
                                    ui.label(n.to_string());
                                    ui.label(format!("{pct:.1} %"));
                                    ui.label(money(*amt));
                                    ui.end_row();
                                }
                                let tot: f64 = b.comm_result.iter().map(|r| r.4).sum();
                                ui.strong("Total");
                                ui.label("");
                                ui.label("");
                                ui.label("");
                                ui.strong(money(tot));
                                ui.end_row();
                            });
                    });
                }
                ui.horizontal(|ui| {
                    if style::btn_enabled(ui, !b.comm_chosen.is_empty(), "OK") {
                        run = true;
                    }
                    if style::btn(ui, "Close") {
                        done = true;
                    }
                });
                ui.small("Revenue is the income recorded in the cost accounting journal for phones at each contract's sites.");
            },
        );
        if run {
            let from = self.biz.comm_from.clone();
            let to = self.biz.comm_to.clone();
            let mut out = vec![];
            for cid in self.biz.comm_chosen.clone() {
                let Some(c) = self.db.contract(Some(cid)).cloned() else {
                    continue;
                };
                let anis: Vec<String> = self
                    .db
                    .phones
                    .iter()
                    .filter(|p| p.site_id.map(|s| c.site_ids.contains(&s)).unwrap_or(false))
                    .map(|p| p.ani.clone())
                    .collect();
                let entries: Vec<&JournalEntry> = self
                    .db
                    .journal
                    .iter()
                    .filter(|j| anis.contains(&j.ani))
                    .filter(|j| self.db.account_kind(&j.account_code) == "Income")
                    .filter(|j| from.is_empty() || j.date.as_str() >= from.as_str())
                    .filter(|j| to.is_empty() || j.date.as_str() <= to.as_str())
                    .collect();
                let rev: f64 = entries.iter().map(|j| j.amount).sum();
                out.push((
                    c.description.clone(),
                    rev,
                    entries.len() as u32,
                    c.commission_pct,
                    rev * c.commission_pct / 100.0,
                ));
            }
            self.biz.comm_result = out;
            self.message("Commission calculation complete");
        }
        if done || close {
            self.w.commission = false;
        }
    }

    fn tax_window(&mut self, ctx: &egui::Context) {
        if !self.w.tax_templates {
            return;
        }
        let pos = at(self.area, 0.3, 0.1);
        let rows: Vec<(String, String)> = self
            .db
            .tax_templates
            .iter()
            .map(|t| (t.name.clone(), format!("{:<22} {}", t.name, t.description)))
            .collect();
        let phone = self.active_ani.clone();
        let mut sel = self.biz.tax_sel.clone();
        let mut action = "";
        let close = style::child_window(ctx, "tax", "Tax Rate Templates", self.area, pos, 440.0, |ui| {
            ui.label(format!(
                "Phone To Assign Template: {}",
                phone.clone().unwrap_or_else(|| "(no active phone)".into())
            ));
            if style::list_box(ui, "taxlist", &rows, &mut sel, Vec2::new(428.0, 120.0)).is_some() {
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
                if style::btn(ui, "Cancel") {
                    action = "close";
                }
            });
        });
        self.biz.tax_sel = sel.clone();
        match action {
            "new" => {
                self.biz.tax_edit = Some((
                    TaxTemplate {
                        name: "New Template".into(),
                        ..Default::default()
                    },
                    String::new(),
                ))
            }
            "edit" => {
                if let Some(t) = sel
                    .as_ref()
                    .and_then(|n| self.db.tax_templates.iter().find(|t| &t.name == n))
                {
                    self.biz.tax_edit = Some((t.clone(), t.name.clone()));
                }
            }
            "delete" => {
                let n = sel.unwrap();
                self.db.tax_templates.retain(|t| t.name != n);
                self.biz.tax_sel = None;
                self.save();
            }
            "assign" => {
                let (a, n) = (phone.unwrap(), sel.unwrap());
                if let Some(p) = self.db.phone_mut(&a) {
                    p.tax_template = n.clone();
                    p.needs_update = true;
                }
                self.message(format!("Tax template {n} assigned to {a}; the phone needs a download"));
                self.save();
            }
            "close" => self.w.tax_templates = false,
            _ => {}
        }
        if close {
            self.w.tax_templates = false;
        }
        let pos2 = at(self.area, 0.36, 0.3);
        if let Some((t, _)) = self.biz.tax_edit.as_mut() {
            let mut ok = false;
            let mut cancel = false;
            let c = style::child_window(ctx, "taxedit", "Tax Rate Template", self.area, pos2, 330.0, |ui| {
                egui::Grid::new("taxg").num_columns(2).show(ui, |ui| {
                    style::field(ui, "Template Name:", &mut t.name, 180.0);
                    style::field(ui, "Description:", &mut t.description, 180.0);
                    style::num_field(ui, "Federal %", &mut t.federal, 80.0);
                    style::num_field(ui, "State %", &mut t.state, 80.0);
                    style::num_field(ui, "City %", &mut t.city, 80.0);
                    style::num_field(ui, "Local %", &mut t.local, 80.0);
                    style::num_field(ui, "Other %", &mut t.other, 80.0);
                });
                ui.label(format!(
                    "Total tax: {:.2} %",
                    t.federal + t.state + t.city + t.local + t.other
                ));
                ui.horizontal(|ui| {
                    ok = style::btn(ui, "OK");
                    cancel = style::btn(ui, "Cancel");
                });
            });
            if ok {
                let (t, orig) = self.biz.tax_edit.take().unwrap();
                if let Some(x) = self
                    .db
                    .tax_templates
                    .iter_mut()
                    .find(|x| x.name == orig && !orig.is_empty())
                {
                    *x = t.clone();
                } else {
                    self.db.tax_templates.push(t.clone());
                }
                if !orig.is_empty() && orig != t.name {
                    for p in self.db.phones.iter_mut().filter(|p| p.tax_template == orig) {
                        p.tax_template = t.name.clone();
                    }
                }
                self.save();
            } else if cancel || c {
                self.biz.tax_edit = None;
            }
        }
    }

    fn rate_window(&mut self, ctx: &egui::Context) {
        if !self.w.rate_templates {
            return;
        }
        let pos = at(self.area, 0.28, 0.06);
        let rows: Vec<(String, String)> = self
            .db
            .rate_templates
            .iter()
            .map(|t| (t.name.clone(), format!("{:<22} {}", t.name, t.description)))
            .collect();
        let phone = self.active_ani.clone();
        let mut sel = self.biz.rate_sel.clone();
        let mut action = "";
        let close = style::child_window(
            ctx,
            "rates",
            "Local/LD Charged Rate Templates",
            self.area,
            pos,
            440.0,
            |ui| {
                ui.label(format!(
                    "Phone To Assign Template: {}",
                    phone.clone().unwrap_or_else(|| "(no active phone)".into())
                ));
                if style::list_box(ui, "ratelist", &rows, &mut sel, Vec2::new(428.0, 120.0)).is_some() {
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
                    if style::btn(ui, "Cancel") {
                        action = "close";
                    }
                });
            },
        );
        self.biz.rate_sel = sel.clone();
        match action {
            "new" => {
                self.biz.rate_edit = Some((
                    RateTemplate {
                        name: "New Template".into(),
                        ..Default::default()
                    },
                    String::new(),
                ))
            }
            "edit" => {
                if let Some(t) = sel
                    .as_ref()
                    .and_then(|n| self.db.rate_templates.iter().find(|t| &t.name == n))
                {
                    self.biz.rate_edit = Some((t.clone(), t.name.clone()));
                }
            }
            "delete" => {
                let n = sel.unwrap();
                self.db.rate_templates.retain(|t| t.name != n);
                self.biz.rate_sel = None;
                self.save();
            }
            "assign" => {
                let (a, n) = (phone.unwrap(), sel.unwrap());
                if let Some(p) = self.db.phone_mut(&a) {
                    p.rate_template = n.clone();
                    p.needs_update = true;
                }
                self.message(format!("Rate template {n} assigned to {a}; the phone needs a download"));
                self.save();
            }
            "close" => self.w.rate_templates = false,
            _ => {}
        }
        if close {
            self.w.rate_templates = false;
        }
        let pos2 = at(self.area, 0.34, 0.28);
        if let Some((t, _)) = self.biz.rate_edit.as_mut() {
            let mut ok = false;
            let mut cancel = false;
            let c = style::child_window(ctx, "rateedit", "Charged Rate Template", self.area, pos2, 340.0, |ui| {
                egui::Grid::new("rateg").num_columns(2).show(ui, |ui| {
                    style::field(ui, "Template Name:", &mut t.name, 180.0);
                    style::field(ui, "Description:", &mut t.description, 180.0);
                    style::num_field(ui, "LEC Flat Rate Charge $", &mut t.lec_flat, 80.0);
                    style::num_field(ui, "LEC Rate Per Minute $", &mut t.lec_per_min, 80.0);
                    style::num_field(ui, "IXC Flat Rate Charge $", &mut t.ixc_flat, 80.0);
                    style::num_field(ui, "IXC Rate Per Minute $", &mut t.ixc_per_min, 80.0);
                });
                ui.horizontal(|ui| {
                    ok = style::btn(ui, "OK");
                    cancel = style::btn(ui, "Cancel");
                });
            });
            if ok {
                let (t, orig) = self.biz.rate_edit.take().unwrap();
                if let Some(x) = self
                    .db
                    .rate_templates
                    .iter_mut()
                    .find(|x| x.name == orig && !orig.is_empty())
                {
                    *x = t.clone();
                } else {
                    self.db.rate_templates.push(t.clone());
                }
                if !orig.is_empty() && orig != t.name {
                    for p in self.db.phones.iter_mut().filter(|p| p.rate_template == orig) {
                        p.rate_template = t.name.clone();
                    }
                }
                self.save();
            } else if cancel || c {
                self.biz.rate_edit = None;
            }
        }
    }

    fn accounts_window(&mut self, ctx: &egui::Context) {
        if !self.w.accounts {
            return;
        }
        let pos = at(self.area, 0.25, 0.12);
        let rows: Vec<(String, String)> = self
            .db
            .account_codes
            .iter()
            .map(|a| {
                (
                    a.code.clone(),
                    format!("{:<6} {:<26} {:<8} {}", a.code, a.description, a.kind, money(a.amount)),
                )
            })
            .collect();
        let mut sel = self.biz.acct_sel.clone();
        let mut action = "";
        let close = style::child_window(ctx, "accts", "Account Codes", self.area, pos, 440.0, |ui| {
            style::header(
                ui,
                &format!("{:<6} {:<26} {:<8} {}", "Code", "Description", "Type", "Amount"),
            );
            if style::list_box(ui, "acctlist", &rows, &mut sel, Vec2::new(428.0, 150.0)).is_some() {
                action = "edit";
            }
            ui.horizontal(|ui| {
                if style::btn(ui, "New Code") {
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
        self.biz.acct_sel = sel.clone();
        match action {
            "new" => {
                self.biz.acct_edit = Some((
                    AccountCode {
                        kind: "Expense".into(),
                        ..Default::default()
                    },
                    String::new(),
                ))
            }
            "edit" => {
                if let Some(a) = sel
                    .as_ref()
                    .and_then(|c| self.db.account_codes.iter().find(|a| &a.code == c))
                {
                    self.biz.acct_edit = Some((a.clone(), a.code.clone()));
                }
            }
            "delete" => {
                let c = sel.unwrap();
                self.db.account_codes.retain(|a| a.code != c);
                self.biz.acct_sel = None;
                self.save();
            }
            "close" => self.w.accounts = false,
            _ => {}
        }
        if close {
            self.w.accounts = false;
        }
        let pos2 = at(self.area, 0.32, 0.3);
        if let Some((a, _)) = self.biz.acct_edit.as_mut() {
            let mut ok = false;
            let mut cancel = false;
            let c = style::child_window(ctx, "acctedit", "New Account Code", self.area, pos2, 320.0, |ui| {
                egui::Grid::new("acctg").num_columns(2).show(ui, |ui| {
                    style::field(ui, "Account Code:", &mut a.code, 80.0);
                    style::field(ui, "Description:", &mut a.description, 200.0);
                    style::num_field(ui, "Amount:", &mut a.amount, 80.0);
                    ui.label("Type:");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut a.kind, "Income".to_string(), "Income");
                        ui.radio_value(&mut a.kind, "Expense".to_string(), "Expense");
                    });
                    ui.end_row();
                });
                ui.horizontal(|ui| {
                    ok = style::btn(ui, "OK");
                    cancel = style::btn(ui, "Cancel");
                });
            });
            if ok {
                let (a, orig) = self.biz.acct_edit.take().unwrap();
                if !a.code.trim().is_empty() {
                    if let Some(x) = self
                        .db
                        .account_codes
                        .iter_mut()
                        .find(|x| x.code == orig && !orig.is_empty())
                    {
                        *x = a;
                    } else if !self.db.account_codes.iter().any(|x| x.code == a.code) {
                        self.db.account_codes.push(a);
                    }
                    self.db.account_codes.sort_by(|x, y| x.code.cmp(&y.code));
                    self.save();
                }
            } else if cancel || c {
                self.biz.acct_edit = None;
            }
        }
    }
}
