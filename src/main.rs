// GUI Imports
use eframe::{egui, CreationContext};

// Web server imports
use actix_web::{get, App as ActixApp, HttpServer, HttpResponse, Result, Error};

// Thread imports
use std::thread;

// Standard file imports
use std::fs::{self, File};
use std::io::{Write, BufReader, BufRead, BufWriter};
use std::path::Path;
use std::path::PathBuf;

// Random number generator imports
use rand::seq::SliceRandom;
use rand::thread_rng;
 
// Process imports
use std::process::Command;
use std::env;

// PDF Generation imports
use printpdf::*;

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

struct Recipe {
    title: String,
    from: String,
    servings: String,
    prep_time: String,
    cook_time: String,
    total_time: String,
    ingreds: Vec<String>,
    instructions: Vec<String>,
    notes: Vec<String>,
}

fn parse_recipe_file(file_path: &PathBuf) -> Result<Recipe, std::io::Error> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut recipe = Recipe {
        title: String::new(),
        from: String::new(),
        servings: String::new(),
        prep_time: String::new(),
        cook_time: String::new(),
        total_time: String::new(),
        ingreds: Vec::new(),
        instructions: Vec::new(),
        notes: Vec::new(),
    };

    let mut current_section = "";

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        if line.contains('\t') {
            let parts: Vec<&str> = line.splitn(2, '\t').collect();
            if parts.len() == 2 {
                match parts[0].trim() {
                    "Title" => recipe.title = parts[1].trim().to_string(),
                    "From" => recipe.from = parts[1].trim().to_string(),
                    "Servings" => recipe.servings = parts[1].trim().to_string(),
                    "Prep Time" => recipe.prep_time = parts[1].trim().to_string(),
                    "Cook Time" => recipe.cook_time = parts[1].trim().to_string(),
                    "Total Time" => recipe.total_time = parts[1].trim().to_string(),
                    _ => {}
                }
            }
        } else {
            match line.trim() {
                "Ingredients Start" => current_section = "Ingredients",
                "Ingredients End" => current_section = "",
                "Instructions Start" => current_section = "Instructions",
                "Instructions End" => current_section = "",
                "Notes Start" => current_section = "Notes",
                "Notes End" => current_section = "",
                _ => match current_section {
                    "Ingredients" => recipe.ingreds.push(line.trim().to_string()),
                    "Instructions" => recipe.instructions.push(line.trim().to_string()),
                    "Notes" => recipe.notes.push(line.trim().to_string()),
                    _ => {}
                },
            }
        }
    }

    Ok(recipe)
}

