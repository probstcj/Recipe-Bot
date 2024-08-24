// GUI Imports
use eframe::{egui, CreationContext};

// Web server imports
use actix_web::{get, App as ActixApp, HttpServer, HttpResponse, Result, Error};

// Thread imports
use std::thread;

// Standard file imports
use std::fs::{self, File};
use std::io::{Write, BufReader, BufRead};
use std::path::Path;
use std::path::PathBuf;

// Random number generator imports
use rand::seq::SliceRandom;
use rand::thread_rng;
 
// Process imports
use std::process::Command;
use std::env;

#[derive(Default)]
pub struct AppState {
    pub is_dark_mode: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self { is_dark_mode: true }
    }

    pub fn toggle_dark_mode(&mut self) {
        self.is_dark_mode = !self.is_dark_mode;
    }
}

struct MainScreen {
    app_state: AppState,
    current_screen: Option<Box<dyn Screen>>,
}

impl Default for MainScreen {
    fn default() -> Self {
        Self {
            app_state: AppState::new(),
            current_screen: None,
        }
    }
}

impl MainScreen {
    fn name() -> &'static str {
        "Recipe Bot"
    }

    fn handle_dark_mode_toggle(&mut self) {
        self.app_state.toggle_dark_mode();
    }

    fn update(&mut self, ctx: &egui::Context) {
        ctx.set_pixels_per_point(5.0);
        let is_dark_mode = self.app_state.is_dark_mode;
        let background_color = if is_dark_mode {
            egui::Color32::from_rgb(30, 30, 30)
        } else {
            egui::Color32::WHITE
        };
        if let Some(screen) = &mut self.current_screen {
            if screen.wants_to_exit() {
                self.current_screen = None;
            } else {
                if let Some(new_screen) = screen.update(ctx, &mut self.app_state) {
                    self.current_screen = Some(new_screen);
                }
                return;
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, background_color);
            ui.vertical_centered(|ui| {
                ui.heading("Recipe Bot");

                if ui.button("Create Weekly Recipes").clicked() {
                    self.current_screen = Some(Box::new(CreateWeeklyRecipesScreen::default()));
                }

                if ui.button("Update and Restart").clicked() {
                    if let Err(e) = self.update_and_restart() {
                        eprintln!("Failed to update and restart: {}", e);
                    }
                }

                if ui.button("Create New Recipe - Manual Entry").clicked() {
                    self.current_screen = Some(Box::new(CreateRecipeManuallyScreen::default()));
                }

                if ui.button("Light/Dark Mode Toggle").clicked() {
                    self.handle_dark_mode_toggle();
                }

                // Update text color based on dark mode
                if is_dark_mode {
                    ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);
                } else {
                    ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                }
            });
        });
    }
    fn update_and_restart(&self) -> Result<(), Box<dyn std::error::Error>> {
        let current_exe = env::current_exe()?;

        // Pull from git
        Command::new("git")
            .args(&["pull", "origin", "main"]) // Adjust branch name if necessary
            .status()?;

        // Recompile the program
        Command::new("cargo")
            .args(&["build", "--release"])
            .status()?;

        // Restart the program
        Command::new(current_exe)
            .spawn()?;

        // Exit the current instance
        std::process::exit(0);

        Ok(())
    }
}

impl eframe::App for MainScreen {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame){
        self.update(ctx);
    }
}
trait Screen {
    fn update(&mut self, ctx: &egui::Context, app_state: &mut AppState) -> Option<Box<dyn Screen>>;
    fn wants_to_exit(&self) -> bool;
}

struct CreateWeeklyRecipesScreen{
    wants_to_exit: bool,
    recipes: Vec<String>,
    selected_recipes: Vec<String>,
    processing_message: String,
}

