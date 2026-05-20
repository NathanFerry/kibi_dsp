#![allow(dead_code)]

mod app;
mod audio;
mod dsp;
mod ui;

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "DSP App",
        options,
        Box::new(|cc| {
            ui::theme::apply_theme(&cc.egui_ctx);
            Ok(Box::new(app::DspApp::default()))
        }),
    )
}
