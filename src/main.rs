#![feature(decl_macro)]

use app::TodoApp;
use eframe::egui;

mod app;
mod data;

fn main() {
    let app = match TodoApp::load() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("{}", e);
            TodoApp::default()
        }
    };
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(620., 480.)),
        ..Default::default()
    };
    eframe::run_native(Box::new(app), native_options);
}