impl CreateWeeklyRecipesScreen {
    fn load_recipes() -> Vec<String> {
        let recipes_dir = Path::new("recipes/dinner");
        fs::read_dir(recipes_dir)
            .unwrap_or_else(|_| panic!("Failed to read recipes directory"))
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()? == "rec" {
                    Some(path.file_stem()?.to_string_lossy().into_owned())
                } 
                else {
                    None
                }
            })
            .collect()
    }
    fn randomize_all(&mut self) {
        let mut rng = thread_rng();
        for recipe in &mut self.selected_recipes {
            *recipe = self.recipes.choose(&mut rng).unwrap_or(&String::new()).clone();
        }
    }
    fn randomize_single(&mut self, idx: usize) {
        let mut rng = thread_rng();
        if let Some(recipe) = self.selected_recipes.get_mut(idx) {
            *recipe = self.recipes.choose(&mut rng).unwrap_or(&String::new()).clone();
        }
    }
    fn process_selected_recipes(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all("schedule")?;
        let mut process_ingredients = String::new();
        let mut process_schedule = String::new();
        let days = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

        for (i, recipe_name) in self.selected_recipes.iter().enumerate() {
            if recipe_name.is_empty(){
                continue;
            }
            let recipe_path = Path::new("recipes/dinner").join(format!("{}.rec",recipe_name));
            let dest_path = Path::new("schedule").join(format!("{}.rec", days[i]));
            fs::copy(&recipe_path, &dest_path)?;
            process_schedule.push_str(&format!("{}: {}\n", days[i], recipe_name));
            let file = File::open(&recipe_path)?;
            let reader = BufReader::new(file);
            let mut in_ingredients = false;
            for line in reader.lines() {
                let line = line?;
                if line.trim() == "Ingredients Start" {
                    in_ingredients = true;
                }
                else if line.trim() == "Ingredients End" {
                    in_ingredients = false;
                }
                else if in_ingredients{
                    process_ingredients.push_str(&line);
                    process_ingredients.push('\n');
                }
            }
        }
        let mut ingredients_file = File::create("schedule/ingredients.sup")?;
        ingredients_file.write_all(process_ingredients.as_bytes())?;
        let mut schedule_file = File::create("schedule/schedule.txt")?;
        schedule_file.write_all(process_schedule.as_bytes())?;

        Ok(())
    }
    fn clear_processing_message(&mut self) {
        self.processing_message.clear();
    }
}

impl Default for CreateWeeklyRecipesScreen {
    fn default() -> Self {
        let recipes = Self::load_recipes();
        Self {
            wants_to_exit: false,
            recipes: recipes.clone(),
            selected_recipes: vec![String::new(); 7],
            processing_message: String::new(),
        }
    }
}

impl Screen for CreateWeeklyRecipesScreen {
    fn update(&mut self, ctx: &egui::Context, app_state: &mut AppState) -> Option<Box<dyn Screen>> {
        ctx.set_pixels_per_point(5.0);

        let is_dark_mode = app_state.is_dark_mode;
        let background_color = if is_dark_mode {
            egui::Color32::from_rgb(30, 30, 30)
        } else {
            egui::Color32::WHITE
        };


        egui::CentralPanel::default().show(ctx, |ui| {
            ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, background_color);
            ui.vertical_centered(|ui| {
                ui.heading("Create Weekly Recipes Screen");

                let days = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

                for (i, day) in days.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 4.0);
                        ui.label(*day);
                        egui::ComboBox::from_id_source(format!("recipe_combo_{}", i))
                            .selected_text(&self.selected_recipes[i])
                            .show_ui(ui, |ui| {
                                for recipe in &self.recipes {
                                    ui.selectable_value(&mut self.selected_recipes[i], recipe.clone(), recipe);
                                }
                            });
                        if ui.button("🎲").clicked() {
                            self.randomize_single(i);
                        }
                    });
                }
                
                ui.add_space(10.0);

                ui.vertical_centered(|ui| {
                    if ui.button("Randomize All").clicked() {
                        self.randomize_all();
                    }
                });

                ui.vertical_centered(|ui| {
                    if ui.button("Process Selected Recipes").clicked() {
                        self.clear_processing_message();
                        match self.process_selected_recipes() {
                            Ok(_) => self.processing_message = "Processing completed successfully.".to_string(),
                            Err(e) => self.processing_message = format!("Error during processing: {}", e),
                        }
                    }
                });

