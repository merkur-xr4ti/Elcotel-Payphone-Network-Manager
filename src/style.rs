//! Classic grey 3D look, the child-window frame, drawn icons and small form helpers.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

pub const GREY: Color32 = Color32::from_rgb(0xC0, 0xC0, 0xC0);
pub const DARK: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);
pub const NAVY: Color32 = Color32::from_rgb(0x00, 0x00, 0x80);
pub const BLUE_TEXT: Color32 = Color32::from_rgb(0x00, 0x00, 0xFF);
pub const WORKSPACE: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);
pub const WHITE: Color32 = Color32::WHITE;
pub const BLACK: Color32 = Color32::BLACK;

/// Use the system's classic sans-serif (closest to the original's MS Sans Serif) when present.
fn load_system_font(ctx: &egui::Context) {
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let candidates = [
        format!("{windir}\\Fonts\\tahoma.ttf"),
        format!("{windir}\\Fonts\\arial.ttf"),
        "/System/Library/Fonts/Supplemental/Tahoma.ttf".to_string(),
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string(),
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(&path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "system-sans".into(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                f.insert(0, "system-sans".into());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

pub fn apply(ctx: &egui::Context) {
    load_system_font(ctx);
    let mut v = egui::Visuals::light();
    v.panel_fill = GREY;
    v.window_fill = GREY;
    v.faint_bg_color = Color32::from_rgb(0xE8, 0xE8, 0xE8);
    v.extreme_bg_color = WHITE;
    v.window_corner_radius = 0.into();
    v.menu_corner_radius = 0.into();
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.window_stroke = Stroke::new(1.0_f32, BLACK);
    v.override_text_color = Some(BLACK);
    v.selection.bg_fill = NAVY;
    v.selection.stroke = Stroke::new(1.0_f32, WHITE);
    v.hyperlink_color = BLUE_TEXT;
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = 0.into();
        w.expansion = 0.0;
    }
    v.widgets.noninteractive.bg_fill = GREY;
    v.widgets.noninteractive.weak_bg_fill = GREY;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, DARK);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, BLACK);
    v.widgets.inactive.bg_fill = WHITE;
    v.widgets.inactive.weak_bg_fill = GREY;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, DARK);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, BLACK);
    v.widgets.hovered.bg_fill = WHITE;
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xD4, 0xD4, 0xD4);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BLACK);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, BLACK);
    v.widgets.active.bg_fill = Color32::from_rgb(0xA8, 0xA8, 0xA8);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0xA8, 0xA8, 0xA8);
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, BLACK);
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, BLACK);
    v.widgets.open.weak_bg_fill = Color32::from_rgb(0xD4, 0xD4, 0xD4);

    // egui keeps a light and a dark style and follows the system theme; use this look for both.
    let mut style = egui::Style {
        visuals: v,
        ..Default::default()
    };
    style.spacing.item_spacing = Vec2::new(6.0, 4.0);
    style.spacing.button_padding = Vec2::new(8.0, 3.0);
    style.spacing.interact_size.y = 20.0;
    for (ts, size) in [
        (egui::TextStyle::Body, 13.0),
        (egui::TextStyle::Button, 13.0),
        (egui::TextStyle::Small, 11.0),
        (egui::TextStyle::Heading, 16.0),
        (egui::TextStyle::Monospace, 12.5),
    ] {
        if let Some(f) = style.text_styles.get_mut(&ts) {
            f.size = size;
        }
    }
    ctx.set_style_of(egui::Theme::Light, style.clone());
    ctx.set_style_of(egui::Theme::Dark, style);
    ctx.set_theme(egui::ThemePreference::Light);
}

/// Raised (or sunken) 3D border around a rectangle.
pub fn bevel(painter: &egui::Painter, r: Rect, raised: bool) {
    let (tl, br) = if raised { (WHITE, DARK) } else { (DARK, WHITE) };
    let s1 = Stroke::new(1.0_f32, tl);
    let s2 = Stroke::new(1.0_f32, br);
    let r = r.shrink(0.5);
    painter.line_segment([r.left_top(), r.right_top()], s1);
    painter.line_segment([r.left_top(), r.left_bottom()], s1);
    painter.line_segment([r.left_bottom(), r.right_bottom()], s2);
    painter.line_segment([r.right_top(), r.right_bottom()], s2);
}

