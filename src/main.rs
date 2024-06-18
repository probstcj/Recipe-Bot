use eframe::egui;

#[derive(Default)]
struct ExampleApp {}

impl ExampleApp {
    fn name() -> &'static str {
        "Recipe Bot"
    }
}

impl eframe::App for ExampleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(5.0);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Recipe Bot");

                    if ui.button("Create Weekly Recipes").clicked() {
                        // Handle button click
                    }
                    if ui.button("Check/Edit Current Stock").clicked() {
                        // Handle button click
                    }

                    if ui.button("Create New Recipe - Manual").clicked() {
                        // Handle button click
                    }
                    if ui.button("Create New Recipe - From Link").clicked() {
                        // Handle button click
                    }

                    if ui.button("Light/Dark Mode Toggle").clicked() {
                        // Handle button click
                    }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((400.0, 400.0)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        ExampleApp::name(),
        native_options,
        Box::new(|_| Box::<ExampleApp>::default()),
    )
}