                ui.vertical_centered(|ui| {
                    if ui.button("Back to Main Screen").clicked() {
                        self.clear_processing_message();
                        self.wants_to_exit = true;
                    }
                });
                ui.vertical_centered(|ui|{
                    if !self.processing_message.is_empty() {
                        ui.colored_label(
                            if self.processing_message.starts_with("Error") { egui::Color32::RED } else { egui::Color32::GREEN},
                            &self.processing_message
                        );
                    }
                });

                // Update text color based on dark mode
                if is_dark_mode {
                    ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);
                } else {
                    ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
                }
            });
        });

        None
    }

    fn wants_to_exit(&self) -> bool {
        self.wants_to_exit
    }
}

struct CreateRecipeManuallyScreen {
    wants_to_exit: bool,
    title: String,
    from: String,
    servings: String,
    prep_time: String,
    cook_time: String,
    total_time: String,
    ingredients: String,
    instructions: Vec<String>,
    processing_message: String,
}

impl Default for CreateRecipeManuallyScreen {
    fn default() -> Self {
        Self {
            wants_to_exit: false,
            title: String::new(),
            from: String::new(),
            servings: String::new(),
            prep_time: String::new(),
            cook_time: String::new(),
            total_time: String::new(),
            ingredients: String::new(),
            instructions: vec![String::new()],
            processing_message: String::new(),
        }
    }
}

impl Screen for CreateRecipeManuallyScreen {
    fn update(&mut self, ctx: &egui::Context, app_state: &mut AppState) -> Option<Box<dyn Screen>> {
        ctx.set_pixels_per_point(5.0);

        let is_dark_mode = app_state.is_dark_mode;
        let background_color = if is_dark_mode {
            egui::Color32::from_rgb(30, 30, 30)
        } else {
            egui::Color32::WHITE
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, background_color);
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Create Recipe Manually");

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Title:");
                        ui.text_edit_singleline(&mut self.title);
                    });

                    ui.horizontal(|ui| {
                        ui.label("From:");
                        ui.text_edit_singleline(&mut self.from);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Servings:");
                        ui.text_edit_singleline(&mut self.servings);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Prep Time:");
                        ui.text_edit_singleline(&mut self.prep_time);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Cook Time:");
                        ui.text_edit_singleline(&mut self.cook_time);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Total Time:");
                        ui.text_edit_singleline(&mut self.total_time);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Ingredients (comma separated):");
                        ui.text_edit_multiline(&mut self.ingredients);
                    });
                    ui.label("Instructions:");
                    let mut updates = Vec::new();
                    let mut instruction_to_remove: Option<usize> = None;
                    let mut instruction_to_add = false;

                    // Render instructions
                    for (idx, instruction) in self.instructions.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}.", idx + 1));
                            let mut instruction_text = instruction.clone();
                            if ui.text_edit_singleline(&mut instruction_text).changed() {
                                updates.push((idx, instruction_text));
                            }
                            if ui.button("-").clicked() && self.instructions.len() > 1 {
                                instruction_to_remove = Some(idx);
                            }
                        });
                    }

                    // Add new instruction button
                    if ui.button("Add Instruction").clicked() {
                        instruction_to_add = true;
                    }

                    // Apply changes
                    for (idx, instruction_text) in updates {
                        self.instructions[idx] = instruction_text;
                    }

                    if let Some(idx) = instruction_to_remove {
                        self.instructions.remove(idx);
                    }

                    if instruction_to_add {
                        self.instructions.push(String::new());
                    }

                    ui.add_space(10.0);

                    if ui.button("Save Recipe").clicked() {
                        if let Err(e) = self.save_recipe() {
                            self.processing_message = format!("Error saving recipe: {}", e);
                        } else {
                            self.processing_message = "Recipe saved successfully".to_string();
                        }
                    }

                    ui.add_space(10.0);

                    if ui.button("Back to Main Screen").clicked() {
                        self.wants_to_exit = true;
                    }

                    if !self.processing_message.is_empty() {
                        ui.colored_label(
                            if self.processing_message.starts_with("Error") { egui::Color32::RED } else { egui::Color32::GREEN },
                            &self.processing_message
                        );
                    }
                });
            });

            if is_dark_mode {
                ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);
            } else {
                ui.visuals_mut().override_text_color = Some(egui::Color32::BLACK);
            }
        });

        None
    }

    fn wants_to_exit(&self) -> bool {
        self.wants_to_exit
    }
}

