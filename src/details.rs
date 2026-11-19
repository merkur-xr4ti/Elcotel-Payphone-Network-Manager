//! Detail windows for customers, contracts, sites, phones, routes and technicians.

use crate::app::App;
use crate::model::*;
use crate::state::*;
use crate::style;
use eframe::egui::{self, Vec2};

/// "Add >>" / "<< Remove" between two lists. Returns (added, removed, add_all, remove_all).
#[allow(clippy::too_many_arguments)]
pub fn dual_list<K: Clone + PartialEq>(
    ui: &mut egui::Ui,
    id: &str,
    left_title: &str,
    left: &[(K, String)],
    right_title: &str,
    right: &[(K, String)],
    pick: &mut Picker<K>,
    width: f32,
    height: f32,
) -> (Option<K>, Option<K>, bool, bool) {
    let mut add = None;
    let mut rem = None;
    let mut add_all = false;
    let mut rem_all = false;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.label(left_title);
            if let Some(k) = style::list_box(
                ui,
                &format!("{id}-l"),
                left,
                &mut pick.left_sel,
                Vec2::new(width, height),
            ) {
                add = Some(k);
            }
        });
        ui.vertical(|ui| {
            ui.add_space(height / 2.0 - 40.0);
            if style::btn_enabled(ui, pick.left_sel.is_some(), "Add >>") {
                add = pick.left_sel.clone();
            }
            if style::btn_enabled(ui, pick.right_sel.is_some(), "<< Remove") {
                rem = pick.right_sel.clone();
            }
            if style::btn_enabled(ui, !left.is_empty(), "Add All") {
                add_all = true;
            }
            if style::btn_enabled(ui, !right.is_empty(), "Remove All") {
                rem_all = true;
            }
        });
        ui.vertical(|ui| {
            ui.label(right_title);
            if let Some(k) = style::list_box(
                ui,
                &format!("{id}-r"),
                right,
                &mut pick.right_sel,
                Vec2::new(width, height),
            ) {
                rem = Some(k);
            }
        });
    });
    if add.is_some() {
        pick.left_sel = None;
    }
    if rem.is_some() {
        pick.right_sel = None;
    }
    (add, rem, add_all, rem_all)
}

fn ok_cancel_delete(ui: &mut egui::Ui, can_delete: bool) -> (bool, bool, bool, bool) {
    let mut ok = false;
    let mut cancel = false;
    let mut del = false;
    let mut new = false;
    ui.horizontal(|ui| {
        if style::btn(ui, "New") {
            new = true;
        }
        if style::btn_enabled(ui, can_delete, "Delete") {
            del = true;
        }
        if style::btn(ui, "OK") {
            ok = true;
        }
        if style::btn(ui, "Cancel") {
            cancel = true;
        }
    });
    (ok, cancel, del, new)
}

impl App {
    pub fn open_customer(&mut self, id: Option<u32>) {
        self.ed.customer = Some(self.db.customer(id).cloned().unwrap_or_default());
    }
    pub fn open_contract(&mut self, id: Option<u32>) {
        self.ed.contract = Some(self.db.contract(id).cloned().unwrap_or_else(|| Contract {
            customer_id: self.nav.customer,
            start: today(),
            notify_days: 30,
            ..Default::default()
        }));
        self.ed.contract_sites = false;
    }
    pub fn open_site(&mut self, id: Option<u32>) {
        let mut s = self.db.site(id).cloned().unwrap_or_else(|| Site {
            time_zone: "Central".into(),
            route_id: self.nav.route,
            ..Default::default()
        });
        if s.name.is_empty() && s.id == 0 {
            s.state = "IL".into();
        }
        self.ed.site = Some(s);
        self.ed.site_phones = None;
    }
    pub fn open_phone(&mut self, ani: &str) {
        if let Some(p) = self.db.phone(ani).cloned() {
            self.ed.phone_orig = p.ani.clone();
            self.ed.phone = Some(p);
            self.ed.error.clear();
            self.set_active(ani);
        }
    }
    pub fn new_phone(&mut self) {
        let s = &self.db.settings;
        self.ed.phone_orig.clear();
        self.ed.error.clear();
        self.ed.phone = Some(Phone {
            model: "Series 5".into(),
            site_id: self.nav.site,
            password: s.default_password.clone(),
            bypass: s.default_bypass.clone(),
            phone_type: "Coin".into(),
            coll_interval: 14,
            install_date: today(),
            tax_template: self
                .db
                .tax_templates
                .first()
                .map(|t| t.name.clone())
                .unwrap_or_default(),
            rate_template: self
                .db
                .rate_templates
                .first()
                .map(|t| t.name.clone())
                .unwrap_or_default(),
            needs_update: true,
            ..Default::default()
        });
    }
    pub fn open_route(&mut self, id: Option<u32>) {
        self.ed.route = Some(self.db.route(id).cloned().unwrap_or_default());
    }
    pub fn open_tech(&mut self, id: Option<u32>) {
        self.ed.tech = Some(self.db.tech(id).cloned().unwrap_or_default());
    }

    pub fn detail_windows(&mut self, ctx: &egui::Context) {
        self.customer_window(ctx);
        self.contract_window(ctx);
        self.site_window(ctx);
        self.phone_window(ctx);
        self.route_window(ctx);
        self.tech_window(ctx);
        self.download_prompt(ctx);
    }

