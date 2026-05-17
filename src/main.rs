fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "ReviewDesk",
        native_options,
        Box::new(|_cc| Ok(Box::<reviewdesk::app::ReviewDeskApp>::default())),
    )
}