impl CreateRecipeManuallyScreen {
    fn save_recipe(&self) -> Result<(), Box<dyn std::error::Error>> {
        let file_name = format!("recipes/generated/{}.rec", self.title.replace(" ", "_"));
        let mut file = File::create(file_name)?;

        writeln!(file, "Title\t{}", self.title)?;
        writeln!(file, "From\t{}", self.from)?;
        writeln!(file, "Servings\t{}", self.servings)?;
        writeln!(file, "Prep Time\t{}", self.prep_time)?;
        writeln!(file, "Cook Time\t{}", self.cook_time)?;
        writeln!(file, "Total Time\t{}", self.total_time)?;
        writeln!(file, "Ingredients Start")?;
        for ingredient in self.ingredients.split(',') {
            writeln!(file, "{}", ingredient.trim())?;
        }
        writeln!(file, "Ingredients End")?;
        writeln!(file, "Instructions Start")?;
        for instruction in &self.instructions {
            writeln!(file, "{}", instruction)?;
        }
        writeln!(file, "Instructions End")?;

        Ok(())
    }
}

#[get("/")]
async fn index() -> HttpResponse {
    HttpResponse::Ok().body(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Recipe Bot Web Server</title>
            <style>
                body {
                    font-family: Arial, sans-serif;
                    background-color: #f0f0f0;
                    margin: 0;
                    padding: 0;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    height: 100vh;
                }
                .container {
                    text-align: center;
                    background-color: #ffffff;
                    padding: 50px;
                    border-radius: 8px;
                    box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
                }
                h1 {
                    color: #333333;
                }
                .links {
                    margin-top: 20px;
                }
                .link-button {
                    display: inline-block;
                    margin: 10px;
                    padding: 15px 30px;
                    font-size: 16px;
                    color: #ffffff;
                    background-color: #007BFF;
                    border: none;
                    border-radius: 5px;
                    text-decoration: none;
                    transition: background-color 0.3s;
                }
                .link-button:hover {
                    backgorund-color: #0056B3;
                }
            </style>
        </head>
        <body>
            <div class="container">
                <h1>Welcome to Recipe Bot's Web Server</h1>
                <div class="links">
                    <a href="/schedule" class="link-button">Weekly Food Schedule</a>
                    <a href="/ingredients" class="link-button">Ingredients Needed</a>
                </div>
            </div>
        </body>
        </html>
        "#
    )
}

#[get("/schedule")]
async fn schedule() -> Result<HttpResponse> {
    let path = PathBuf::from("schedule/schedule.txt");
    if path.exists() {
        let contents = fs::read_to_string(path)?;
        let list_items: String = contents
            .lines()
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, ": ").collect();
                if parts.len() == 2 {
                    format!("<div class=\"day\"><h2>{}</h2> <p class=\"meal\">{}</p></div>", parts[0], parts[1])
                } else {
                    let remaining: String = parts.join(" ");
                    format!("<h2>{}</h2> <p class=\"meal\">{}</p>", parts[0], remaining)
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        Ok(HttpResponse::Ok().body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>Meal Schedule</title>
                <style>
                    body {{
                        font-family: Arial, sans-serif;
                        background-color: #f0f0f0;
                        margin: 0;
                        padding: 0;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        height: 100vh;
                    }}
                    .container {{
                        text-align: center;
                        background-color: #ffffff;
                        padding: 50px;
                        border-radius: 8px;
                        box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
                        max-width: 600px;
                        width: 100%;
                    }}
                    h1 {{
                        color: #333333;
                    }}
                    .schedule {{
                        margin-top: 20px;
                    }}
                    .day {{
                        margin: 10px 0;
                        padding: 15px;
                        background-color: #e9ecef;
                        border-radius: 5px;
                        box-shadow: 0 0 5px rgba(0, 0, 0, 0.1);
                    }}
                    .day h2 {{
                        margin: 0;
                        color: #007BFF;
                    }}
                    .meal {{
                        margin-top: 5px;
                        color: #555555;
                    }}
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>Weekly Meal Schedule</h1>
                    <div class="schedule">
                        {}
                    </div>
                </div>
            </body>
            </html>
            "#,
            list_items
        )))
    } else {
        Err(Error::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Schedule file not found"
        )))
    }
}