fn generate_recipe_pdf(recipe_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the recipe file
    let recipe = parse_recipe_file(recipe_path)?;

    // Create a new PDF document
    let (doc, page1, layer1) = PdfDocument::new(&recipe.title, Mm(210.0), Mm(297.0), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Use a built-in font
    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    // Create a struct to hold the mutable state
    struct State {
        y_position: f32,
        current_page: PdfPageIndex,
        current_layer: PdfLayerIndex,
    }

    let mut state = State {
        y_position: 280.0,
        current_page: page1,
        current_layer: layer1,
    };

    // Function to wrap text
    fn wrap_text(text: &str, font_size: f32, max_width: f32) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let space_width = font_size * 0.3; // Approximate space width

        for word in words {
            let word_width = word.len() as f32 * font_size * 0.6; // Approximate word width
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() as f32 * font_size * 0.6 + space_width + word_width <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        lines
    }

    // Helper function to add text
    let add_text = |text: &str, size: f32, x: f32, state: &mut State| {
        let max_width = 680.0; // Page width minus margins
        let wrapped_lines = wrap_text(text, size, max_width);

        for line in wrapped_lines {
            if state.y_position < 20.0 {
                // Create a new page
                let (new_page, new_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
                state.current_page = new_page;
                state.current_layer = new_layer;
                state.y_position = 280.0;
            }
            let layer = doc.get_page(state.current_page).get_layer(state.current_layer);
            layer.use_text(&line, size, Mm(x), Mm(state.y_position), &font);
            state.y_position -= size as f32 + 2.0; // Move down by font size plus a small gap
        }
    };

    // Add recipe details
    add_text(&recipe.title, 20.0, 10.0, &mut state);
    add_text(&format!("From: {}", recipe.from), 14.0, 10.0, &mut state);
    add_text(&format!("Servings: {}", recipe.servings), 14.0, 10.0, &mut state);
    add_text(&format!("Prep Time: {}", recipe.prep_time), 14.0, 10.0, &mut state);
    add_text(&format!("Cook Time: {}", recipe.cook_time), 14.0, 10.0, &mut state);
    add_text(&format!("Total Time: {}", recipe.total_time), 14.0, 10.0, &mut state);

    state.y_position -= 10.0; // Add some space

    // Add ingredients
    add_text("Ingredients:", 16.0, 10.0, &mut state);
    for ingredient in &recipe.ingreds {
        add_text(&format!("• {}", ingredient), 12.0, 15.0, &mut state);
    }

    state.y_position -= 10.0; // Add some space

    // Add instructions
    add_text("Instructions:", 16.0, 10.0, &mut state);
    for (idx, instruction) in recipe.instructions.iter().enumerate() {
        add_text(&format!("{}", instruction), 12.0, 15.0, &mut state);
    }

    state.y_position -= 10.0; // Add some space

    // Add notes if any
    if !recipe.notes.is_empty() {
        add_text("Notes:", 16.0, 10.0, &mut state);
        for note in &recipe.notes {
            add_text(&format!("{}", note), 12.0, 15.0, &mut state);
        }
    }

    // Save the PDF to a file
    let output_filename = format!("{}.pdf", recipe.title.replace(" ", "_"));
    let output_path = env::current_dir()?.join(&output_filename);
    let mut output_file = BufWriter::new(File::create(&output_path)?);
    doc.save(&mut output_file)?;

    println!("PDF saved to: {:?}", output_path);

    Ok(())
}

fn open_pdf(pdf_path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(&["/C", "start", "", pdf_path.to_str().unwrap()])
            .spawn()?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new("xdg-open")
            .arg(pdf_path)
            .spawn()?;
    }
    Ok(())
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
        ctx.set_pixels_per_point(2.0);
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

                if ui.button("View Recipe").clicked() {
                    self.current_screen = Some(Box::new(RecipeSelectionScreen::default()));
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
        ctx.set_pixels_per_point(2.0);

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
    notes: Vec<String>,
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
            notes: vec![String::new()],
            processing_message: String::new(),
        }
    }
}

impl Screen for CreateRecipeManuallyScreen {
    fn update(&mut self, ctx: &egui::Context, app_state: &mut AppState) -> Option<Box<dyn Screen>> {
        ctx.set_pixels_per_point(2.0);

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

                    ui.label("Notes:");
                    let mut note_updates = Vec::new();
                    let mut note_to_remove: Option<usize> = None;
                    let mut note_to_add = false;

                    // Render notes
                    for (idx, note) in self.notes.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}.", idx + 1));
                            let mut note_text = note.clone();
                            if ui.text_edit_singleline(&mut note_text).changed() {
                                note_updates.push((idx, note_text));
                            }
                            if ui.button("-").clicked() && self.notes.len() > 1 {
                                note_to_remove = Some(idx);
                            }
                        });
                    }

                    // Add new note button
                    if ui.button("Add Note").clicked() {
                        note_to_add = true;
                    }

                    // Apply changes to notes
                    for (idx, note_text) in note_updates {
                        self.notes[idx] = note_text;
                    }

                    if let Some(idx) = note_to_remove {
                        self.notes.remove(idx);
                    }

                    if note_to_add {
                        self.notes.push(String::new());
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
        writeln!(file, "Notes Start")?;
        for note in &self.notes {
            writeln!(file, "{}", note)?;
        }
        writeln!(file, "Notes End")?;

        Ok(())
    }
}

struct RecipeSelectionScreen {
    selected_recipe: Option<String>,
    recipes: Vec<String>,
    wants_to_exit: bool,
    processing_message: String,
}

impl Default for RecipeSelectionScreen {
    fn default() -> Self {
        Self {
            selected_recipe: None,
            recipes: Vec::new(),
            wants_to_exit: false,
            processing_message: String::new(),
        }
    }
}

