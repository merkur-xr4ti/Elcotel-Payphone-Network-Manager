//! Navigation window (customers → contracts → sites → phones, routes and technicians) and Find A Phone.

use crate::app::{at, App};
use crate::state::*;
use crate::style::{self, Icon};
use eframe::egui::{self, Pos2, Rect, Vec2};

impl App {
    pub fn nav_rows(&self, g: NavGroup) -> Vec<(String, String)> {
        let db = &self.db;
        let n = &self.nav;
        let f = n.filter.to_lowercase();
        let keep = |t: &str| f.is_empty() || t.to_lowercase().contains(&f);
        let mut rows: Vec<(String, String)> = match g {
            NavGroup::Customer => db
                .customers
                .iter()
                .map(|c| (c.id.to_string(), c.company.clone()))
                .collect(),
            NavGroup::Contract => db
                .contracts
                .iter()
                .filter(|c| n.customer.is_none() || c.customer_id == n.customer)
                .map(|c| (c.id.to_string(), format!("{:<28} {}", c.description, c.end)))
                .collect(),
            NavGroup::Site => {
                let allowed: Option<Vec<u32>> = if let Some(ct) = db.contract(n.contract) {
                    Some(ct.site_ids.clone())
                } else if n.customer.is_some() {
                    Some(
                        db.contracts
                            .iter()
                            .filter(|c| c.customer_id == n.customer)
                            .flat_map(|c| c.site_ids.clone())
                            .collect(),
                    )
                } else {
                    None
                };
                db.sites
                    .iter()
                    .filter(|s| allowed.as_ref().map(|a| a.contains(&s.id)).unwrap_or(true))
                    .map(|s| (s.id.to_string(), format!("{:<26} {}", s.name, s.city)))
                    .collect()
            }
            NavGroup::Phone => db
                .phones
                .iter()
                .filter(|p| n.site.is_none() || p.site_id == n.site)
                .map(|p| {
                    let flag = if db.open_alarms(&p.ani) > 0 { "!" } else { " " };
                    let t = if n.phone_by_location {
                        format!("{}{:<24} {}", flag, p.location, p.ani)
                    } else {
                        format!("{}{}  {}", flag, p.ani, p.location)
                    };
                    (p.ani.clone(), t)
                })
                .collect(),
            NavGroup::Route => db
                .routes
                .iter()
                .map(|r| {
                    (
                        r.id.to_string(),
                        format!("{:<26} {} sites", r.description, r.site_ids.len()),
                    )
                })
                .collect(),
            NavGroup::Tech => db.techs.iter().map(|t| (t.id.to_string(), t.name.clone())).collect(),
        };
        rows.retain(|(_, t)| keep(t));
        rows.sort_by(|a, b| a.1.trim_start_matches('!').cmp(b.1.trim_start_matches('!')));
        rows
    }

    fn nav_selected(&self, g: NavGroup) -> Option<String> {
        let n = &self.nav;
        match g {
            NavGroup::Customer => n.customer.map(|v| v.to_string()),
            NavGroup::Contract => n.contract.map(|v| v.to_string()),
            NavGroup::Site => n.site.map(|v| v.to_string()),
            NavGroup::Phone => n.phone.clone(),
            NavGroup::Route => n.route.map(|v| v.to_string()),
            NavGroup::Tech => n.tech.map(|v| v.to_string()),
        }
    }

