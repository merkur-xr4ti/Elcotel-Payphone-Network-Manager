#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod business;
mod calls;
mod config;
mod details;
mod link;
mod model;
mod nav;
mod poll;
mod service;
mod sip;
mod state;
mod style;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--call") {
        std::process::exit(link::cli_call(&args[i + 1..]));
    }
    let mut shot: Option<std::path::PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--screenshot" {
            if let Some(p) = args.get(i + 1) {
                shot = Some(std::path::PathBuf::from(p));
                i += 1;
            }
        } else if let Some(p) = args[i].strip_prefix("--screenshot=") {
            shot = Some(std::path::PathBuf::from(p));
        }
        i += 1;
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Payphone Network Manager")
        .with_inner_size([1100.0, 720.0])
        .with_min_inner_size([800.0, 560.0]);
    if shot.is_some() {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    if shot.is_some() {
        options.event_loop_builder = Some(Box::new(|b| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            b.with_activate_ignoring_other_apps(false);
        }));
    }
    eframe::run_native(
        "Payphone Network Manager",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, shot.clone())))),
    )
}
