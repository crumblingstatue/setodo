#![feature(let_chains)]

use {
    app::{default_data_file_path, TodoApp},
    eframe::egui::{self, ViewportBuilder, Visuals},
    existing_instance::Endpoint,
    std::{path::PathBuf, time::Duration},
};

mod app;
mod data;
mod tree;
mod ui;

fn main() {
    let ipc_listener = match existing_instance::establish_endpoint("rust-setodo", true).unwrap() {
        Endpoint::New(listener) => listener,
        Endpoint::Existing(_) => {
            return;
        }
    };
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size(egui::vec2(620., 480.)),
        ..Default::default()
    };
    argwerk::define! {
        #[usage = "setodo [options...]"]
        struct Args {
            help: bool,
            datafile_path: PathBuf = default_data_file_path(),
        }
        /// Use a custom data file instead of default (~/.setodo.dat)
        ["-f" | "--file", #[os] path] => {
            datafile_path = path.into();
        }
        /// Print this help.
        ["-h" | "--help"] => {
            println!("{}", HELP);
            help = true;
        }
    }
    let args = match Args::args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    if args.help {
        return;
    }
    eframe::run_native(
        "Simple egui todo",
        native_options,
        Box::new(|c_ctx| {
            let egui_ctx = c_ctx.egui_ctx.clone();
            std::thread::spawn(move || loop {
                if ipc_listener.accept().is_some() {
                    egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    egui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                std::thread::sleep(Duration::from_millis(250));
            });
            c_ctx.egui_ctx.set_visuals(Visuals::dark());
            let mut app = match TodoApp::load(args.datafile_path.clone(), &c_ctx.egui_ctx) {
                Ok(app) => app,
                Err(e) => {
                    eprintln!("{:?}", e);
                    TodoApp::new(args.datafile_path, &c_ctx.egui_ctx)
                }
            };
            let mut fonts = egui::FontDefinitions::default();
            if let Some(stored) = &app.per.stored_font_data {
                if let Err(e) =
                    egui_fontcfg::load_custom_fonts(&stored.custom, &mut fonts.font_data)
                {
                    eprintln!("Failed to load custom fonts: {e}");
                }
                fonts.families = stored.families.clone();
            }
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            app.temp.font_defs_edit_copy = fonts.clone();
            c_ctx.egui_ctx.set_fonts(fonts);
            Ok(Box::new(app))
        }),
    )
    .unwrap();
}