    pub fn nav_select(&mut self, g: NavGroup, key: &str) {
        let id = key.parse::<u32>().ok();
        match g {
            NavGroup::Customer => {
                self.nav.customer = id;
                if let Some(ct) = self.db.contract(self.nav.contract) {
                    if ct.customer_id != id {
                        self.nav.contract = None;
                        self.nav.site = None;
                        self.nav.phone = None;
                    }
                }
            }
            NavGroup::Contract => {
                self.nav.contract = id;
                if let Some(ct) = self.db.contract(id) {
                    self.nav.customer = ct.customer_id;
                    if let Some(s) = self.nav.site {
                        if !ct.site_ids.contains(&s) {
                            self.nav.site = None;
                            self.nav.phone = None;
                        }
                    }
                }
            }
            NavGroup::Site => {
                self.nav.site = id;
                if let Some(s) = self.db.site(id) {
                    self.nav.route = s.route_id;
                    self.nav.tech = self.db.tech_of_route(s.route_id).map(|t| t.id);
                    if let Some(ct) = self.db.contract_of_site(s.id) {
                        self.nav.contract = Some(ct.id);
                        self.nav.customer = ct.customer_id;
                    }
                }
                if let Some(p) = self.nav.phone.clone() {
                    if self.db.phone(&p).map(|p| p.site_id) != Some(id) {
                        self.nav.phone = None;
                    }
                }
            }
            NavGroup::Phone => {
                let site = self.db.phone(key).and_then(|p| p.site_id);
                if let Some(s) = site {
                    self.nav_select(NavGroup::Site, &s.to_string());
                }
                self.set_active(key);
            }
            NavGroup::Route => {
                self.nav.route = id;
                self.nav.tech = self.db.tech_of_route(id).map(|t| t.id);
            }
            NavGroup::Tech => self.nav.tech = id,
        }
    }

    pub fn nav_open_detail(&mut self, g: NavGroup) {
        match g {
            NavGroup::Customer => self.open_customer(self.nav.customer),
            NavGroup::Contract => self.open_contract(self.nav.contract),
            NavGroup::Site => self.open_site(self.nav.site),
            NavGroup::Phone => {
                if let Some(a) = self.nav.phone.clone() {
                    self.open_phone(&a);
                } else {
                    self.new_phone();
                }
            }
            NavGroup::Route => self.open_route(self.nav.route),
            NavGroup::Tech => self.open_tech(self.nav.tech),
        }
    }

    fn nav_new(&mut self, g: NavGroup) {
        match g {
            NavGroup::Customer => self.open_customer(None),
            NavGroup::Contract => self.open_contract(None),
            NavGroup::Site => self.open_site(None),
            NavGroup::Phone => self.new_phone(),
            NavGroup::Route => self.open_route(None),
            NavGroup::Tech => self.open_tech(None),
        }
    }

    fn panel_text(&self, g: NavGroup) -> (String, usize) {
        let db = &self.db;
        let n = &self.nav;
        let none = |what: &str| (format!("No {what}\nselected"), 0);
        match g {
            NavGroup::Customer => match db.customer(n.customer) {
                Some(c) => (format!("{}\n{}\n{}", c.company, c.contact, c.phone), 0),
                None => none("customer"),
            },
            NavGroup::Contract => match db.contract(n.contract) {
                Some(c) => (format!("{}\n{} to\n{}", c.description, c.start, c.end), 0),
                None => none("contract"),
            },
            NavGroup::Site => match db.site(n.site) {
                Some(s) => (
                    format!(
                        "{}\n{}\n{}, {}  {}\n{}  {}",
                        s.name, s.address, s.city, s.state, s.zip, s.contact, s.phone
                    ),
                    0,
                ),
                None => none("site"),
            },
            NavGroup::Phone => match n.phone.as_deref().and_then(|a| db.phone(a)) {
                Some(p) => {
                    let al = db.open_alarms(&p.ani);
                    let alarms = if al > 0 {
                        format!("{al} alarm(s) waiting")
                    } else {
                        "No alarms".into()
                    };
                    (
                        format!(
                            "ANI {}\n{}\n{}   ID {}\n{}",
                            p.ani, p.location, p.model, p.phone_id, alarms
                        ),
                        al,
                    )
                }
                None => none("phone"),
            },
            NavGroup::Route => match db.route(n.route) {
                Some(r) => (format!("{}\n{} sites", r.description, r.site_ids.len()), 0),
                None => none("route"),
            },
            NavGroup::Tech => match db.tech(n.tech) {
                Some(t) => (format!("{}\nBus {}\nBeeper {}", t.name, t.bus_phone, t.beeper), 0),
                None => none("technician"),
            },
        }
    }