impl RecipeSelectionScreen {
    fn load_recipes(&mut self) {
        self.recipes.clear();
        let directories = ["recipes/desert", "recipes/dinner", "recipes/sides"];
        for dir in &directories {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file() && path.extension().map_or(false, |ext| ext == "rec") {
                            if let Some(file_name) = path.file_stem() {
                                self.recipes.push(file_name.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
        self.recipes.sort();
    }

    fn get_recipe_path(&self, recipe_name: &str) -> PathBuf {
        let directories = ["recipes/desert", "recipes/dinner", "recipes/sides"];
        for dir in &directories {
            let path = Path::new(dir).join(format!("{}.rec", recipe_name));
            if path.exists() {
                return path;
            }
        }
        PathBuf::new() // Return an empty path if not found
    }
}

impl Screen for RecipeSelectionScreen {
    fn update(&mut self, ctx: &egui::Context, app_state: &mut AppState) -> Option<Box<dyn Screen>> {
        ctx.set_pixels_per_point(2.0);

        let is_dark_mode = app_state.is_dark_mode;
        let background_color = if is_dark_mode {
            egui::Color32::from_rgb(30, 30, 30)
        } else {
            egui::Color32::WHITE
        };

        if self.recipes.is_empty() {
            self.load_recipes();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.painter().rect_filled(ui.available_rect_before_wrap(), 0.0, background_color);
            
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Select Recipe to View");

                    ui.add_space(10.0);

                    // Center the combo box
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        egui::ComboBox::from_label("Recipe")
                            .width(200.0) // Set a fixed width for the combo box
                            .selected_text(self.selected_recipe.clone().unwrap_or_else(|| "Select a recipe".to_string()))
                            .show_ui(ui, |ui| {
                                for recipe in &self.recipes {
                                    ui.selectable_value(&mut self.selected_recipe, Some(recipe.clone()), recipe);
                                }
                            });
                    });

                    ui.add_space(10.0);

                    if let Some(selected_recipe) = &self.selected_recipe {
                        if ui.button("View Recipe").clicked() {
                            let recipe_path = self.get_recipe_path(selected_recipe);
                            if recipe_path.exists() {
                                match parse_recipe_file(&recipe_path) {
                                    Ok(recipe) => {
                                        self.processing_message = format!("Recipe: {}\n\nFrom: {}\n\nServings: {}\n\nPrep Time: {}\nCook Time: {}\nTotal Time: {}\n\nIngredients:\n{}\n\nInstructions:\n{}\n\nNotes:\n{}",
                                            recipe.title,
                                            recipe.from,
                                            recipe.servings,
                                            recipe.prep_time,
                                            recipe.cook_time,
                                            recipe.total_time,
                                            recipe.ingreds.join("\n"),
                                            recipe.instructions.join("\n"),
                                            recipe.notes.join("\n")
                                        );
                                    },
                                    Err(e) => {
                                        self.processing_message = format!("Error reading recipe: {}", e);
                                    }
                                }
                            } else {
                                self.processing_message = "Recipe file not found".to_string();
                            }
                        }

                        if ui.button("Generate PDF").clicked() {
                            let recipe_path = self.get_recipe_path(selected_recipe);
                            if recipe_path.exists() {
                                match parse_recipe_file(&recipe_path) {
                                    Ok(recipe) => {
                                        if let Err(e) = generate_recipe_pdf(&recipe_path) {
                                            self.processing_message = format!("Error generating PDF: {}", e);
                                        } else {
                                            let pdf_filename = format!("{}.pdf", recipe.title.replace(" ", "_"));
                                            let pdf_path = env::current_dir().unwrap().join(&pdf_filename);
                                            if let Err(e) = open_pdf(&pdf_path) {
                                                self.processing_message = format!("Error opening PDF: {}", e);
                                            } else {
                                                self.processing_message = "PDF generated and opened successfully".to_string();
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        self.processing_message = format!("Error parsing recipe: {}", e);
                                    }
                                }
                            } else {
                                self.processing_message = "Recipe file not found".to_string();
                            }
                        }
                    }

                    ui.add_space(10.0);

                    if ui.button("Back to Main Screen").clicked() {
                        self.wants_to_exit = true;
                    }

                    ui.add_space(10.0);

                    if !self.processing_message.is_empty() {
                        ui.label(&self.processing_message);
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

#[derive(Debug, Clone)]
enum Message {
    RecipeSelected(PathBuf),
    GeneratePDF,
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
