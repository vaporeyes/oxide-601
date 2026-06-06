// ABOUTME: Entry point for the Minivac 601 emulator.
// ABOUTME: Configures and launches the eframe desktop application window.

mod app;
mod simulation;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MINIVAC 601 Emulator / Replica")
            .with_inner_size(egui::vec2(1416.0, 780.0))
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "oxide_minivac_601",
        native_options,
        Box::new(|_cc| Box::new(app::MinivacApp::default())),
    )
}