    fn nav_panel(&mut self, ui: &mut egui::Ui, g: NavGroup, size: Vec2, icon: Icon, icon_left: bool) {
        let (r, resp) = ui.allocate_exact_size(size, egui::Sense::click());
        let active = self.nav.group == g;
        let (text, alarms) = self.panel_text(g);
        let p = ui.painter();
        p.rect_filled(r, 0.0, style::GREY);
        style::bevel(p, r, !active);
        if active {
            style::bevel(p, r.shrink(1.0), false);
        }
        let ir = if icon_left {
            Rect::from_min_size(r.min + Vec2::new(4.0, (r.height() - 48.0) / 2.0), Vec2::splat(48.0))
        } else {
            Rect::from_min_size(
                Pos2::new(r.right() - 52.0, r.top() + (r.height() - 48.0) / 2.0),
                Vec2::splat(48.0),
            )
        };
        style::draw_icon(p, ir, icon);
        if alarms > 0 {
            let blink = (ui.input(|i| i.time) * 4.0) as i64 % 2 == 0;
            let ar = Rect::from_min_size(ir.right_top() + Vec2::new(-14.0, -2.0), Vec2::splat(18.0));
            if blink {
                style::draw_icon(p, ar, Icon::Alarm);
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
        let tx = if icon_left { r.left() + 58.0 } else { r.left() + 6.0 };
        let color = if g == NavGroup::Phone {
            style::BLUE_TEXT
        } else {
            style::BLACK
        };
        let font = egui::FontId::proportional(if g == NavGroup::Phone { 12.5 } else { 11.5 });
        let wrap = r.width() - 64.0;
        let galley = p.layout(text, font, color, wrap);
        p.galley(Pos2::new(tx, r.top() + 5.0), galley, color);
        if resp.clicked() {
            self.nav.group = g;
        }
        if resp.double_clicked() {
            self.nav.group = g;
            self.nav_open_detail(g);
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    pub fn navigation(&mut self, ctx: &egui::Context) {
        if !self.w.nav {
            return;
        }
        let area = self.area;
        let pos = at(self.area, 0.01, 0.02);
        let mut close_btn = false;
        let close = style::child_window(ctx, "nav", "Navigation", area, pos, 640.0, |ui| {
            ui.horizontal_top(|ui| {
                // left: the list for the chosen group
                ui.vertical(|ui| {
                    ui.set_width(300.0);
                    let g = self.nav.group;
                    let title = match g {
                        NavGroup::Customer => "Customers",
                        NavGroup::Contract => "Contracts",
                        NavGroup::Site => "Sites",
                        NavGroup::Phone => "Phones",
                        NavGroup::Route => "Routes",
                        NavGroup::Tech => "Technicians",
                    };
                    style::group(ui, title, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Filter:");
                            ui.add(egui::TextEdit::singleline(&mut self.nav.filter).desired_width(200.0));
                        });
                        if g == NavGroup::Phone {
                            ui.horizontal(|ui| {
                                ui.radio_value(&mut self.nav.phone_by_location, false, "By ANI");
                                ui.radio_value(&mut self.nav.phone_by_location, true, "By location name");
                            });
                            if self.nav.site.is_some() {
                                ui.small("Showing phones at the selected site");
                            }
                        }
                        let rows = self.nav_rows(g);
                        let mut sel = self.nav_selected(g);
                        let before = sel.clone();
                        let h = if g == NavGroup::Phone { 290.0 } else { 322.0 };
                        let dbl = style::list_box(ui, "navlist", &rows, &mut sel, Vec2::new(280.0, h));
                        if sel != before {
                            if let Some(k) = sel.clone() {
                                self.nav_select(g, &k);
                            }
                        }
                        if let Some(k) = dbl {
                            self.nav_select(g, &k);
                            self.nav_open_detail(g);
                        }
                    });
                });
                // right: the six context panels
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                    ui.horizontal(|ui| {
                        self.nav_panel(ui, NavGroup::Customer, Vec2::new(158.0, 66.0), Icon::Customer, true);
                        self.nav_panel(ui, NavGroup::Contract, Vec2::new(158.0, 66.0), Icon::Contract, true);
                    });
                    self.nav_panel(ui, NavGroup::Site, Vec2::new(320.0, 80.0), Icon::Site, true);
                    self.nav_panel(ui, NavGroup::Phone, Vec2::new(320.0, 80.0), Icon::Payphone, true);
                    ui.horizontal(|ui| {
                        self.nav_panel(ui, NavGroup::Route, Vec2::new(158.0, 66.0), Icon::Route, false);
                        self.nav_panel(ui, NavGroup::Tech, Vec2::new(158.0, 66.0), Icon::Tech, false);
                    });
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        if style::btn(ui, "New") {
                            self.nav_new(self.nav.group);
                        }
                        if style::btn(ui, "Clear All") {
                            let g = self.nav.group;
                            let by = self.nav.phone_by_location;
                            self.nav = NavState {
                                group: g,
                                phone_by_location: by,
                                ..Default::default()
                            };
                            self.active_ani = None;
                        }
                        if style::btn(ui, "Detail") {
                            self.nav_open_detail(self.nav.group);
                        }
                        if style::btn(ui, "Close") {
                            close_btn = true;
                        }
                    });
                    ui.add_space(2.0);
                    ui.small("Click a panel to list its records; double-click to open it.");
                });
            });
        });
        if close || close_btn {
            self.w.nav = false;
        }
    }

    pub fn find_window(&mut self, ctx: &egui::Context) {
        if !self.w.find {
            return;
        }
        let mut done = false;
        let mut choose: Option<String> = None;
        let pos = at(self.area, 0.3, 0.15);
        let close = style::child_window(ctx, "find", "Find A Phone", self.area, pos, 380.0, |ui| {
            let f = &mut self.find;
            egui::Grid::new("findgrid").num_columns(2).show(ui, |ui| {
                style::field(ui, "ANI or Partial ANI", &mut f.ani, 200.0);
                style::field(ui, "Phone ID", &mut f.phone_id, 200.0);
                style::field(ui, "Site Name", &mut f.site, 200.0);
            });
            ui.horizontal(|ui| {
                if style::btn(ui, "Find") {
                    let a = f.ani.trim().to_lowercase();
                    let id = f.phone_id.trim().to_lowercase();
                    let s = f.site.trim().to_lowercase();
                    f.results = self
                        .db
                        .phones
                        .iter()
                        .filter(|p| {
                            let site = self.db.site_name(p.site_id).to_lowercase();
                            (a.is_empty() || p.ani.to_lowercase().replace('-', "").contains(&a.replace('-', "")))
                                && (id.is_empty() || p.phone_id.to_lowercase().contains(&id))
                                && (s.is_empty() || site.contains(&s) || p.location.to_lowercase().contains(&s))
                        })
                        .map(|p| p.ani.clone())
                        .collect();
                    f.searched = true;
                    f.sel = f.results.first().cloned();
                }
                if style::btn_enabled(ui, f.sel.is_some(), "Choose") {
                    choose = f.sel.clone();
                }
                if style::btn(ui, "Cancel") {
                    done = true;
                }
            });
            let rows: Vec<(String, String)> = f
                .results
                .iter()
                .map(|a| {
                    let p = self.db.phone(a);
                    (
                        a.clone(),
                        format!(
                            "{}  {:<18} {}",
                            a,
                            p.map(|p| p.location.as_str()).unwrap_or(""),
                            self.db.site_name(p.and_then(|p| p.site_id))
                        ),
                    )
                })
                .collect();
            if f.searched {
                ui.label(format!("{} phone(s) found", rows.len()));
            }
            if let Some(d) = style::list_box(ui, "findres", &rows, &mut f.sel, Vec2::new(368.0, 150.0)) {
                choose = Some(d);
            }
        });
        if let Some(a) = choose {
            self.w.nav = true;
            self.nav.group = NavGroup::Phone;
            self.nav_select(NavGroup::Phone, &a);
            self.message(format!("Phone {a} selected"));
            done = true;
        }
        if close || done {
            self.w.find = false;
        }
    }
}