/// Draws a navy caption strip with a close box. Returns true when the close box was clicked.
pub fn caption(ui: &mut egui::Ui, title: &str, closable: bool) -> bool {
    let width = ui.available_width().max(120.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 20.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, NAVY);
    p.text(
        rect.left_center() + Vec2::new(6.0, 0.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.0),
        WHITE,
    );
    let mut clicked = false;
    if closable {
        let b = Rect::from_min_size(rect.right_top() + Vec2::new(-19.0, 2.0), Vec2::new(16.0, 16.0));
        let resp = ui.interact(b, ui.id().with(("close", title)), egui::Sense::click());
        let p = ui.painter();
        p.rect_filled(b, 0.0, GREY);
        bevel(p, b, !resp.is_pointer_button_down_on());
        let c = b.center();
        let s = Stroke::new(1.5_f32, BLACK);
        p.line_segment([c + Vec2::new(-3.5, -3.5), c + Vec2::new(3.5, 3.5)], s);
        p.line_segment([c + Vec2::new(3.5, -3.5), c + Vec2::new(-3.5, 3.5)], s);
        clicked = resp.clicked();
    }
    clicked
}

/// A child window inside the main workspace, styled like the original's MDI forms.
/// Returns true when the person closed it with the caption's close box.
pub fn child_window(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    area: Rect,
    pos: Pos2,
    width: f32,
    add: impl FnOnce(&mut egui::Ui),
) -> bool {
    let mut close = false;
    let frame = egui::Frame::default()
        .fill(GREY)
        .stroke(Stroke::new(1.0_f32, BLACK))
        .inner_margin(3.0);
    egui::Window::new(title)
        .id(egui::Id::new(("child-window", id)))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .default_pos(pos)
        // the real width up front, so the first (sizing) frame doesn't push the window left
        .default_size([width + 10.0, 60.0])
        .constrain_to(area)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(width);
            let r = ui.max_rect();
            if caption(ui, title, true) {
                close = true;
            }
            egui::Frame::default().inner_margin(6.0).show(ui, |ui| {
                ui.set_width(width - 12.0);
                add(ui);
            });
            bevel(ui.painter(), r.expand(2.0), true);
        });
    close
}

/// Label + single-line text box in a two-column grid.
pub fn field(ui: &mut egui::Ui, label: &str, value: &mut String, width: f32) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(value).desired_width(width));
    ui.end_row();
}

pub fn num_field(ui: &mut egui::Ui, label: &str, value: &mut f64, width: f32) {
    ui.label(label);
    ui.add_sized([width, 20.0], egui::DragValue::new(value).speed(0.05).max_decimals(2));
    ui.end_row();
}

pub fn combo(ui: &mut egui::Ui, id: &str, value: &mut String, options: &[String], width: f32) {
    egui::ComboBox::from_id_salt(id)
        .width(width)
        .selected_text(if value.is_empty() {
            "(none)".to_string()
        } else {
            value.clone()
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(value, String::new(), "(none)");
            for o in options {
                ui.selectable_value(value, o.clone(), o);
            }
        });
}

pub fn combo_id(ui: &mut egui::Ui, id: &str, value: &mut Option<u32>, options: &[(u32, String)], width: f32) {
    let current = options
        .iter()
        .find(|(i, _)| Some(*i) == *value)
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| "(none)".into());
    egui::ComboBox::from_id_salt(id)
        .width(width)
        .selected_text(current)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, "(none)");
            for (i, n) in options {
                ui.selectable_value(value, Some(*i), n);
            }
        });
}

/// Fixed-width button like the original's 1035-twip command buttons.
pub fn btn(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add_sized([78.0, 26.0], egui::Button::new(text)).clicked()
}
pub fn btn_enabled(ui: &mut egui::Ui, enabled: bool, text: &str) -> bool {
    ui.add_enabled(enabled, egui::Button::new(text).min_size(Vec2::new(78.0, 26.0)))
        .clicked()
}