    fn customer_window(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.ed.customer.as_mut() else { return };
        let title = if ed.id == 0 {
            "Customer Detail - New".to_string()
        } else {
            "Customer Detail".to_string()
        };
        let mut act = (false, false, false, false);
        let mut view_contract = None;
        let contracts: Vec<(u32, String)> = self
            .db
            .contracts
            .iter()
            .filter(|c| ed.id != 0 && c.customer_id == Some(ed.id))
            .map(|c| (c.id, format!("{}  ({} - {})", c.description, c.start, c.end)))
            .collect();
        let close = style::child_window(
            ctx,
            "cust",
            &title,
            self.area,
            self.area.min + Vec2::new(120.0, 40.0),
            430.0,
            |ui| {
                egui::Grid::new("custgrid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Customer ID");
                        ui.label(if ed.id == 0 { "(new)".into() } else { ed.id.to_string() });
                        ui.end_row();
                        style::field(ui, "Company", &mut ed.company, 300.0);
                        style::field(ui, "Address", &mut ed.address, 300.0);
                        style::field(ui, "City", &mut ed.city, 200.0);
                        ui.label("State / Zip");
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut ed.state).desired_width(40.0));
                            ui.label("Zip");
                            ui.add(egui::TextEdit::singleline(&mut ed.zip).desired_width(80.0));
                        });
                        ui.end_row();
                        style::field(ui, "Contact", &mut ed.contact, 200.0);
                        style::field(ui, "Phone", &mut ed.phone, 140.0);
                        style::field(ui, "FAX", &mut ed.fax, 140.0);
                        style::field(ui, "Tax ID", &mut ed.tax_id, 140.0);
                        ui.label("Notes");
                        ui.add(
                            egui::TextEdit::multiline(&mut ed.notes)
                                .desired_width(300.0)
                                .desired_rows(3),
                        );
                        ui.end_row();
                    });
                ui.label("Contracts for this customer:");
                let mut sel = None;
                if let Some(d) = style::list_box(ui, "custcontracts", &contracts, &mut sel, Vec2::new(418.0, 64.0)) {
                    view_contract = Some(d);
                }
                act = ok_cancel_delete(ui, ed.id != 0);
            },
        );
        let (ok, cancel, del, new) = act;
        if ok {
            let mut c = self.ed.customer.take().unwrap();
            if c.company.trim().is_empty() {
                c.company = "New Customer".into();
            }
            if c.id == 0 {
                c.id = self.db.new_id();
                self.db.customers.push(c.clone());
            } else if let Some(x) = self.db.customers.iter_mut().find(|x| x.id == c.id) {
                *x = c.clone();
            }
            self.nav.customer = Some(c.id);
            self.message(format!("Customer {} saved", c.company));
            self.save();
        } else if del {
            let c = self.ed.customer.take().unwrap();
            self.db.customers.retain(|x| x.id != c.id);
            for ct in self.db.contracts.iter_mut().filter(|ct| ct.customer_id == Some(c.id)) {
                ct.customer_id = None;
            }
            if self.nav.customer == Some(c.id) {
                self.nav.customer = None;
            }
            self.message(format!("Customer {} deleted", c.company));
            self.save();
        } else if new {
            self.ed.customer = Some(Customer::default());
        } else if cancel || close {
            self.ed.customer = None;
        }
        if let Some(id) = view_contract {
            self.open_contract(Some(id));
        }
    }

    fn contract_window(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.ed.contract.as_mut() else { return };
        let customers: Vec<(u32, String)> = self.db.customers.iter().map(|c| (c.id, c.company.clone())).collect();
        let mut act = (false, false, false, false);
        let mut toggle_sites = false;
        let mut open_comm = false;
        let assigned: Vec<u32> = self
            .db
            .contracts
            .iter()
            .filter(|c| c.id != ed.id)
            .flat_map(|c| c.site_ids.clone())
            .collect();
        let site_label = |id: &u32| self.db.site(Some(*id)).map(|s| s.name.clone()).unwrap_or_default();
        let right: Vec<(u32, String)> = ed.site_ids.iter().map(|i| (*i, site_label(i))).collect();
        let left: Vec<(u32, String)> = self
            .db
            .sites
            .iter()
            .filter(|s| !ed.site_ids.contains(&s.id) && !assigned.contains(&s.id))
            .map(|s| (s.id, s.name.clone()))
            .collect();
        let show_sites = self.ed.contract_sites;
        let pick = &mut self.ed.contract_pick;
        let title = if ed.id == 0 {
            "Contract Detail - New"
        } else {
            "Contract Detail"
        };
        let close = style::child_window(
            ctx,
            "contract",
            title,
            self.area,
            self.area.min + Vec2::new(160.0, 60.0),
            470.0,
            |ui| {
                egui::Grid::new("ctgrid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Contract ID");
                        ui.label(if ed.id == 0 { "(new)".into() } else { ed.id.to_string() });
                        ui.end_row();
                        ui.label("Contract for Cust.");
                        style::combo_id(ui, "ctcust", &mut ed.customer_id, &customers, 300.0);
                        ui.end_row();
                        style::field(ui, "Description", &mut ed.description, 300.0);
                        style::field(ui, "Start Date", &mut ed.start, 110.0);
                        style::field(ui, "End Date", &mut ed.end, 110.0);
                        ui.label("Notify");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut ed.notify_days).range(0..=365));
                            ui.label("days prior to end");
                        });
                        ui.end_row();
                        ui.label("Commission %");
                        ui.add(
                            egui::DragValue::new(&mut ed.commission_pct)
                                .range(0.0..=100.0)
                                .speed(0.25)
                                .suffix(" %"),
                        );
                        ui.end_row();
                    });
                if !ed.end.is_empty() && ed.end.as_str() < today().as_str() {
                    ui.colored_label(egui::Color32::from_rgb(160, 0, 0), "This contract has ended.");
                }
                if show_sites {
                    let (a, r, aa, ra) = dual_list(
                        ui,
                        "ctsites",
                        "All Unassigned Sites",
                        &left,
                        "Sites Assigned",
                        &right,
                        pick,
                        190.0,
                        130.0,
                    );
                    if let Some(a) = a {
                        ed.site_ids.push(a);
                    }
                    if let Some(r) = r {
                        ed.site_ids.retain(|x| *x != r);
                    }
                    if aa {
                        ed.site_ids.extend(left.iter().map(|(k, _)| *k));
                    }
                    if ra {
                        ed.site_ids.clear();
                    }
                } else {
                    ui.label(format!("Sites covered under contract ({}):", ed.site_ids.len()));
                    let mut sel = None;
                    style::list_box(ui, "ctsitelist", &right, &mut sel, Vec2::new(458.0, 70.0));
                }
                ui.horizontal(|ui| {
                    if style::btn(ui, if show_sites { "Done Sites" } else { "Change Sites" }) {
                        toggle_sites = true;
                    }
                    if style::btn(ui, "Commissions...") {
                        open_comm = true;
                    }
                });
                act = ok_cancel_delete(ui, ed.id != 0);
            },
        );
        if toggle_sites {
            self.ed.contract_sites = !self.ed.contract_sites;
        }
        if open_comm {
            if let Some(c) = &self.ed.contract {
                if c.id != 0 && !self.biz.comm_chosen.contains(&c.id) {
                    self.biz.comm_chosen.push(c.id);
                }
            }
            self.w.commission = true;
        }
        let (ok, cancel, del, new) = act;
        if ok {
            let mut c = self.ed.contract.take().unwrap();
            if c.description.trim().is_empty() {
                c.description = "New contract".into();
            }
            if c.id == 0 {
                c.id = self.db.new_id();
                self.db.contracts.push(c.clone());
            } else if let Some(x) = self.db.contracts.iter_mut().find(|x| x.id == c.id) {
                *x = c.clone();
            }
            self.nav.contract = Some(c.id);
            self.message(format!("Contract {} saved", c.description));
            self.save();
        } else if del {
            let c = self.ed.contract.take().unwrap();
            self.db.contracts.retain(|x| x.id != c.id);
            if self.nav.contract == Some(c.id) {
                self.nav.contract = None;
            }
            self.message("Contract deleted");
            self.save();
        } else if new {
            self.open_contract(None);
        } else if cancel || close {
            self.ed.contract = None;
        }
    }

    fn site_window(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.ed.site.as_mut() else { return };
        let routes: Vec<(u32, String)> = self.db.routes.iter().map(|r| (r.id, r.description.clone())).collect();
        let contract = self
            .db
            .contract_of_site(ed.id)
            .map(|c| c.description.clone())
            .unwrap_or_else(|| "(none)".into());
        let mut act = (false, false, false, false);
        let mut new_phone = false;
        let mut assign = false;
        let mut open_phone = None;
        let phones_here: Vec<String> = match &self.ed.site_phones {
            Some(list) => list.clone(),
            None => self
                .db
                .phones
                .iter()
                .filter(|p| ed.id != 0 && p.site_id == Some(ed.id))
                .map(|p| p.ani.clone())
                .collect(),
        };
        let label = |a: &String| (a.clone(), self.db.phone_label(a));
        let right: Vec<(String, String)> = phones_here.iter().map(label).collect();
        let left: Vec<(String, String)> = self
            .db
            .phones
            .iter()
            .filter(|p| p.site_id.is_none() && !phones_here.contains(&p.ani))
            .map(|p| (p.ani.clone(), self.db.phone_label(&p.ani)))
            .collect();
        let assigning = self.ed.site_phones.is_some();
        let pick = &mut self.ed.site_pick;
        let mut new_list: Option<Vec<String>> = None;
        let title = if ed.id == 0 { "Site Detail - New" } else { "Site Detail" };
        let close = style::child_window(
            ctx,
            "site",
            title,
            self.area,
            self.area.min + Vec2::new(90.0, 30.0),
            560.0,
            |ui| {
                ui.horizontal_top(|ui| {
                    egui::Grid::new("sitegrid")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Site ID");
                            ui.label(if ed.id == 0 { "(new)".into() } else { ed.id.to_string() });
                            ui.end_row();
                            style::field(ui, "Site Name", &mut ed.name, 220.0);
                            style::field(ui, "Address", &mut ed.address, 220.0);
                            style::field(ui, "City", &mut ed.city, 160.0);
                            ui.label("State / Zip");
                            ui.horizontal(|ui| {
                                ui.add(egui::TextEdit::singleline(&mut ed.state).desired_width(36.0));
                                ui.add(egui::TextEdit::singleline(&mut ed.zip).desired_width(70.0));
                            });
                            ui.end_row();
                            style::field(ui, "Contact", &mut ed.contact, 160.0);
                            style::field(ui, "Phone", &mut ed.phone, 120.0);
                            style::field(ui, "Map Coord", &mut ed.map_coord, 80.0);
                        });
                    egui::Grid::new("sitegrid2")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Time Zone");
                            let tz: Vec<String> = TIME_ZONES.iter().map(|s| s.to_string()).collect();
                            style::combo(ui, "sitetz", &mut ed.time_zone, &tz, 110.0);
                            ui.end_row();
                            ui.label("Route");
                            style::combo_id(ui, "siteroute", &mut ed.route_id, &routes, 110.0);
                            ui.end_row();
                            ui.label("Contract");
                            ui.label(&contract);
                            ui.end_row();
                            style::field(ui, "Dial Prefix", &mut ed.dial_prefix, 60.0);
                            style::field(ui, "Pattern", &mut ed.dial_pattern, 110.0);
                            style::field(ui, "User Var 1", &mut ed.user_var[0], 110.0);
                            style::field(ui, "User Var 2", &mut ed.user_var[1], 110.0);
                            style::field(ui, "User Var 3", &mut ed.user_var[2], 110.0);
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Notes");
                    ui.add(
                        egui::TextEdit::multiline(&mut ed.notes)
                            .desired_width(490.0)
                            .desired_rows(2),
                    );
                });
                if assigning {
                    let (a, r, aa, ra) = dual_list(
                        ui,
                        "sitephones",
                        "Unassigned Phones",
                        &left,
                        "Phones At Site",
                        &right,
                        pick,
                        230.0,
                        110.0,
                    );
                    let mut list = phones_here.clone();
                    if let Some(a) = a {
                        list.push(a);
                    }
                    if let Some(r) = r {
                        list.retain(|x| *x != r);
                    }
                    if aa {
                        list.extend(left.iter().map(|(k, _)| k.clone()));
                    }
                    if ra {
                        list.clear();
                    }
                    if list != phones_here {
                        new_list = Some(list);
                    }
                } else {
                    ui.label(format!("Phones at this site ({}):", right.len()));
                    let mut sel = None;
                    if let Some(d) = style::list_box(ui, "sitephonelist", &right, &mut sel, Vec2::new(548.0, 70.0)) {
                        open_phone = Some(d);
                    }
                }
                ui.horizontal(|ui| {
                    if style::btn_enabled(ui, ed.id != 0, "New Phone") {
                        new_phone = true;
                    }
                    if style::btn(ui, if assigning { "Done" } else { "Assign Phones" }) {
                        assign = true;
                    }
                });
                act = ok_cancel_delete(ui, ed.id != 0);
            },
        );
        if let Some(l) = new_list {
            self.ed.site_phones = Some(l);
        }
        if assign {
            if self.ed.site_phones.is_some() {
                // keep the pending list until OK
            } else {
                self.ed.site_phones = Some(phones_here.clone());
            }
        }
        let (ok, cancel, del, new) = act;
        if ok {
            let mut s = self.ed.site.take().unwrap();
            if s.name.trim().is_empty() {
                s.name = "New Site".into();
            }
            if s.id == 0 {
                s.id = self.db.new_id();
                self.db.sites.push(s.clone());
            } else if let Some(x) = self.db.sites.iter_mut().find(|x| x.id == s.id) {
                *x = s.clone();
            }
            // keep the route's site list in step
            for r in self.db.routes.iter_mut() {
                let on = r.site_ids.contains(&s.id);
                if Some(r.id) == s.route_id && !on {
                    r.site_ids.push(s.id);
                } else if Some(r.id) != s.route_id && on {
                    r.site_ids.retain(|x| *x != s.id);
                }
            }
            if let Some(list) = self.ed.site_phones.take() {
                for p in self.db.phones.iter_mut() {
                    if list.contains(&p.ani) {
                        p.site_id = Some(s.id);
                    } else if p.site_id == Some(s.id) {
                        p.site_id = None;
                    }
                }
            }
            self.nav.site = Some(s.id);
            self.message(format!("Site {} saved", s.name));
            self.save();
        } else if del {
            let s = self.ed.site.take().unwrap();
            self.db.sites.retain(|x| x.id != s.id);
            for p in self.db.phones.iter_mut().filter(|p| p.site_id == Some(s.id)) {
                p.site_id = None;
            }
            for r in self.db.routes.iter_mut() {
                r.site_ids.retain(|x| *x != s.id);
            }
            for c in self.db.contracts.iter_mut() {
                c.site_ids.retain(|x| *x != s.id);
            }
            if self.nav.site == Some(s.id) {
                self.nav.site = None;
            }
            self.message("Site deleted; its phones are now unassigned");
            self.save();
            self.ed.site_phones = None;
        } else if new {
            self.open_site(None);
        } else if cancel || close {
            self.ed.site = None;
            self.ed.site_phones = None;
        } else if new_phone {
            let sid = self.ed.site.as_ref().map(|s| s.id);
            self.nav.site = sid;
            self.new_phone();
        }
        if let Some(a) = open_phone {
            self.open_phone(&a);
        }
    }

    fn phone_window(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.ed.phone.as_mut() else { return };
        let sites: Vec<(u32, String)> = self.db.sites.iter().map(|s| (s.id, s.name.clone())).collect();
        let taxes: Vec<String> = self.db.tax_templates.iter().map(|t| t.name.clone()).collect();
        let rates: Vec<String> = self.db.rate_templates.iter().map(|t| t.name.clone()).collect();
        let models: Vec<String> = MODELS.iter().map(|s| s.to_string()).collect();
        let types: Vec<String> = ["Coin", "Coinless", "Inmate", "Card only"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let files_by_kind: Vec<(String, Vec<String>)> = FILE_KINDS
            .iter()
            .map(|k| {
                (
                    k.0.to_string(),
                    self.db
                        .config_files
                        .iter()
                        .filter(|f| f.kind == k.0)
                        .map(|f| f.name.clone())
                        .collect(),
                )
            })
            .collect();
        let is_new = self.ed.phone_orig.is_empty();
        let mut act = (false, false, false, false);
        let mut poll_btn = false;
        let mut cmd_btn = false;
        let mut notes_btn = false;
        let error = self.ed.error.clone();
        let original = self.db.phone(&self.ed.phone_orig).cloned();
        let lists: Vec<String> = self
            .db
            .poll_lists
            .iter()
            .filter(|l| !is_new && l.phones.contains(&self.ed.phone_orig))
            .map(|l| l.name.clone())
            .collect();
        let title = if is_new {
            "Phone Detail - New".to_string()
        } else {
            format!("Phone Detail - {}", ed.ani)
        };
        let close = style::child_window(
            ctx,
            "phone",
            &title,
            self.area,
            self.area.min + Vec2::new(60.0, 20.0),
            640.0,
            |ui| {
                ui.horizontal_top(|ui| {
                egui::Grid::new("ph1").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                    style::field(ui, "Phone ANI", &mut ed.ani, 120.0);
                    ui.label("Phone Model");
                    style::combo(ui, "phmodel", &mut ed.model, &models, 120.0);
                    ui.end_row();
                    ui.label("Site Name");
                    style::combo_id(ui, "phsite", &mut ed.site_id, &sites, 180.0);
                    ui.end_row();
                    style::field(ui, "Location", &mut ed.location, 180.0);
                    style::field(ui, "Description", &mut ed.description, 180.0);
                    ui.label("Phone Type");
                    style::combo(ui, "phtype", &mut ed.phone_type, &types, 120.0);
                    ui.end_row();
                    style::field(ui, "Group", &mut ed.group, 60.0);
                    ui.label("Coll. Interval");
                    ui.add(egui::DragValue::new(&mut ed.coll_interval).range(1..=90).suffix(" days"));
                    ui.end_row();
                    style::field(ui, "Install Date", &mut ed.install_date, 100.0);
                    ui.label("Dial String").on_hover_text("What the modem dials: a phone number or a sip: address. Blank = the site's prefix and the ANI.");
                    ui.add(egui::TextEdit::singleline(&mut ed.dial_target).desired_width(180.0).hint_text("site prefix + ANI"));
                    ui.end_row();
                });
                egui::Grid::new("ph2").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                    style::field(ui, "Serial No.", &mut ed.serial, 110.0);
                    style::field(ui, "ROM Ver.", &mut ed.rom_ver, 80.0);
                    style::field(ui, "Pgm Ver.", &mut ed.pgm_ver, 80.0);
                    style::field(ui, "User Def 1", &mut ed.user_def[0], 110.0);
                    style::field(ui, "User Def 2", &mut ed.user_def[1], 110.0);
                    style::field(ui, "User Def 3", &mut ed.user_def[2], 110.0);
                    ui.label("Totalizer");
                    ui.label(money(ed.totalizer));
                    ui.end_row();
                    ui.label("Cashbox");
                    ui.label(format!("{}   (last {})", money(ed.cashbox), money(ed.last_collected)));
                    ui.end_row();
                    ui.label("Last contact");
                    ui.label(if ed.last_contact.is_empty() { "never".to_string() } else { ed.last_contact.clone() });
                    ui.end_row();
                });
            });
                ui.horizontal_top(|ui| {
                    style::group(ui, "Security", |ui| {
                        egui::Grid::new("phsec").num_columns(2).show(ui, |ui| {
                            style::field(ui, "Phone ID", &mut ed.phone_id, 90.0);
                            style::field(ui, "Password", &mut ed.password, 90.0);
                            style::field(ui, "Bypass", &mut ed.bypass, 90.0);
                        });
                    });
                    style::group(ui, "Cash Box", |ui| {
                        egui::Grid::new("phcb").num_columns(2).show(ui, |ui| {
                            style::field(ui, "Serial", &mut ed.cashbox_serial, 80.0);
                            style::field(ui, "Seal", &mut ed.cashbox_seal, 80.0);
                        });
                        ui.label("Keys");
                        egui::Grid::new("phkeys").num_columns(2).show(ui, |ui| {
                            style::field(ui, "Upper", &mut ed.upper_key, 50.0);
                            style::field(ui, "Lower", &mut ed.lower_key, 50.0);
                        });
                    });
                    style::group(ui, "Rates", |ui| {
                        ui.label("Call Type Tax Rates");
                        style::combo(ui, "phtax", &mut ed.tax_template, &taxes, 150.0);
                        ui.label("LEC/LD Rates");
                        style::combo(ui, "phrate", &mut ed.rate_template, &rates, 150.0);
                        if ed.needs_update {
                            ui.colored_label(egui::Color32::from_rgb(160, 0, 0), "Phone needs a download");
                        }
                    });
                });
                style::group(ui, "Operational Files (file assigned / last sent to the phone)", |ui| {
                    egui::Grid::new("phfiles")
                        .num_columns(6)
                        .spacing([6.0, 3.0])
                        .show(ui, |ui| {
                            for (i, (kind, names)) in files_by_kind.iter().enumerate() {
                                ui.label(kind.as_str());
                                let mut cur = ed.files.get(kind).cloned().unwrap_or_default();
                                let before = cur.clone();
                                style::combo(ui, &format!("phfile{i}"), &mut cur, names, 90.0);
                                if cur != before {
                                    if cur.is_empty() {
                                        ed.files.remove(kind);
                                    } else {
                                        ed.files.insert(kind.clone(), cur);
                                    }
                                    ed.needs_update = true;
                                }
                                let date = ed.file_dates.get(kind).cloned().unwrap_or_else(|| "never".into());
                                ui.label(egui::RichText::new(date).small());
                                if i % 2 == 1 {
                                    ui.end_row();
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Notes");
                    ui.add(
                        egui::TextEdit::multiline(&mut ed.notes)
                            .desired_width(560.0)
                            .desired_rows(2),
                    );
                });
                if !lists.is_empty() {
                    ui.label(format!("On poll lists: {}", lists.join(", ")));
                }
                if !error.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(180, 0, 0), &error);
                }
                ui.horizontal(|ui| {
                    if style::btn_enabled(ui, !is_new, "Poll List...") {
                        poll_btn = true;
                    }
                    if style::btn_enabled(ui, !is_new, "Cmd List...") {
                        cmd_btn = true;
                    }
                    if style::btn_enabled(ui, !is_new, "History...") {
                        notes_btn = true;
                    }
                    ui.add_space(12.0);
                    act = ok_cancel_delete(ui, !is_new);
                });
            },
        );
        let (ok, cancel, del, new) = act;
        if poll_btn {
            self.ed.phone_polls = true;
        }
        if cmd_btn {
            let a = self.ed.phone_orig.clone();
            self.open_dial(Some(a));
        }
        if notes_btn {
            self.svc.conhist_ani = Some(self.ed.phone_orig.clone());
            self.w.conhist = true;
        }
        if ok {
            let mut p = self.ed.phone.clone().unwrap();
            p.ani = p.ani.trim().to_string();
            let orig = self.ed.phone_orig.clone();
            if p.ani.is_empty() {
                self.ed.error = "Enter the phone's ANI (its phone number).".into();
            } else if p.ani != orig && self.db.phone(&p.ani).is_some() {
                self.ed.error = format!("A phone with ANI {} already exists.", p.ani);
            } else {
                let mut changed = is_new;
                if let Some(o) = &original {
                    changed = o.password != p.password
                        || o.phone_id != p.phone_id
                        || o.bypass != p.bypass
                        || o.tax_template != p.tax_template
                        || o.rate_template != p.rate_template
                        || o.ani != p.ani;
                }
                if changed {
                    p.needs_update = true;
                }
                if is_new {
                    self.db.phones.push(p.clone());
                } else if let Some(x) = self.db.phone_mut(&orig) {
                    *x = p.clone();
                }
                if !is_new && orig != p.ani {
                    for l in self.db.poll_lists.iter_mut() {
                        for a in l.phones.iter_mut().filter(|a| **a == orig) {
                            *a = p.ani.clone();
                        }
                    }
                    for a in self.db.alarms.iter_mut().filter(|a| a.ani == orig) {
                        a.ani = p.ani.clone();
                    }
                }
                self.ed.phone = None;
                self.set_active(&p.ani);
                self.message(format!("Phone {} saved", p.ani));
                self.save();
                if changed {
                    self.ed.notes_for = Some((p.ani.clone(), String::new()));
                }
            }
        } else if del {
            let orig = self.ed.phone_orig.clone();
            self.db.phones.retain(|x| x.ani != orig);
            for l in self.db.poll_lists.iter_mut() {
                l.phones.retain(|a| *a != orig);
            }
            if self.active_ani.as_deref() == Some(orig.as_str()) {
                self.active_ani = None;
                self.nav.phone = None;
            }
            self.ed.phone = None;
            self.message(format!("Phone {orig} deleted"));
            self.save();
        } else if new {
            self.new_phone();
        } else if cancel || close {
            self.ed.phone = None;
        }
        self.phone_poll_window(ctx);
    }

    fn phone_poll_window(&mut self, ctx: &egui::Context) {
        if !self.ed.phone_polls {
            return;
        }
        let ani = self.ed.phone_orig.clone();
        let mut done = false;
        let mut add = None;
        let mut rem = None;
        let current: Vec<(u32, String)> = self
            .db
            .poll_lists
            .iter()
            .filter(|l| l.phones.contains(&ani))
            .map(|l| (l.id, l.name.clone()))
            .collect();
        let avail: Vec<(u32, String)> = self
            .db
            .poll_lists
            .iter()
            .filter(|l| !l.phones.contains(&ani))
            .map(|l| (l.id, l.name.clone()))
            .collect();
        let mut pick = Picker {
            left_sel: self.ed.phone_poll_sel,
            right_sel: None,
        };
        let close = style::child_window(
            ctx,
            "phpoll",
            &format!("Assign Poll List - {ani}"),
            self.area,
            self.area.min + Vec2::new(200.0, 140.0),
            470.0,
            |ui| {
                let (a, r, _, _) = dual_list(
                    ui,
                    "phpl",
                    "Available Poll Lists:",
                    &avail,
                    &format!("Current Poll Lists for {ani}:"),
                    &current,
                    &mut pick,
                    170.0,
                    120.0,
                );
                add = a;
                rem = r;
                if style::btn(ui, "OK") {
                    done = true;
                }
            },
        );
        self.ed.phone_poll_sel = pick.left_sel;
        if let Some(id) = add {
            if let Some(l) = self.db.poll_lists.iter_mut().find(|l| l.id == id) {
                l.phones.push(ani.clone());
            }
            self.save();
        }
        if let Some(id) = rem {
            if let Some(l) = self.db.poll_lists.iter_mut().find(|l| l.id == id) {
                l.phones.retain(|a| *a != ani);
            }
            self.save();
        }
        if done || close {
            self.ed.phone_polls = false;
        }
    }

    /// After a change that the phone itself must receive, ask how to update it (like the original).
    fn download_prompt(&mut self, ctx: &egui::Context) {
        let Some((ani, _)) = self.ed.notes_for.clone() else {
            return;
        };
        let mut choice = None;
        let close = style::child_window(
            ctx,
            "dlprompt",
            "Phone Needs An Update",
            self.area,
            self.area.min + Vec2::new(240.0, 160.0),
            430.0,
            |ui| {
                ui.label("You have made a change that affects the configuration of the payphone. The payphone needs to be updated via a download. Pick an update method:");
                ui.add_space(4.0);
                let items = [
                    "Add this phone to an existing poll list",
                    "Create a new one-time poll list",
                    "Please call the phone now",
                    "Update at next contact",
                    "I will take care of the phones later",
                ];
                for (i, t) in items.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if ui
                            .add_sized([30.0, 24.0], egui::Button::new(format!("{}", i + 1)))
                            .clicked()
                        {
                            choice = Some(i);
                        }
                        ui.label(*t);
                    });
                }
            },
        );
        let cmds: Vec<String> = vec![
            "Download Rates".into(),
            "Download Speed Dial".into(),
            "Download Priority Parsing".into(),
        ];
        match choice {
            Some(0) => {
                self.ed.phone_orig = ani.clone();
                self.ed.phone_polls = true;
            }
            Some(1) => {
                let id = self.db.new_id();
                self.db.poll_lists.push(PollList {
                    id,
                    name: format!("Update {ani}"),
                    kind: "One-time".into(),
                    commands: cmds,
                    phones: vec![ani.clone()],
                    schedule: vec![chrono::Local::now().format("%H00").to_string()],
                    last_run: String::new(),
                });
                self.poll.sel = Some(id);
                self.w.poll = true;
                self.message(format!("One-time poll list created for {ani}"));
                self.save();
            }
            Some(2) => {
                self.open_dial(Some(ani.clone()));
                self.dial.selected = cmds;
            }
            Some(3) => self.message(format!("{ani} will be updated at its next contact")),
            Some(_) => self.message(format!("{ani} still needs a download")),
            None => {}
        }
        if choice.is_some() || close {
            self.ed.notes_for = None;
        }
    }

    fn route_window(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.ed.route.as_mut() else { return };
        let on_other: Vec<u32> = self
            .db
            .routes
            .iter()
            .filter(|r| r.id != ed.id)
            .flat_map(|r| r.site_ids.clone())
            .collect();
        let right: Vec<(u32, String)> = ed
            .site_ids
            .iter()
            .enumerate()
            .map(|(i, s)| (*s, format!("{:>2}. {}", i + 1, self.db.site_name(Some(*s)))))
            .collect();
        let left: Vec<(u32, String)> = self
            .db
            .sites
            .iter()
            .filter(|s| !ed.site_ids.contains(&s.id) && !on_other.contains(&s.id))
            .map(|s| (s.id, s.name.clone()))
            .collect();
        let tech = self
            .db
            .tech_of_route(Some(ed.id))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "(no technician)".into());
        let pick = &mut self.ed.route_pick;
        let mut act = (false, false, false, false);
        let title = if ed.id == 0 {
            "Route Detail - New"
        } else {
            "Route Detail"
        };
        let close = style::child_window(
            ctx,
            "route",
            title,
            self.area,
            self.area.min + Vec2::new(140.0, 80.0),
            500.0,
            |ui| {
                egui::Grid::new("rtgrid").num_columns(2).show(ui, |ui| {
                    style::field(ui, "Description", &mut ed.description, 300.0);
                    ui.label("Technician");
                    ui.label(&tech);
                    ui.end_row();
                });
                ui.label("Assign Sites (stops are visited in order):");
                let (a, r, aa, ra) = dual_list(
                    ui,
                    "rtsites",
                    "Unassigned Sites",
                    &left,
                    "Sites on route",
                    &right,
                    pick,
                    200.0,
                    140.0,
                );
                if let Some(a) = a {
                    ed.site_ids.push(a);
                }
                if let Some(r) = r {
                    ed.site_ids.retain(|x| *x != r);
                }
                if aa {
                    ed.site_ids.extend(left.iter().map(|(k, _)| *k));
                }
                if ra {
                    ed.site_ids.clear();
                }
                ui.horizontal(|ui| {
                    let sel = pick.right_sel;
                    let idx = sel.and_then(|s| ed.site_ids.iter().position(|x| *x == s));
                    if style::btn_enabled(ui, idx.map(|i| i > 0).unwrap_or(false), "Move Up") {
                        let i = idx.unwrap();
                        ed.site_ids.swap(i, i - 1);
                    }
                    if style::btn_enabled(ui, idx.map(|i| i + 1 < ed.site_ids.len()).unwrap_or(false), "Move Down") {
                        let i = idx.unwrap();
                        ed.site_ids.swap(i, i + 1);
                    }
                });
                act = ok_cancel_delete(ui, ed.id != 0);
            },
        );
        let (ok, cancel, del, new) = act;
        if ok {
            let mut r = self.ed.route.take().unwrap();
            if r.description.trim().is_empty() {
                r.description = "New Route".into();
            }
            if r.id == 0 {
                r.id = self.db.new_id();
                self.db.routes.push(r.clone());
            } else if let Some(x) = self.db.routes.iter_mut().find(|x| x.id == r.id) {
                *x = r.clone();
            }
            for s in self.db.sites.iter_mut() {
                if r.site_ids.contains(&s.id) {
                    s.route_id = Some(r.id);
                } else if s.route_id == Some(r.id) {
                    s.route_id = None;
                }
            }
            self.nav.route = Some(r.id);
            self.message(format!("Route {} saved", r.description));
            self.save();
        } else if del {
            let r = self.ed.route.take().unwrap();
            self.db.routes.retain(|x| x.id != r.id);
            for s in self.db.sites.iter_mut().filter(|s| s.route_id == Some(r.id)) {
                s.route_id = None;
            }
            for t in self.db.techs.iter_mut() {
                t.route_ids.retain(|x| *x != r.id);
            }
            self.nav.route = None;
            self.message("Route deleted");
            self.save();
        } else if new {
            self.open_route(None);
        } else if cancel || close {
            self.ed.route = None;
        }
    }

    fn tech_window(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.ed.tech.as_mut() else { return };
        let routes: Vec<(u32, String)> = self.db.routes.iter().map(|r| (r.id, r.description.clone())).collect();
        let mut act = (false, false, false, false);
        let title = if ed.id == 0 {
            "Technician Detail - New"
        } else {
            "Technician Detail"
        };
        let close = style::child_window(
            ctx,
            "tech",
            title,
            self.area,
            self.area.min + Vec2::new(180.0, 50.0),
            520.0,
            |ui| {
                ui.horizontal_top(|ui| {
                    egui::Grid::new("tg1")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("Technician ID");
                            ui.label(if ed.id == 0 { "(new)".into() } else { ed.id.to_string() });
                            ui.end_row();
                            style::field(ui, "Name", &mut ed.name, 180.0);
                            style::field(ui, "Address", &mut ed.address, 180.0);
                            style::field(ui, "City", &mut ed.city, 140.0);
                            style::field(ui, "State", &mut ed.state, 40.0);
                            style::field(ui, "Zip", &mut ed.zip, 80.0);
                            style::field(ui, "CB Handle", &mut ed.cb_handle, 120.0);
                            style::field(ui, "PIN", &mut ed.pin, 70.0);
                        });
                    egui::Grid::new("tg2")
                        .num_columns(2)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            style::field(ui, "Bus.", &mut ed.bus_phone, 110.0);
                            style::field(ui, "Home", &mut ed.home_phone, 110.0);
                            style::field(ui, "FAX", &mut ed.fax, 110.0);
                            style::field(ui, "Cellular", &mut ed.cellular, 110.0);
                            style::field(ui, "Beeper", &mut ed.beeper, 110.0);
                        });
                });
                ui.label("Routes serviced:");
                ui.horizontal_wrapped(|ui| {
                    for (id, name) in &routes {
                        let mut on = ed.route_ids.contains(id);
                        if ui.checkbox(&mut on, name).changed() {
                            if on {
                                ed.route_ids.push(*id);
                            } else {
                                ed.route_ids.retain(|x| x != id);
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Notes");
                    ui.add(
                        egui::TextEdit::multiline(&mut ed.notes)
                            .desired_width(440.0)
                            .desired_rows(2),
                    );
                });
                act = ok_cancel_delete(ui, ed.id != 0);
            },
        );
        let (ok, cancel, del, new) = act;
        if ok {
            let mut t = self.ed.tech.take().unwrap();
            if t.name.trim().is_empty() {
                t.name = "New Technician".into();
            }
            if t.id == 0 {
                t.id = self.db.new_id();
                self.db.techs.push(t.clone());
            } else if let Some(x) = self.db.techs.iter_mut().find(|x| x.id == t.id) {
                *x = t.clone();
            }
            // a route has one technician
            for o in self.db.techs.iter_mut().filter(|o| o.id != t.id) {
                o.route_ids.retain(|r| !t.route_ids.contains(r));
            }
            self.nav.tech = Some(t.id);
            self.message(format!("Technician {} saved", t.name));
            self.save();
        } else if del {
            let t = self.ed.tech.take().unwrap();
            self.db.techs.retain(|x| x.id != t.id);
            for w in self.db.work_orders.iter_mut().filter(|w| w.tech_id == Some(t.id)) {
                w.tech_id = None;
            }
            self.nav.tech = None;
            self.message("Technician deleted");
            self.save();
        } else if new {
            self.open_tech(None);
        } else if cancel || close {
            self.ed.tech = None;
        }
    }
}
