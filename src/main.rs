use {
    app::TodoApp,
    eframe::egui::{self, ViewportBuilder, Visuals},
};

mod app;
mod data;
mod ui;

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size(egui::vec2(620., 480.)),
        ..Default::default()
    };
    eframe::run_native(
        "Simple egui todo",
        native_options,
        Box::new(|ctx| {
            ctx.egui_ctx.set_visuals(Visuals::dark());
            let app = match TodoApp::load() {
                Ok(app) => app,
                Err(e) => {
                    eprintln!("{:?}", e);
                    TodoApp::default()
                }
            };
            let mut fonts = egui::FontDefinitions::default();
            if let Some(stored) = &app.stored_font_data {
                if let Err(e) =
                    egui_fontcfg::load_custom_fonts(&stored.custom, &mut fonts.font_data)
                {
                    eprintln!("Failed to load custom fonts: {e}");
                }
                fonts.families = stored.families.clone();
            }
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            ctx.egui_ctx.set_fonts(fonts);
            Box::new(app)
        }),
    )
    .unwrap();
}