/// A white list box with a sunken border. Rows are (key, text).
pub fn list_box<K: PartialEq + Clone>(
    ui: &mut egui::Ui,
    id: &str,
    rows: &[(K, String)],
    selected: &mut Option<K>,
    size: Vec2,
) -> Option<K> {
    let mut double = None;
    let frame = egui::Frame::default().fill(WHITE).inner_margin(2.0);
    let resp = frame.show(ui, |ui| {
        ui.set_min_size(size - Vec2::splat(4.0));
        ui.set_max_size(size - Vec2::splat(4.0));
        egui::ScrollArea::vertical()
            .id_salt(id)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let font = egui::FontId::monospace(12.5);
                for (k, t) in rows {
                    let sel = selected.as_ref() == Some(k);
                    let w = ui.available_width();
                    let (rect, r) = ui.allocate_exact_size(Vec2::new(w, 18.0), egui::Sense::click());
                    // a vertical scroll area doesn't clip sideways, so keep long rows inside the box
                    let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
                    let p = &p;
                    if sel {
                        p.rect_filled(rect, 0.0, NAVY);
                    } else if r.hovered() {
                        p.rect_filled(rect, 0.0, Color32::from_rgb(0xE4, 0xE4, 0xF0));
                    }
                    let fg = if sel { WHITE } else { BLACK };
                    let mut x = rect.left() + 3.0;
                    let mut text = t.as_str();
                    if let Some(rest) = text.strip_prefix('!') {
                        // a waiting alarm: red marker in front of the row
                        let red = if sel {
                            Color32::from_rgb(255, 120, 120)
                        } else {
                            Color32::from_rgb(200, 0, 0)
                        };
                        p.text(
                            egui::pos2(x, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            "!",
                            font.clone(),
                            red,
                        );
                        x += 8.0;
                        text = rest;
                    }
                    p.text(
                        egui::pos2(x, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        text,
                        font.clone(),
                        fg,
                    );
                    if r.clicked() {
                        *selected = Some(k.clone());
                    }
                    if r.double_clicked() {
                        double = Some(k.clone());
                    }
                }
                if rows.is_empty() {
                    ui.weak("(empty)");
                }
            });
    });
    bevel(ui.painter(), resp.response.rect, false);
    double
}

/// Simple header for a table-like list.
pub fn header(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).monospace().strong());
}

pub fn group(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let resp = egui::Frame::default()
        .inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 14,
            bottom: 6,
        })
        .show(ui, |ui| {
            ui.vertical(|ui| add(ui));
        });
    let r = resp.response.rect.shrink2(Vec2::new(1.0, 0.0));
    let r = Rect::from_min_max(r.min + Vec2::new(0.0, 7.0), r.max);
    let p = ui.painter();
    bevel(p, r, false);
    bevel(p, r.shrink(1.0), true);
    let galley = p.layout_no_wrap(title.to_string(), egui::FontId::proportional(12.5), BLACK);
    let tr = Rect::from_min_size(r.min + Vec2::new(8.0, -7.0), galley.size() + Vec2::new(6.0, 0.0));
    p.rect_filled(tr, 0.0, GREY);
    p.galley(tr.min + Vec2::new(3.0, 0.0), galley, BLACK);
}

// ---------------------------------------------------------------- icons

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Payphone,
    Business,
    Service,
    PollList,
    Config,
    Customer,
    Contract,
    Site,
    Route,
    Tech,
    Find,
    Dial,
    Alarm,
    WorkOrder,
    Journal,
    Stats,
    Menu,
    Save,
}

