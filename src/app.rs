//! The main window: menu bar, toolbar, status bar, workspace and the menu screens.

use crate::model::*;
use crate::style::{self, Icon};
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::time::Instant;

pub use crate::state::*;

pub struct App {
    pub db: Db,
    pub persist: bool,
    pub info: String,
    pub active_ani: Option<String>,
    pub splash_until: Option<Instant>,
    pub area: Rect,
    pub w: Windows,
    pub nav: NavState,
    pub ed: Editors,
    pub find: FindState,
    pub dial: DialState,
    pub svc: ServiceState,
    pub biz: BusinessState,
    pub poll: PollState,
    pub cfg: ConfigState,
    pub last_schedule_check: String,
    // screenshot mode
    pub shot_path: Option<std::path::PathBuf>,
    pub frames: u32,
    pub shot_requested: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, shot_path: Option<std::path::PathBuf>) -> Self {
        style::apply(&cc.egui_ctx);
        let shot = shot_path.is_some();
        let db = if shot { Db::sample() } else { Db::load_or_sample() };
        let mut app = App {
            db,
            persist: !shot,
            info: "No contact with Poll Manager".into(),
            active_ani: None,
            splash_until: if shot {
                None
            } else {
                Some(Instant::now() + std::time::Duration::from_millis(2500))
            },
            area: Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 650.0)),
            w: Windows {
                menu: true,
                nav: true,
                ..Default::default()
            },
            nav: NavState::default(),
            ed: Editors::default(),
            find: FindState::default(),
            dial: DialState::default(),
            svc: ServiceState::default(),
            biz: BusinessState::default(),
            poll: PollState::default(),
            cfg: ConfigState::default(),
            last_schedule_check: String::new(),
            shot_path,
            frames: 0,
            shot_requested: false,
        };
        if shot {
            // A populated, representative view for the picture.
            let c = app.db.customers.first().map(|c| c.id);
            app.nav.customer = c;
            app.nav.contract = app.db.contracts.first().map(|c| c.id);
            app.nav.site = app.db.sites.first().map(|s| s.id);
            app.nav.group = NavGroup::Phone;
            if let Some(p) = app.db.phones.get(1) {
                app.nav.phone = Some(p.ani.clone());
                app.active_ani = Some(p.ani.clone());
            }
            app.nav.route = app.db.sites.first().and_then(|s| s.route_id);
            app.nav.tech = app.db.tech_of_route(app.nav.route).map(|t| t.id);
            // Screenshot scenes: PAYPHONE_SCENE=dial,phone,... opens more windows for the picture
            // (used to make the pictures in docs/screenshots; see the README).
            if let Ok(scene) = std::env::var("PAYPHONE_SCENE") {
                app.w.menu = false;
                for s in scene.split(',') {
                    match s.trim() {
                        "nav" => app.w.nav = true,
                        "phone" => {
                            app.w.nav = false;
                            let a = app.db.phones[1].ani.clone();
                            app.open_phone(&a);
                        }
                        "site" => {
                            app.w.nav = false;
                            app.open_site(app.nav.site);
                        }
                        "dial" => {
                            app.w.nav = false;
                            let a = app.db.phones[1].ani.clone();
                            app.open_dial(Some(a.clone()));
                            // show the phone's last recorded contact (sample data)
                            app.dial.result = app.db.contacts.iter().rev().find(|c| c.ani == a).cloned();
                            app.dial.status = "Ready".into();
                        }
                        "status" => app.w.status = true,
                        "handle" => {
                            app.svc.handle_alarm = app.db.alarms.iter().find(|a| !a.handled).map(|a| a.id);
                            app.w.handle = true;
                        }
                        "wo" => app.w.wo = true,
                        "journal" => {
                            app.w.journal = true;
                            app.biz.j_summary = true;
                        }
                        "poll" => app.w.poll = true,
                        "stats" => app.w.stats = true,
                        "diag" => {
                            app.svc.diag_ani = Some(app.db.phones[1].ani.clone());
                            app.w.diag = true;
                        }
                        "service" => app.w.service = true,
                        "business" => app.w.business = true,
                        "config" => app.w.config = true,
                        "global" => app.w.global = true,
                        "about" => app.w.about = true,
                        "modem" => {
                            app.w.nav = false;
                            let s = &mut app.db.settings;
                            s.connection = crate::link::Connection::Sip;
                            s.comm_port = "COM3".into();
                            s.sip_server = "sip.example.com".into();
                            s.sip_port = 5060;
                            s.sip_user = "payphone-manager".into();
                            s.sip_password = "example".into();
                            app.cfg.settings = app.db.settings.clone();
                            app.cfg.ports = vec!["COM1".into(), "COM3".into()];
                            app.w.modem = true;
                        }
                        "files" => {
                            app.open_files("Rates");
                            let f = app.db.config_files[0].clone();
                            app.cfg.file_edit = Some((f.clone(), f.name));
                        }
                        "commission" => {
                            app.biz.comm_chosen = app.db.contracts.iter().map(|c| c.id).collect();
                            app.w.commission = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        app
    }

    pub fn save(&mut self) {
        if !self.persist {
            return;
        }
        if let Err(e) = self.db.save() {
            self.info = format!("Could not save database: {e}");
        }
    }

    pub fn message(&mut self, text: impl Into<String>) {
        let t = text.into();
        self.info = t.clone();
        self.db.messages.push(format!("{}  {}", now_stamp(), t));
        if self.db.messages.len() > 300 {
            self.db.messages.remove(0);
        }
    }

    pub fn set_active(&mut self, ani: &str) {
        self.active_ani = Some(ani.to_string());
        self.nav.phone = Some(ani.to_string());
    }

    // ------------------------------------------------------------ chrome

    fn menu_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save Database").clicked() {
                    self.save();
                    self.message("Database saved");
                    ui.close_menu();
                }
                if ui.button("Backup Database...").clicked() {
                    self.w.backup = true;
                    ui.close_menu();
                }
                if ui.button("Information Messages").clicked() {
                    self.w.info = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    self.save();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Payphone", |ui| {
                if ui.button("Navigation").clicked() {
                    self.w.nav = true;
                    ui.close_menu();
                }
                if ui.button("Find A Phone...").clicked() {
                    self.w.find = true;
                    ui.close_menu();
                }
                if ui.button("Dial Payphone...").clicked() {
                    self.open_dial(self.active_ani.clone());
                    ui.close_menu();
                }
                if ui.button("Global Phone Changes...").clicked() {
                    self.w.global = true;
                    ui.close_menu();
                }
            });
            ui.menu_button("Business", |ui| {
                for (t, f) in [
                    ("Customer Information", 0),
                    ("Contract Information", 1),
                    ("Cost Accounting", 2),
                    ("Commission Calculation", 3),
                    ("Charged Rates", 4),
                    ("Tax Rates", 5),
                ] {
                    if ui.button(t).clicked() {
                        self.business_item(f);
                        ui.close_menu();
                    }
                }
            });
            ui.menu_button("Service", |ui| {
                for (t, f) in [
                    ("Phone Status", 0),
                    ("Work Order Maintenance", 1),
                    ("Customer Service", 2),
                    ("View Diagnostics", 3),
                    ("View RAM Image", 4),
                ] {
                    if ui.button(t).clicked() {
                        self.service_item(f);
                        ui.close_menu();
                    }
                }
            });
            ui.menu_button("Poll List", |ui| {
                if ui.button("Poll List Maintenance").clicked() {
                    self.w.poll = true;
                    ui.close_menu();
                }
                if ui.button("Run Due Poll Lists Now").clicked() {
                    self.run_due_polls(true);
                    ui.close_menu();
                }
            });
            ui.menu_button("Configuration", |ui| {
                for (i, t) in CONFIG_ITEMS.iter().enumerate() {
                    if ui.button(*t).clicked() {
                        self.config_item(i);
                        ui.close_menu();
                    }
                }
                ui.separator();
                if ui.button("Voice Files").clicked() {
                    self.open_files("Voice");
                    ui.close_menu();
                }
                if ui.button("Account Codes").clicked() {
                    self.config_item(10);
                    ui.close_menu();
                }
                if ui.button("Purge Journal Entries").clicked() {
                    self.config_item(11);
                    ui.close_menu();
                }
            });
            ui.menu_button("Window", |ui| {
                if ui.button("Main Menu").clicked() {
                    self.w.menu = true;
                    ui.close_menu();
                }
                if ui.button("Close All").clicked() {
                    self.w = Windows::default();
                    self.ed = Editors::default();
                    ui.close_menu();
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Statistics").clicked() {
                    self.w.stats = true;
                    ui.close_menu();
                }
                if ui.button("About...").clicked() {
                    self.w.about = true;
                    ui.close_menu();
                }
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if style::tool_button(ui, Icon::Menu, "Main Menu") {
                self.w.menu = true;
            }
            if style::tool_button(ui, Icon::Save, "Save Database") {
                self.save();
                self.message("Database saved");
            }
            ui.add_space(6.0);
            if style::tool_button(ui, Icon::Payphone, "Navigation") {
                self.w.nav = true;
            }
            if style::tool_button(ui, Icon::Find, "Find A Phone") {
                self.w.find = true;
            }
            if style::tool_button(ui, Icon::Dial, "Dial Payphone") {
                self.open_dial(self.active_ani.clone());
            }
            ui.add_space(6.0);
            if style::tool_button(ui, Icon::Alarm, "Phone Status") {
                self.w.status = true;
            }
            if style::tool_button(ui, Icon::WorkOrder, "Work Orders") {
                self.w.wo = true;
            }
            if style::tool_button(ui, Icon::PollList, "Poll Lists") {
                self.w.poll = true;
            }
            if style::tool_button(ui, Icon::Journal, "Cost Accounting Journal") {
                self.w.journal = true;
            }
            ui.add_space(6.0);
            if style::tool_button(ui, Icon::Stats, "Statistics") {
                self.w.stats = true;
            }
            if style::tool_button(ui, Icon::Config, "Configuration Menu") {
                self.w.config = true;
            }
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let h = 20.0;
            let ani_text = match &self.active_ani {
                Some(a) => format!("Active ANI: {a}"),
                None => "No active ANI".into(),
            };
            let panel = |ui: &mut egui::Ui, w: f32, text: egui::RichText| {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(w, h), egui::Sense::click());
                style::bevel(ui.painter(), r, false);
                let g = ui.painter().layout_no_wrap(
                    text.text().to_string(),
                    egui::FontId::proportional(12.5),
                    text_color(&text),
                );
                ui.painter()
                    .galley(r.left_center() + Vec2::new(5.0, -g.size().y / 2.0), g, style::BLACK);
                resp
            };
            fn text_color(t: &egui::RichText) -> egui::Color32 {
                if t.text().starts_with("Active") || t.text().starts_with("No active") {
                    style::BLUE_TEXT
                } else {
                    style::BLACK
                }
            }
            if panel(ui, 190.0, egui::RichText::new(ani_text)).clicked() && self.active_ani.is_some() {
                let a = self.active_ani.clone().unwrap();
                self.open_phone(&a);
            }
            let rest = (ui.available_width() - 110.0).max(100.0);
            if panel(ui, rest, egui::RichText::new(self.info.clone()))
                .on_hover_text("Click for information messages")
                .clicked()
            {
                self.w.info = true;
            }
            let clock = chrono::Local::now().format("%I:%M:%S %p").to_string();
            panel(ui, 100.0, egui::RichText::new(clock));
        });
    }

    // ------------------------------------------------------------ menu screens

    fn main_menu(&mut self, ctx: &egui::Context) {
        if !self.w.menu {
            return;
        }
        let area = self.area;
        let pos = Pos2::new(area.right() - 262.0, area.top() + 8.0);
        let mut pick = None;
        let close = style::child_window(ctx, "menu", "Main Menu", area, pos, 240.0, |ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(228.0, 236.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, style::WHITE);
            style::bevel(ui.painter(), rect, false);
            let items = [
                (Icon::Payphone, "Payphone Administration", Vec2::new(114.0, 34.0), 0),
                (Icon::Business, "Business\nAdministration", Vec2::new(56.0, 108.0), 1),
                (Icon::Service, "Service\nManagement", Vec2::new(172.0, 108.0), 2),
                (Icon::PollList, "Poll List", Vec2::new(56.0, 184.0), 3),
                (Icon::Config, "Configuration", Vec2::new(172.0, 184.0), 4),
            ];
            for (icon, label, c, id) in items {
                let center = rect.min + c;
                let ir = Rect::from_center_size(center, Vec2::splat(44.0));
                let resp = ui.interact(ir, ui.id().with(("menuicon", id)), egui::Sense::click());
                let p = ui.painter();
                if resp.hovered() {
                    p.rect_filled(ir.expand(2.0), 0.0, egui::Color32::from_rgb(230, 230, 245));
                }
                style::draw_icon(p, ir, icon);
                for (li, line) in label.lines().enumerate() {
                    p.text(
                        center + Vec2::new(0.0, 25.0 + li as f32 * 14.0),
                        egui::Align2::CENTER_TOP,
                        line,
                        egui::FontId::proportional(12.0),
                        style::BLACK,
                    );
                }
                let lr = Rect::from_center_size(center + Vec2::new(0.0, 33.0), Vec2::new(110.0, 26.0));
                let lresp = ui.interact(lr, ui.id().with(("menulabel", id)), egui::Sense::click());
                if resp.clicked() || lresp.clicked() {
                    pick = Some(id);
                }
                if resp.hovered() || lresp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
            }
        });
        if close {
            self.w.menu = false;
        }
        match pick {
            Some(0) => self.w.nav = true,
            Some(1) => self.w.business = true,
            Some(2) => self.w.service = true,
            Some(3) => self.w.poll = true,
            Some(4) => self.w.config = true,
            _ => {}
        }
    }

    fn numbered_menu(
        ctx: &egui::Context,
        id: &str,
        title: &str,
        area: Rect,
        pos: Pos2,
        items: &[&str],
    ) -> (bool, Option<usize>) {
        let mut pick = None;
        let close = style::child_window(ctx, id, title, area, pos, 300.0, |ui| {
            for (i, t) in items.iter().enumerate() {
                ui.horizontal(|ui| {
                    let key = if i == 9 { "10".to_string() } else { format!("{}", i + 1) };
                    if ui.add_sized([34.0, 28.0], egui::Button::new(key)).clicked() {
                        pick = Some(i);
                    }
                    if ui.add(egui::Label::new(*t).sense(egui::Sense::click())).clicked() {
                        pick = Some(i);
                    }
                });
            }
        });
        // number keys pick an item, like the original's &1..&6 accelerators
        (close, pick)
    }

    pub fn business_item(&mut self, i: usize) {
        match i {
            0 => {
                self.w.nav = true;
                self.nav.group = NavGroup::Customer;
            }
            1 => {
                self.w.nav = true;
                self.nav.group = NavGroup::Contract;
            }
            2 => self.w.journal = true,
            3 => self.w.commission = true,
            4 => self.w.rate_templates = true,
            5 => self.w.tax_templates = true,
            _ => {}
        }
    }

    pub fn service_item(&mut self, i: usize) {
        match i {
            0 => self.w.status = true,
            1 => self.w.wo = true,
            2 => self.w.complaints = true,
            3 => {
                self.svc.diag_ani = self.active_ani.clone();
                self.w.diag = true;
            }
            4 => {
                self.svc.ram_ani = self.active_ani.clone();
                self.w.ram = true;
            }
            _ => {}
        }
    }

    pub fn config_item(&mut self, i: usize) {
        match i {
            0 => self.open_files("Rates"),
            1 => self.open_files("Speed Dial"),
            2 => self.open_files("Priority Parsing"),
            3 => self.open_files("Messages"),
            4 => self.open_files("Rep/Cards"),
            5 => self.open_files("Registers & Options"),
            6 => self.w.macros = true,
            7 => self.w.global = true,
            8 => {
                self.cfg.settings = self.db.settings.clone();
                self.w.sysconfig = true;
            }
            9 => {
                self.cfg.settings = self.db.settings.clone();
                self.w.modem = true;
            }
            10 => self.w.accounts = true,
            11 => self.w.purge = true,
            _ => {}
        }
        self.w.config = true;
    }

    fn sub_menus(&mut self, ctx: &egui::Context) {
        let area = self.area;
        if self.w.business {
            let items = [
                "Customer Information",
                "Contract Information",
                "Cost Accounting",
                "Commission Calculation",
                "Charged Rates",
                "Tax Rates",
            ];
            let (c, p) = Self::numbered_menu(
                ctx,
                "bizmenu",
                "Business Administration Menu",
                area,
                at(self.area, 0.42, 0.04),
                &items,
            );
            if c {
                self.w.business = false;
            }
            if let Some(i) = p {
                self.business_item(i);
            }
        }
        if self.w.service {
            let items = [
                "Phone Status",
                "Work Order Maintenance",
                "Customer Service (211)",
                "View Diagnostics",
                "View RAM Image",
            ];
            let (c, p) = Self::numbered_menu(
                ctx,
                "svcmenu",
                "Service Management Menu",
                area,
                at(self.area, 0.42, 0.08),
                &items,
            );
            if c {
                self.w.service = false;
            }
            if let Some(i) = p {
                self.service_item(i);
            }
        }
        if self.w.config {
            let (c, p) = Self::numbered_menu(
                ctx,
                "cfgmenu",
                "Configuration Menu",
                area,
                at(self.area, 0.40, 0.03),
                CONFIG_ITEMS,
            );
            if c {
                self.w.config = false;
            }
            if let Some(i) = p {
                self.config_item(i);
            }
        }
    }

    fn about(&mut self, ctx: &egui::Context) {
        if !self.w.about {
            return;
        }
        let mut ok = false;
        let close = style::child_window(
            ctx,
            "about",
            "About Payphone Network Manager",
            self.area,
            at(self.area, 0.25, 0.18),
            440.0,
            |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(48.0), egui::Sense::hover());
                    style::draw_icon(ui.painter(), r, Icon::Payphone);
                    ui.vertical(|ui| {
                        ui.heading("Payphone Network Manager");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.label("Manages payphone customers, sites, phones, routes,");
                        ui.label("technicians, poll lists, service and accounting.");
                    });
                });
                ui.add_space(8.0);
                style::group(ui, "Disclaimer", |ui| {
                    ui.label(
                        "This program was created with AI, using the decompiled source code of the original \
                     payphone management software (PNM Plus) as its reference.",
                    );
                    ui.add_space(2.0);
                    ui.label(
                        "Its communication with payphones was tested against an emulated Elcotel Series 5 \
                     payphone running the phone's own firmware, not against every real phone. \
                     Use it with care on phones in service.",
                    );
                    ui.add_space(2.0);
                    ui.label("It is not made, endorsed or supported by Elcotel or QuorTech Solutions.");
                });
                ui.add_space(6.0);
                ui.label(format!("Database: {}", Db::path_display()));
                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    if style::btn(ui, "OK") {
                        ok = true;
                    }
                });
            },
        );
        if close || ok {
            self.w.about = false;
        }
    }

    fn info_messages(&mut self, ctx: &egui::Context) {
        if !self.w.info {
            return;
        }
        let mut done = false;
        let msgs = self.db.messages.clone();
        let close = style::child_window(
            ctx,
            "info",
            "Information Messages",
            self.area,
            at(self.area, 0.2, 0.5),
            460.0,
            |ui| {
                let rows: Vec<(usize, String)> = msgs.iter().rev().cloned().enumerate().collect();
                let mut sel = None;
                style::list_box(ui, "infolist", &rows, &mut sel, Vec2::new(448.0, 200.0));
                ui.horizontal(|ui| {
                    if style::btn(ui, "&Close".trim_start_matches('&')) {
                        done = true;
                    }
                });
            },
        );
        if close || done {
            self.w.info = false;
        }
    }

    fn statistics(&mut self, ctx: &egui::Context) {
        if !self.w.stats {
            return;
        }
        let db = &self.db;
        let mut ok = false;
        let size = std::fs::metadata(Db::path()).map(|m| m.len()).unwrap_or(0);
        let close = style::child_window(
            ctx,
            "stats",
            "Statistics",
            self.area,
            at(self.area, 0.18, 0.06),
            520.0,
            |ui| {
                ui.horizontal_top(|ui| {
                    style::group(ui, "Database", |ui| {
                        egui::Grid::new("statsdb")
                            .num_columns(2)
                            .spacing([16.0, 3.0])
                            .show(ui, |ui| {
                                let rows = [
                                    ("Customers", db.customers.len()),
                                    ("Contracts", db.contracts.len()),
                                    ("Sites", db.sites.len()),
                                    ("Routes", db.routes.len()),
                                    ("Technicians", db.techs.len()),
                                    ("Acct'g Journal", db.journal.len()),
                                    ("Contact History", db.contacts.len()),
                                    ("Complaints", db.complaints.len()),
                                    ("Work Orders", db.work_orders.len()),
                                    ("Poll Lists", db.poll_lists.len()),
                                ];
                                for (n, c) in rows {
                                    ui.label(n);
                                    ui.label(c.to_string());
                                    ui.end_row();
                                }
                            });
                    });
                    style::group(ui, "Phone counts by model", |ui| {
                        egui::Grid::new("statsmodels")
                            .num_columns(2)
                            .spacing([16.0, 3.0])
                            .show(ui, |ui| {
                                for m in MODELS {
                                    let n = db.phones.iter().filter(|p| p.model == *m).count();
                                    ui.label(*m);
                                    ui.label(n.to_string());
                                    ui.end_row();
                                }
                                ui.label(egui::RichText::new("Total phones in database:").strong());
                                ui.label(egui::RichText::new(db.phones.len().to_string()).strong());
                                ui.end_row();
                                let un = db.phones.iter().filter(|p| p.site_id.is_none()).count();
                                ui.label("Unassigned to a site");
                                ui.label(un.to_string());
                                ui.end_row();
                            });
                    });
                });
                style::group(ui, "Resources", |ui| {
                    egui::Grid::new("statsres")
                        .num_columns(2)
                        .spacing([16.0, 3.0])
                        .show(ui, |ui| {
                            ui.label("Company");
                            ui.label(&db.settings.company);
                            ui.end_row();
                            ui.label("User Name / ID");
                            ui.label(format!("{}  ({})", db.settings.user_name, db.settings.user_id));
                            ui.end_row();
                            ui.label("Database Path");
                            ui.label(Db::path_display());
                            ui.end_row();
                            ui.label("DB Size");
                            ui.label(format!("{:.1} KB", size as f64 / 1024.0));
                            ui.end_row();
                            ui.label("Call Records (SMDR)");
                            ui.label(db.call_records.len().to_string());
                            ui.end_row();
                            ui.label("Pgm Version");
                            ui.label(env!("CARGO_PKG_VERSION"));
                            ui.end_row();
                        });
                });
                ui.vertical_centered(|ui| {
                    if style::btn(ui, "OK") {
                        ok = true;
                    }
                });
            },
        );
        if close || ok {
            self.w.stats = false;
        }
    }

    fn backup(&mut self, ctx: &egui::Context) {
        if !self.w.backup {
            return;
        }
        if self.cfg.backup_path.is_empty() {
            let dir = Db::path().parent().map(|p| p.to_path_buf()).unwrap_or_default();
            self.cfg.backup_path = dir
                .join(format!("backup-{}.json", chrono::Local::now().format("%Y%m%d-%H%M")))
                .display()
                .to_string();
        }
        let mut go = false;
        let mut cancel = false;
        let path = &mut self.cfg.backup_path;
        let close = style::child_window(
            ctx,
            "backup",
            "Backup Database",
            self.area,
            at(self.area, 0.25, 0.3),
            460.0,
            |ui| {
                ui.label("Save a copy of every record to this file:");
                ui.add(egui::TextEdit::singleline(path).desired_width(440.0));
                ui.horizontal(|ui| {
                    if style::btn(ui, "OK") {
                        go = true;
                    }
                    if style::btn(ui, "Cancel") {
                        cancel = true;
                    }
                });
            },
        );
        if go {
            let p = std::path::PathBuf::from(self.cfg.backup_path.clone());
            match self.db.export_to(&p) {
                Ok(()) => self.message(format!("Backup written to {}", p.display())),
                Err(e) => self.message(format!("Backup failed: {e}")),
            }
            self.cfg.backup_path.clear();
        }
        if close || go || cancel {
            self.w.backup = false;
        }
    }

    fn splash(&mut self, ctx: &egui::Context) -> bool {
        let Some(until) = self.splash_until else { return false };
        if Instant::now() >= until {
            self.splash_until = None;
            return false;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        let mut dismiss = false;
        egui::Area::new(egui::Id::new("splash"))
            .fixed_pos(self.area.center() - Vec2::new(210.0, 130.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(420.0, 260.0), egui::Sense::click());
                let p = ui.painter();
                p.rect_filled(r, 0.0, style::GREY);
                style::bevel(p, r, true);
                let inner = r.shrink(10.0);
                // neutral gradient banner instead of the original's artwork
                let n = 40;
                for i in 0..n {
                    let t = i as f32 / n as f32;
                    let col = egui::Color32::from_rgb(
                        (20.0 + 40.0 * t) as u8,
                        (40.0 + 60.0 * t) as u8,
                        (120.0 + 100.0 * t) as u8,
                    );
                    let x0 = inner.left() + inner.width() * t;
                    let rr = Rect::from_min_max(
                        Pos2::new(x0, inner.top()),
                        Pos2::new(x0 + inner.width() / n as f32 + 1.0, inner.top() + 150.0),
                    );
                    p.rect_filled(rr, 0.0, col);
                }
                style::draw_icon(
                    p,
                    Rect::from_center_size(inner.left_top() + Vec2::new(60.0, 75.0), Vec2::splat(80.0)),
                    Icon::Payphone,
                );
                p.text(
                    inner.left_top() + Vec2::new(120.0, 50.0),
                    egui::Align2::LEFT_TOP,
                    "Payphone Network",
                    egui::FontId::proportional(26.0),
                    style::WHITE,
                );
                p.text(
                    inner.left_top() + Vec2::new(120.0, 82.0),
                    egui::Align2::LEFT_TOP,
                    "Manager",
                    egui::FontId::proportional(26.0),
                    style::WHITE,
                );
                let y = inner.top() + 162.0;
                let s = &self.db.settings;
                for (i, (k, v)) in [
                    ("Registered to:", s.user_name.as_str()),
                    ("Company:", s.company.as_str()),
                    ("Version", "1.0"),
                ]
                .iter()
                .enumerate()
                {
                    p.text(
                        Pos2::new(inner.left() + 10.0, y + i as f32 * 20.0),
                        egui::Align2::LEFT_TOP,
                        *k,
                        egui::FontId::proportional(13.0),
                        style::BLACK,
                    );
                    p.text(
                        Pos2::new(inner.left() + 120.0, y + i as f32 * 20.0),
                        egui::Align2::LEFT_TOP,
                        *v,
                        egui::FontId::proportional(13.0),
                        style::NAVY,
                    );
                }
                if resp.clicked() {
                    dismiss = true;
                }
            });
        if dismiss {
            self.splash_until = None;
        }
        true
    }

    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot_path.clone() else { return };
        ctx.request_repaint();
        self.frames += 1;
        if self.frames >= 120 && !self.shot_requested {
            self.shot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let mut shot = None;
        ctx.input(|i| {
            for e in &i.raw.events {
                if let egui::Event::Screenshot { image, .. } = e {
                    shot = Some(image.clone());
                }
            }
        });
        if let Some(img) = shot {
            let [w, h] = img.size;
            let mut bytes = Vec::with_capacity(w * h * 4);
            for c in &img.pixels {
                bytes.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
            }
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = image::save_buffer(&path, &bytes, w as u32, h as u32, image::ColorType::Rgba8) {
                eprintln!("could not save screenshot: {e}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.frames > 600 {
            // never hang a check
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// A point inside the workspace, as fractions of its width and height.
pub fn at(area: Rect, fx: f32, fy: f32) -> Pos2 {
    area.min + Vec2::new(area.width() * fx, area.height() * fy)
}

/// Order follows the original help's configuration chapter (exact menu order is estimated).
pub const CONFIG_ITEMS: &[&str] = &[
    "Rate Files",
    "Speed Dial Files",
    "Priority Parsing Files",
    "Scrolling Messages",
    "Rep/Cards Templates",
    "Registers & Options Templates",
    "Phone Macros",
    "Global Phone Changes",
    "System Configuration",
    "Modem/Communications Settings",
];

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_screenshot(ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        self.tick(ctx);

        egui::TopBottomPanel::top("menubar")
            .frame(
                egui::Frame::default()
                    .fill(style::GREY)
                    .inner_margin(egui::Margin::symmetric(4, 2)),
            )
            .show(ctx, |ui| self.menu_bar(ui, ctx));
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::default()
                    .fill(style::GREY)
                    .inner_margin(egui::Margin::symmetric(4, 3)),
            )
            .show(ctx, |ui| {
                self.toolbar(ui);
                let r = ui.max_rect().expand2(Vec2::new(4.0, 3.0));
                style::bevel(ui.painter(), r, true);
            });
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::default()
                    .fill(style::GREY)
                    .inner_margin(egui::Margin::symmetric(3, 3)),
            )
            .show(ctx, |ui| self.status_bar(ui));
        let central = egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(style::WORKSPACE))
            .show(ctx, |_ui| {});
        self.area = central.response.rect;

        if self.splash(ctx) {
            return;
        }

        self.main_menu(ctx);
        self.sub_menus(ctx);
        self.navigation(ctx);
        self.find_window(ctx);
        self.detail_windows(ctx);
        self.service_windows(ctx);
        self.business_windows(ctx);
        self.poll_windows(ctx);
        self.dial_window(ctx);
        self.config_windows(ctx);
        self.statistics(ctx);
        self.info_messages(ctx);
        self.backup(ctx);
        self.about(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save();
    }
}
