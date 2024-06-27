// GUI Imports
use eframe::{egui, CreationContext};

// Web server imports
use actix_web::{get, App as ActixApp, HttpServer, HttpResponse, Result, Error};

// Web scraping imports
use reqwest::blocking::get;
use scraper::{Html, Selector};

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

struct RecipeDetails {
    title: String,
    from: String,
    servings: String,
    prep_time: String,
    cook_time: String,
    total_time: String,
    ingreds: Vec<String>,
    instructions: Vec<String>,
}

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

                if ui.button("Check/Edit Current Stock").clicked() {
                    // Handle button click
                }

                if ui.button("Create New Recipe - Manual").clicked() {
                    // Handle button click
                }

                if ui.button("Create New Recipe - From Link").clicked() {
                    self.current_screen = Some(Box::new(CreateRecipeFromLinkScreen::default()));
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

struct CreateRecipeFromLinkScreen{
    wants_to_exit: bool,
    processing_message: String,
    url: String,
    recipe_details: Option<RecipeDetails>,
}

impl CreateRecipeFromLinkScreen {
    fn clear_processing_message(&mut self) {
        self.processing_message.clear();
    }
    fn clear_url_string(&mut self) {
        self.url.clear();
    }
    fn scrape_recipe_details(&mut self) -> Result<RecipeDetails, Box<dyn std::error::Error>> {
        let res = get(&self.url)?;
        let body = res.text()?;
        let document = Html::parse_document(&body);

        fn find_text(document: &Html, selector: &str, text: &str) -> Option<String> {
            let selector = Selector::parse(selector).unwrap();
            document.select(&selector)
                .find(|element| element.inner_html().to_lowercase().contains(text))
                .map(|element| element.inner_html())
        }

        let title = document.select(&Selector::parse("title").unwrap())
            .next().map_or("Unknown".to_string(), |el| el.inner_html());

        let from = find_text(&document, "meta[name='author'], .author, .byline", "author")
            .unwrap_or("Unknown".to_string());

        let servings = find_text(&document, "*", "servings").or_else(|| find_text(&document, "*", "yield"))
            .unwrap_or("Unknown".to_string());

        let prep_time = find_text(&document, "*", "prep time")
            .unwrap_or("Unknown".to_string());

        let cook_time = find_text(&document, "*", "cook time")
            .unwrap_or("Unknown".to_string());

        let total_time = if prep_time != "Unknown" && cook_time != "Unknown" {
            format!("{} + {}", prep_time, cook_time)
        } else {
            "Unknown".to_string()
        };

        let ingreds = document.select(&Selector::parse("*").unwrap())
            .skip_while(|element| !element.inner_html().to_lowercase().contains("ingredients"))
            .skip(1)
            .take_while(|element| !element.inner_html().to_lowercase().contains("instructions"))
            .map(|element| element.inner_html().trim().to_string())
            .collect();

        let instructions = document.select(&Selector::parse("*").unwrap())
            .skip_while(|element| !element.inner_html().to_lowercase().contains("instructions"))
            .skip(1)
            .take_while(|element| !element.inner_html().to_lowercase().contains("end"))
            .map(|element| element.inner_html().trim().to_string())
            .collect::<Vec<String>>()
            .iter().enumerate()
            .map(|(i, instruction)| if instruction.starts_with(char::is_numeric) {
                instruction.clone()
            } else {
                format!("{}. {}", i + 1, instruction)
            })
            .collect();

        Ok(RecipeDetails {
            title,
            from,
            servings,
            prep_time,
            cook_time,
            total_time,
            ingreds,
            instructions,
        })
    }
    fn write_recipe_to_file(&self, recipe: &RecipeDetails) -> Result<(), Box<dyn std::error::Error>> {
        let file_name = format!("{}.rec", recipe.title.replace(" ", "_"));
        let mut file = File::create(file_name)?;

        writeln!(file, "Title\t{}", recipe.title)?;
        writeln!(file, "From\t{}", recipe.from)?;
        writeln!(file, "Servings\t{}", recipe.servings)?;
        writeln!(file, "Prep Time\t{}", recipe.prep_time)?;
        writeln!(file, "Cook Time\t{}", recipe.cook_time)?;
        writeln!(file, "Total Time\t{}", recipe.total_time)?;
        writeln!(file, "Ingredients Start")?;
        for ingredient in &recipe.ingreds {
            writeln!(file, "{}", ingredient)?;
        }
        writeln!(file, "Ingredients End")?;
        writeln!(file, "Instructions Start")?;
        for instruction in &recipe.instructions {
            writeln!(file, "{}", instruction)?;
        }
        writeln!(file, "Instructions End")?;

        Ok(())
    }
}

impl Default for CreateRecipeFromLinkScreen {
    fn default() -> Self {
        Self { 
            wants_to_exit: false,
            processing_message: String::new(),
            url: String::new(),
            recipe_details: None,
        }
    }
}

impl Screen for CreateRecipeFromLinkScreen {
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
                ui.heading("Create Recipe From Link");

                ui.horizontal(|ui|{
                    ui.add_space(ui.available_width() / 3.5);
                    ui.label("URL:");
                    ui.text_edit_singleline(&mut self.url);
                });
                ui.add_space(10.0);
                ui.vertical_centered(|ui|{
                    if ui.button("Generate").clicked() {
                        match self.scrape_recipe_details() {
                            Ok(recipe) => {
                                self.processing_message = "Recipe scraped successfully.".to_string();
                                self.write_recipe_to_file(&recipe).unwrap_or_else(|e| {
                                    self.processing_message = format!("Error writing to file: {}", e);
                                });
                            }
                            Err(e) =>{
                                self.processing_message = format!("Error scraping recipe: {}",e);
                            }
                        }
                    }
                });

                ui.vertical_centered(|ui| {
                    if ui.button("Back to Main Screen").clicked() {
                        self.clear_processing_message();
                        self.clear_url_string();
                        self.wants_to_exit = true;
                    }
                });
                ui.add_space(10.0);
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

#[get("/")]
async fn index() -> HttpResponse {
    HttpResponse::Ok().body(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Recipe Bot Server</title>
        </head>
        <body>
            <h1>Recipe Bot Server</h1>
            <ul>
                <li><a href="/schedule">View Schedule</a></li>
                <li><a href="/ingredients">View Ingredients</a></li>
            </ul>
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
                    format!("<li><b>{}:</b> {}</li>", parts[0], parts[1])
                } else {
                    format!("<li>{}</li>",line)
                }
            })
            .collect();
        Ok(HttpResponse::Ok().body(format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Recipe Schedule</title>
            </head>
            <body>
                <h1>Recipe Schedule</h1>
                <ul>{}</ul>
                <a href="/">Back</a>
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
        Ok(HttpResponse::Ok().body(format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Ingredients</title>
            </head>
            <body>
                <h1>Ingredients</h1>
                <pre>{}</pre>
                <a href="/">Back</a>
            </body>
            </html>
            "#,
            contents
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