pub fn draw_icon(p: &egui::Painter, r: Rect, icon: Icon) {
    let c = r.center();
    let s = r.width().min(r.height()) / 32.0;
    let pt = |x: f32, y: f32| Pos2::new(c.x + x * s, c.y + y * s);
    let rr = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(pt(x0, y0), pt(x1, y1));
    let k = Stroke::new((1.2 * s).max(1.0), BLACK);
    match icon {
        Icon::Payphone | Icon::Dial => {
            p.rect_filled(rr(-9.0, -14.0, 9.0, 14.0), 2.0 * s, Color32::from_rgb(40, 70, 160));
            p.rect_filled(rr(-6.0, -11.0, 6.0, -5.0), 0.0, Color32::from_rgb(170, 220, 170));
            for row in 0..4 {
                for col in 0..3 {
                    let x = -5.0 + col as f32 * 5.0;
                    let y = -2.0 + row as f32 * 3.6;
                    p.rect_filled(rr(x - 1.4, y - 1.2, x + 1.4, y + 1.2), 0.0, WHITE);
                }
            }
            p.rect_filled(rr(-14.0, -12.0, -10.0, 8.0), 1.5 * s, BLACK);
            p.line_segment([pt(-12.0, 8.0), pt(-9.0, 12.0)], k);
            if icon == Icon::Dial {
                for i in 1..=3 {
                    let rad = 3.0 * i as f32 * s;
                    p.circle_stroke(pt(11.0, -12.0), rad, Stroke::new(1.0 * s, Color32::from_rgb(200, 0, 0)));
                }
            }
        }
        Icon::Business => {
            p.rect_filled(rr(-12.0, -6.0, 12.0, 12.0), 0.0, Color32::from_rgb(150, 90, 40));
            p.rect_filled(rr(-5.0, -11.0, 5.0, -6.0), 0.0, Color32::from_rgb(110, 60, 20));
            p.line_segment([pt(-12.0, 2.0), pt(12.0, 2.0)], k);
            p.rect_filled(rr(-2.0, 0.0, 2.0, 4.0), 0.0, Color32::from_rgb(230, 200, 60));
        }
        Icon::Service => {
            p.line_segment([pt(-10.0, 10.0), pt(6.0, -6.0)], Stroke::new(4.0 * s, DARK));
            p.circle_filled(pt(8.0, -8.0), 6.0 * s, DARK);
            p.circle_filled(pt(10.0, -10.0), 3.0 * s, GREY);
            p.line_segment(
                [pt(-10.0, -10.0), pt(10.0, 10.0)],
                Stroke::new(3.0 * s, Color32::from_rgb(200, 30, 30)),
            );
        }
        Icon::PollList | Icon::Journal | Icon::WorkOrder | Icon::Contract => {
            p.rect_filled(rr(-10.0, -13.0, 10.0, 13.0), 0.0, WHITE);
            p.rect_filled(
                rr(-10.0, -13.0, 10.0, -10.0),
                0.0,
                match icon {
                    Icon::PollList => Color32::from_rgb(0, 128, 0),
                    Icon::Journal => Color32::from_rgb(128, 0, 0),
                    Icon::WorkOrder => Color32::from_rgb(200, 120, 0),
                    _ => NAVY,
                },
            );
            for i in 0..5 {
                let y = -6.0 + i as f32 * 4.0;
                p.line_segment([pt(-7.0, y), pt(7.0, y)], Stroke::new(1.0_f32, DARK));
                if icon == Icon::PollList {
                    p.circle_filled(pt(-8.5, y), 0.9 * s, BLACK);
                }
            }
            if icon == Icon::Contract {
                p.line_segment([pt(0.0, 9.0), pt(7.0, 7.0)], Stroke::new(1.5_f32, NAVY));
            }
            p.line_segment([pt(-10.0, 13.0), pt(10.0, 13.0)], k);
            p.line_segment([pt(10.0, -13.0), pt(10.0, 13.0)], k);
        }
        Icon::Config => {
            p.circle_filled(c, 10.0 * s, DARK);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let d = Vec2::new(a.cos(), a.sin());
                p.circle_filled(c + d * 11.0 * s, 3.0 * s, DARK);
            }
            p.circle_filled(c, 4.0 * s, GREY);
        }
        Icon::Customer | Icon::Tech => {
            let body = if icon == Icon::Tech {
                Color32::from_rgb(0, 110, 110)
            } else {
                Color32::from_rgb(120, 40, 120)
            };
            p.circle_filled(pt(0.0, -7.0), 6.0 * s, Color32::from_rgb(240, 200, 160));
            p.rect_filled(rr(-9.0, 1.0, 9.0, 14.0), 3.0 * s, body);
            if icon == Icon::Tech {
                p.rect_filled(rr(-7.0, -14.0, 7.0, -10.0), 1.0, Color32::from_rgb(230, 200, 30));
            }
        }
        Icon::Site => {
            p.rect_filled(rr(-12.0, -4.0, 12.0, 13.0), 0.0, Color32::from_rgb(180, 120, 90));
            let roof = vec![pt(-14.0, -3.0), pt(0.0, -14.0), pt(14.0, -3.0)];
            p.add(egui::Shape::convex_polygon(
                roof,
                Color32::from_rgb(140, 30, 30),
                Stroke::NONE,
            ));
            p.rect_filled(rr(-3.0, 4.0, 3.0, 13.0), 0.0, Color32::from_rgb(90, 60, 30));
            p.rect_filled(rr(-10.0, 0.0, -5.0, 5.0), 0.0, Color32::from_rgb(200, 230, 255));
            p.rect_filled(rr(5.0, 0.0, 10.0, 5.0), 0.0, Color32::from_rgb(200, 230, 255));
        }
        Icon::Route => {
            let road = vec![pt(-4.0, -14.0), pt(4.0, -14.0), pt(13.0, 14.0), pt(-13.0, 14.0)];
            p.add(egui::Shape::convex_polygon(
                road,
                Color32::from_rgb(70, 70, 70),
                Stroke::NONE,
            ));
            for i in 0..3 {
                let y = -10.0 + i as f32 * 9.0;
                p.line_segment(
                    [pt(0.0, y), pt(0.0, y + 4.0)],
                    Stroke::new(2.0 * s, Color32::from_rgb(240, 220, 0)),
                );
            }
        }
        Icon::Find => {
            p.circle_stroke(pt(-3.0, -3.0), 8.0 * s, Stroke::new(2.5 * s, NAVY));
            p.line_segment([pt(3.0, 3.0), pt(12.0, 12.0)], Stroke::new(3.5 * s, BLACK));
        }
        Icon::Alarm => {
            let tri = vec![pt(0.0, -13.0), pt(13.0, 11.0), pt(-13.0, 11.0)];
            p.add(egui::Shape::convex_polygon(tri, Color32::from_rgb(240, 200, 0), k));
            p.line_segment([pt(0.0, -5.0), pt(0.0, 4.0)], Stroke::new(2.5 * s, BLACK));
            p.circle_filled(pt(0.0, 7.5), 1.4 * s, BLACK);
        }
        Icon::Stats => {
            for (i, h) in [8.0, 16.0, 11.0, 22.0].iter().enumerate() {
                let x = -12.0 + i as f32 * 6.5;
                p.rect_filled(
                    rr(x, 12.0 - h, x + 5.0, 12.0),
                    0.0,
                    [
                        NAVY,
                        Color32::from_rgb(0, 128, 0),
                        Color32::from_rgb(200, 0, 0),
                        Color32::from_rgb(200, 150, 0),
                    ][i],
                );
            }
            p.line_segment([pt(-13.0, 12.5), pt(13.0, 12.5)], k);
        }
        Icon::Menu => {
            for (i, col) in [
                NAVY,
                Color32::from_rgb(0, 128, 0),
                Color32::from_rgb(160, 0, 0),
                Color32::from_rgb(200, 150, 0),
            ]
            .iter()
            .enumerate()
            {
                let x = if i % 2 == 0 { -11.0 } else { 1.0 };
                let y = if i < 2 { -11.0 } else { 1.0 };
                p.rect_filled(rr(x, y, x + 10.0, y + 10.0), 0.0, *col);
            }
        }
        Icon::Save => {
            p.rect_filled(rr(-12.0, -12.0, 12.0, 12.0), 1.0, NAVY);
            p.rect_filled(rr(-7.0, -12.0, 7.0, -3.0), 0.0, Color32::from_rgb(220, 220, 220));
            p.rect_filled(rr(-8.0, 3.0, 8.0, 12.0), 0.0, WHITE);
        }
    }
}

/// An icon-only button for the toolbar, with a raised border.
pub fn tool_button(ui: &mut egui::Ui, icon: Icon, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 26.0), egui::Sense::click());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, GREY);
    let down = resp.is_pointer_button_down_on();
    bevel(p, rect, !down);
    let off = if down { Vec2::splat(1.0) } else { Vec2::ZERO };
    draw_icon(p, rect.shrink(4.0).translate(off), icon);
    resp.on_hover_text(tip).clicked()
}