#[get("/ingredients")]
async fn ingredients() -> Result<HttpResponse> {
    let path = PathBuf::from("schedule/ingredients.sup");
    if path.exists() {
        let contents = fs::read_to_string(path)?;
        let list_items: String = contents
            .lines()
            .map(|line| format!("<p class=\"item\">{}</p>", line.trim()))
            .collect::<Vec<String>>()
            .join("\n");

        Ok(HttpResponse::Ok().body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>Ingredients</title>
                <style>
                    body {{
                        font-family: Arial, sans-serif;
                        background-color: #f0f0f0;
                        margin: 0;
                        padding: 0;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        height: 100vh;
                    }}
                    .container {{
                        text-align: center;
                        background-color: #ffffff;
                        padding: 50px;
                        border-radius: 8px;
                        box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
                        max-width: 600px;
                        width: 100%;
                    }}
                    h1 {{
                        color: #333333;
                    }}
                    .ingredients {{
                        margin-top: 20px;
                        text-align: left;
                        max-height: 400px;
                        overflow-y: auto;
                        padding-right: 10px; /* to avoid hiding the last item */
                    }}
                    .item {{
                        margin: 10px 0;
                        padding: 15px;
                        background-color: #e9ecef;
                        border-radius: 5px;
                        box-shadow: 0 0 5px rgba(0, 0, 0, 0.1);
                    }}
                    .copy-button {{
                        display: inline-block;
                        margin-top: 20px;
                        padding: 15px 30px;
                        font-size: 16px;
                        color: #ffffff;
                        background-color: #28a745;
                        border: none;
                        border-radius: 5px;
                        cursor: pointer;
                        transition: background-color 0.3s;
                    }}
                    .copy-button:hover {{
                        background-color: #218838;
                    }}
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>Ingredients List</h1>
                    <div class="ingredients" id="ingredients-list">
                        {}
                    </div>
                    <button class="copy-button" onclick="copyToClipboard()">Copy to Clipboard</button>
                </div>
                <script>
                    function copyToClipboard() {{
                        const ingredientsElement = document.getElementById('ingredients-list');
                        const ingredientsText = Array.from(ingredientsElement.getElementsByClassName('item'))
                            .map(item => item.innerText.trim()) // Remove extra whitespace
                            .join('\n'); // Use actual newline character

                        const container = document.createElement('textarea');
                        container.value = ingredientsText;
                        document.body.appendChild(container);
                        container.select();
                        document.execCommand('copy');
                        document.body.removeChild(container);
                        alert('Ingredients copied to clipboard!');
                    }}
                </script>
            </body>
            </html>
            "#,
            list_items
        )))
    } else {
        Err(Error::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Ingredients file not found"
        )))
    }
}

fn start_web_server() -> std::io::Result<()> {
    println!("Starting server at http://0.0.0.0:8080");
    let sys = actix_web::rt::System::new();
    sys.block_on(async {
        HttpServer::new(|| {
            ActixApp::new()
                .service(index)
                .service(schedule)
                .service(ingredients)
        })
        .bind("0.0.0.0:8080")?
        .run()
        .await
    })?;
    Ok(())
}

fn main() -> eframe::Result<()> {

    thread::spawn(|| {
        if let Err(e) = start_web_server() {
            eprintln!("Web server error: {}", e);
        }
    });
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size((400.0, 400.0)),
        ..eframe::NativeOptions::default()
    };

    eframe::run_native(
        MainScreen::name(),
        native_options,
        Box::new(|_cc: &CreationContext<'_>| -> Box<dyn eframe::App> {
            Box::new(MainScreen::default())
        }),
    )
}
