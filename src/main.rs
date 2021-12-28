#![feature(decl_macro)]

use app::TodoApp;

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
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
