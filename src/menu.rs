use crate::course_api::*;
use crate::reqwest_get_ignore_ssl;
use crate::scrape_descriptions::*;
use crate::School::*;
use std::collections::HashMap;

use ::serde::*;
use chrono::*;
use escaper::*;
use log::info;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub struct SchoolMenu {
    school: School,
    cafes: Vec<Cafe>,
}

impl SchoolMenu {
    pub fn new(school: School) -> Self {
        Self {
            school,
            cafes: Vec::new(),
        }
    }

    pub fn get_cafes(&self) -> &Vec<Cafe> {
        &self.cafes
    }

    pub fn add_cafe(&mut self, cafe: Cafe) {
        self.cafes.push(cafe);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub struct Cafe {
    name: String,
    description: String,
    day_menus: Vec<DayMenu>,
    to_go_items: Vec<Meal>,
}

impl Cafe {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            day_menus: Vec::new(),
            to_go_items: Vec::new(),
        }
    }

    pub fn add_menus(&mut self, menus: Vec<DayMenu>) {
        self.day_menus.extend(menus);
    }

    pub fn add_to_go_meals(&mut self, meals: Vec<Meal>) {
        self.to_go_items.extend(meals);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub struct DayMenu {
    date: String,
    menus: Vec<Menu>,
}

impl DayMenu {
    pub fn new(date: String) -> Self {
        Self {
            date,
            menus: Vec::new(),
        }
    }

    pub fn add_menus(&mut self, menus: Vec<Menu>) {
        self.menus.extend(menus);
    }

    pub fn add_menu(&mut self, menu: Menu) {
        self.menus.push(menu);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub struct Menu {
    date: String,
    description: String,
    time_slot: MenuTime,
    time_opens: String,
    time_closes: String,
    stations: Vec<Station>,
    lat: u64,  // Multiplied by 1e7
    long: u64, // Multiplied by 1e7
    notes: String,
}

impl Menu {
    pub fn create_base_menu(date: String, description: String, lat: u64, long: u64) -> Self {
        Self {
            date,
            description,
            time_slot: MenuTime::NA,
            time_opens: "".to_string(),
            time_closes: "".to_string(),
            stations: Vec::new(),
            lat,
            long,
            notes: "".to_string(),
        }
    }

    pub fn create_base_menus(
        num: usize,
        date: &NaiveDate,
        description: String,
        lat: u64,
        long: u64,
    ) -> Vec<Self> {
        let mut menus: Vec<Self> = Vec::new();
        let mut iter_date = date.clone();

        for _ in 0..num {
            let menu = Self::create_base_menu(
                iter_date.format("%Y-%m-%d").to_string(),
                description.clone(),
                lat,
                long,
            );
            menus.push(menu);
            // Increment date
            iter_date = iter_date.succ();
        }
        menus
    }

    pub fn parse_set_timeslot(&mut self, time_slot: &str) {
        match time_slot.to_lowercase().as_str() {
            "breakfast" => self.time_slot = MenuTime::Breakfast,
            "lunch" => self.time_slot = MenuTime::Lunch,
            "dinner" => self.time_slot = MenuTime::Dinner,
            "brunch" | "continuous dining am" => self.time_slot = MenuTime::Brunch,
            "late night" | "night" => self.time_slot = MenuTime::Night,
            _ => self.time_slot = MenuTime::NA,
        }
    }

    pub fn set_start_time(&mut self, time_opens: &str) {
        self.time_opens = time_opens.to_string();
    }

    pub fn set_end_time(&mut self, time_closes: &str) {
        self.time_closes = time_closes.to_string();
    }

    pub fn set_notes(&mut self, notes: &str) {
        self.notes = notes.to_string();
    }

    pub fn add_station(&mut self, station: Station) {
        self.stations.push(station);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Station {
    name: String,
    notes: String,
    meals: Vec<Meal>,
}

impl Station {
    pub fn new(name: String, notes: String) -> Self {
        Self {
            name,
            notes,
            meals: Vec::new(),
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn add_meals(&mut self, meals: Vec<Meal>) {
        self.meals.extend(meals);
    }

    pub fn add_meal(&mut self, meal: Meal) {
        self.meals.push(meal);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub struct Meal {
    name: String,
    notes: String,
    dietary_options: Vec<DietaryOption>,
    cost: Option<u64>, // times 100 to enable serialization
}

impl Meal {
    pub fn new(name: String, notes: String) -> Self {
        Self {
            name,
            notes,
            dietary_options: Vec::new(),
            cost: None,
        }
    }

    pub fn add_dietary_options(&mut self, dietary_options: Vec<DietaryOption>) {
        self.dietary_options.extend(dietary_options);
    }

    pub fn add_dietary_option(&mut self, dietary_option: DietaryOption) {
        self.dietary_options.push(dietary_option);
    }

    pub fn set_cost(&mut self, cost: u64) {
        self.cost = Some(cost);
    }

    pub fn from_cafebonappetit_value(item: &serde_json::Value) -> (&str, Self) {
        let id = item.get("id").unwrap().as_str().unwrap().trim();
        let name = item.get("label").unwrap().to_string().replace("\"", "");
        let description = item
            .get("description")
            .unwrap()
            .to_string()
            .replace("\n", "");

        let notes = pretty_parse_html(&description);

        let price_temp = item.get("price").unwrap().as_f64().unwrap_or(0.);

        let cost: Option<u64> = if price_temp == 0. {
            None
        } else {
            Some((price_temp * 100.0) as u64)
        };

        let dietary_options = item.get("cor_icon").unwrap().as_object();

        let dietary_options = if let Some(dietary_options) = dietary_options {
            DietaryOption::from_cafebonappetit_values(
                &dietary_options
                    .into_iter()
                    .map(|(_, v)| v.clone())
                    .collect(),
            )
        } else {
            Vec::new()
        };

        (
            id,
            Meal {
                name,
                notes,
                cost,
                dietary_options,
            },
        )
    }
    
    pub fn from_eatec_recipes(recipes: &Vec<Value>) -> Vec<Self> {
        let mut meals = Vec::new();

        for meal in recipes {
            meals.push(Meal::from_eatec_value_single(&meal));
        }

        meals
    }

    pub fn from_eatec_value_single(json: &Value) -> Self {
        let name = json["@shortName"].as_str().unwrap().to_string();
        let notes = json["@description"].as_str().unwrap().to_string();

        let dietary_options = json["dietaryChoices"]["dietaryChoice"].as_array().unwrap();
        let allergens = json["allergens"]["allergen"].as_array().unwrap();

        let mut dietary_options = DietaryOption::from_eatec_values(&dietary_options);
        dietary_options.extend(DietaryOption::from_eatec_values(&allergens));

        Self {
            name,
            notes,
            cost: None,
            dietary_options,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub struct DietaryOption {
    food: FoodIngredient,
    contains: bool,
}

impl DietaryOption {
    pub fn new(food: FoodIngredient, contains: bool) -> Self {
        Self { food, contains }
    }

    pub fn from_cafebonappetit_values(values: &Vec<Value>) -> Vec<DietaryOption> {
        let mut dietary_options: Vec<DietaryOption> = Vec::new();

        for value in values {
            let value_str = value.as_str().unwrap();

            let food = FoodIngredient::parse_from_cafebonappetit(value_str);

            let without = value_str.contains("without");

            let dietary_option = DietaryOption::new(food, !without);

            dietary_options.push(dietary_option);
        }

        dietary_options
    }

    pub fn from_sodexomyway_values(values: &Vec<Value>) -> Vec<DietaryOption> {
        let mut dietary_options: Vec<DietaryOption> = Vec::new();

        for value in values {
            let name = value.get("name").unwrap().as_str().unwrap();
            let contains = value.get("contains").unwrap().as_str().unwrap();

            let contains = contains == "true";

            let food = FoodIngredient::parse_from_sodexomyway(name);

            let dietary_option = DietaryOption::new(food, contains);

            dietary_options.push(dietary_option);
        }

        dietary_options
    }

    pub fn from_eatec_values(values: &Vec<Value>) -> Vec<DietaryOption> {
        let mut dietary_options: Vec<DietaryOption> = Vec::new();

        for value in values {
            let name = value.get("@id").unwrap().as_str().unwrap();
            let contains = value.get("#text").unwrap().as_str().unwrap();

            let contains = contains == "Yes";

            let food = FoodIngredient::parse_from_eatec(name);

            let dietary_option = DietaryOption::new(food, contains);

            dietary_options.push(dietary_option);
        }

        dietary_options
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub enum FoodIngredient {
    Vegan,
    Vegetarian,
    Gluten,
    PlantBased,
    Organic,
    Dairy,
    Treenuts,
    Peanut,
    Eggs,
    Soybean,
    Fish,
    Shellfish,
    Halal,
    Other(String),
}

impl FoodIngredient {
    pub fn parse_from_cafebonappetit(text: &str) -> Self {
        match text {
            "Vegan" => Self::Vegan,
            "Vegetarian" => Self::Vegetarian,
            "Made without Gluten-Containing Ingredients" => Self::Gluten,
            "plant-based" => Self::PlantBased,
            "organic" => Self::Organic,
            "dairy" => Self::Dairy,
            "treenuts" => Self::Treenuts,
            "peanut" => Self::Peanut,
            "eggs" => Self::Eggs,
            "soybean" => Self::Soybean,
            "fish" => Self::Fish,
            "shellfish" => Self::Shellfish,
            "Halal" => Self::Halal,
            _ => Self::Other(text.to_string()),
        }
    }

    pub fn parse_from_sodexomyway(text: &str) -> Self {
        match text {
            "Vegan" => Self::Vegan,
            "Vegetarian" => Self::Vegetarian,
            "Gluten" => Self::Gluten,
            "Plant-based" => Self::PlantBased,
            "Organic" => Self::Organic,
            "Milk" => Self::Dairy,
            "Treenuts" => Self::Treenuts,
            "Peanuts" => Self::Peanut,
            "Eggs" => Self::Eggs,
            "Soybean" => Self::Soybean,
            "Fish" => Self::Fish,
            "Shellfish" => Self::Shellfish,
            "Halal" => Self::Halal,
            _ => Self::Other(text.to_string()),
        }
    }

    pub fn parse_from_eatec(text: &str) -> Self {
        match text {
            "Vegan" => Self::Vegan,
            "Vegetarian" => Self::Vegetarian,
            "Gluten Free" => Self::Gluten,
            "Plant-based" => Self::PlantBased,
            "Organic" => Self::Organic,
            "Milk" => Self::Dairy,
            "Tree Nut" => Self::Treenuts,
            "Peanut" => Self::Peanut,
            "Egg" => Self::Eggs,
            "Soybean" => Self::Soybean,
            "Fish" => Self::Fish,
            "Shellfish" => Self::Shellfish,
            "Halal" => Self::Halal,
            _ => Self::Other(text.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub enum MenuTime {
    Breakfast,
    Lunch,
    Dinner,
    Brunch,
    Night,
    Day,
    NA,
}

/*
Pitzer: ✅
    RATES: https://www.pitzer.edu/student-life/meal-plans-claremont-cash/
    McConnell: https://pitzer.cafebonappetit.com/cafe/mcconnell-bistro/{yyyy-mm-dd} ID: 219
    Pit Stop: https://pitzer.cafebonappetit.com/cafe/the-pit-stop-cafe/{yyyy-mm-dd} ID: 220
    Shakedown: https://pitzer.cafebonappetit.com/cafe/shakedown/{yyyy-mm-dd}
HMC: ✅
    RATES: https://www.hmc.edu/bao/dining-services/
    Hoch: https://menus.sodexomyway.com/BiteMenu/Menu?menuId=15258&locationId=13147001&whereami=http://hmc.sodexomyway.com/dining-near-me/hoch-shanahan-dining-commons#
    Jay's Place: https://content-service.sodexomyway.com/media/Jays%20Place%20Menu_tcm316-23062.pdf?url=https://hmc.sodexomyway.com/
    Cafe: Starbucks items
Scripps: ✅
    RATES:
    Mallot: https://scripps.cafebonappetit.com/cafe/malott-dining-commons/{yyyy-mm-dd}/ ID: 2253
    Motley: TBD
CMC: ✅
    RATES:
    Collins: https://collins-cmc.cafebonappetit.com/cafe/collins/{yyyy-mm-dd} ID: 50
    The Hub: https://collins-cmc.cafebonappetit.com/cafe/the-hub-grill/{yyyy-mm-dd} ID: 51, 52
    Anthenaeum: https://www.cmc.edu/athenaeum/weekly-menu
Pomona: ✅
    RATES: https://www.pomona.edu/administration/dining/meal-plans
    Frank: https://www.pomona.edu/administration/dining/menus/frank
    Frary: https://www.pomona.edu/administration/dining/menus/frary
    Oldenborg: https://www.pomona.edu/administration/dining/menus/oldenborg
    COOP: https://www.pomona.edu/administration/campus-center/coop-fountain/menu
KGI:
    Cafe: ID: 1525

CGU:
    Hagelbargers: https://cgu.cafebonappetit.com/cafe/hagelbargers-cafe/{yyyy-mm-dd}

TCCS:
    Honnold Libaray Cafe: ID: 1523
    ACC Cafe: ID: 1524
*/

// Cafe IDs or URLs
const MCCONNELL: &str = "219";
const PIT_STOP: &str = "220";
const SHAKEDOWN: &str = "NA";
const MALLOT: &str = "2253";
const COLLINS: &str = "50";
const HUB: &str = "51";
const HUB_1: &str = "52";
const HOCH: &str = "https://menus.sodexomyway.com/BiteMenu/MenuOnly?menuId=15258&locationId=13147001&whereami=http://hmc.sodexomyway.com/dining-near-me/hoch-shanahan-dining-commons";
const FRANK: &str = "https://my.pomona.edu/eatec/Frank.json";
const FRARY: &str = "https://my.pomona.edu/eatec/Frary.json";
const OLDENBORG: &str = "https://my.pomona.edu/eatec/Oldenborg.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuError {
    InvalidDate,
    InvalidCafe,
    ErrorFetchingURL,
    ErrorParsingJSON,
    ErrorParsingMenu,
}

pub async fn get_seven_day_menus() -> Result<HashMap<School, SchoolMenu>, MenuError>
{
    let today = Local::today().naive_local();

    // Create a HashMap to store the menus
    let mut menus = HashMap::new();

    // Get the menus for each school
    info!("=== Pomona Menus ===");
    let pomona_menus = get_pomona_menus(today, 7).await?;

    info!("=== HMC Menus ===");
    let hmc_menus = get_hmc_menus(today, 7).await?;

    info!("=== Pitzer Menus ===");
    let pitzer_menus = get_pitzer_menus(today, 7).await?;

    info!("=== Scripps Menus ===");
    let scripps_menus = get_scripps_menus(today, 7).await?;

    info!("=== CMC Menus ===");
    let cmc_menus = get_cmc_menus(today, 7).await?;


    // Add the menus to the HashMap
    menus.insert(Pomona, pomona_menus);
    menus.insert(Pitzer, pitzer_menus);
    menus.insert(Scripps, scripps_menus);
    menus.insert(ClaremontMckenna, cmc_menus);
    menus.insert(HarveyMudd, hmc_menus);
    // Return the HashMap

    Ok(menus)
}

pub async fn get_hmc_menus(
    start_date: NaiveDate,
    days_to_get: usize,
) -> Result<SchoolMenu, MenuError> {
    let mut school_menu = SchoolMenu::new(HarveyMudd);

    // Get Hoch
    info!("Getting menus for Hoch");
    let mut hoch_cafe: Cafe = Cafe::new("Hoch Dining Hall".to_string(), "".to_string());
    let (menus, to_go_meals) = get_sodexomyway_menus(
        days_to_get,
        HOCH,
        &start_date,
        (34.1057862 * 1_000_000_f64) as u64,
        (-117.7098119 * 1_000_000_f64) as u64,
    )
    .await?;
    hoch_cafe.add_menus(menus);

    school_menu.add_cafe(hoch_cafe);

    // Get Jay's Place
    info!("Getting menus for Jay's Place");
    let mut jays_place: Cafe = Cafe::new("Jay's Place".to_string(), "".to_string());

    Ok(school_menu)
}

pub async fn get_pitzer_menus(
    start_date: NaiveDate,
    days_to_get: usize,
) -> Result<SchoolMenu, MenuError> {
    let mut school_menu = SchoolMenu::new(Pitzer);

    // Get McConnell
    info!("Getting menus for McConnell");
    let mut mcconnell_cafe: Cafe = Cafe::new("McConnell Dining Hall".to_string(), "".to_string());
    let (menus, to_go_meals) =
        get_cafebonappetit_menus(days_to_get, MCCONNELL, &start_date).await?;
    mcconnell_cafe.add_menus(menus);
    mcconnell_cafe.add_to_go_meals(to_go_meals);

    school_menu.add_cafe(mcconnell_cafe);

    // Get Pit Stop
    info!("Getting menus for Pit Stop");
    let mut pit_stop_cafe: Cafe = Cafe::new("Pit Stop Cafe".to_string(), "".to_string());
    let (meals, to_go_meals) = get_cafebonappetit_menus(days_to_get, PIT_STOP, &start_date).await?;
    pit_stop_cafe.add_menus(meals);
    pit_stop_cafe.add_to_go_meals(to_go_meals);

    school_menu.add_cafe(pit_stop_cafe);

    Ok(school_menu)
}

pub async fn get_scripps_menus(
    start_date: NaiveDate,
    days_to_get: usize,
) -> Result<SchoolMenu, MenuError> {
    let mut school_menu = SchoolMenu::new(Scripps);

    // Get Mallot
    info!("Getting menus for Mallot");
    let mut mallot_cafe: Cafe = Cafe::new("Mallot Dining Hall".to_string(), "".to_string());
    let (menus, to_go_meals) = get_cafebonappetit_menus(days_to_get, MALLOT, &start_date).await?;
    mallot_cafe.add_menus(menus);
    mallot_cafe.add_to_go_meals(to_go_meals);

    school_menu.add_cafe(mallot_cafe);

    Ok(school_menu)
}

pub async fn get_cmc_menus(
    start_date: NaiveDate,
    days_to_get: usize,
) -> Result<SchoolMenu, MenuError> {
    let mut school_menu = SchoolMenu::new(ClaremontMckenna);

    // Get Collins
    info!("Getting menus for Collins");
    let mut collins_cafe: Cafe = Cafe::new("Collins Dining Hall".to_string(), "".to_string());
    let (menus, to_go_meals) = get_cafebonappetit_menus(days_to_get, COLLINS, &start_date).await?;
    collins_cafe.add_menus(menus);
    collins_cafe.add_to_go_meals(to_go_meals);

    school_menu.add_cafe(collins_cafe);

    // Get Hub
    info!("Getting menus for The Hub");
    let mut hub_cafe: Cafe = Cafe::new("The Hub".to_string(), "".to_string());
    let (menus, to_go_meals) = get_cafebonappetit_menus(days_to_get, HUB, &start_date).await?;
    hub_cafe.add_menus(menus);
    hub_cafe.add_to_go_meals(to_go_meals);

    school_menu.add_cafe(hub_cafe);

    Ok(school_menu)
}


pub async fn get_pomona_menus(start_date: NaiveDate, days_to_get: usize) -> Result<SchoolMenu, MenuError> {
    let mut school_menu = SchoolMenu::new(Pomona);

    // Get Frank
    info!("Getting menus for Frank");
    let mut frank_cafe: Cafe = Cafe::new("Frank Dining Hall".to_string(), "".to_string());
    let (menus, to_go_meals) = get_eatec_menu(days_to_get, FRANK, &start_date).await?;
    frank_cafe.add_menus(menus);
    frank_cafe.add_to_go_meals(to_go_meals);

    school_menu.add_cafe(frank_cafe);

    // Get Frary
    info!("Getting menus for Frary");
    let mut frary_cafe: Cafe = Cafe::new("Frary Dining Hall".to_string(), "".to_string());
    let (menus, to_go_meals) = get_eatec_menu(days_to_get, FRARY, &start_date).await?;
    frary_cafe.add_menus(menus);
    frary_cafe.add_to_go_meals(to_go_meals);

    school_menu.add_cafe(frary_cafe);

    // Get Oldenborg
    info!("Getting menus for Oldenborg");
    let mut oldenborg_cafe: Cafe = Cafe::new("Oldenborg Dining Hall".to_string(), "".to_string());
    let (menus, to_go_meals) = get_eatec_menu(days_to_get, OLDENBORG, &start_date).await?;
    oldenborg_cafe.add_menus(menus);
    oldenborg_cafe.add_to_go_meals(to_go_meals);

    school_menu.add_cafe(oldenborg_cafe);

    Ok(school_menu)
}

/// Function to get the menus from Cafebonappetit's API, provided a menu ID
pub async fn get_cafebonappetit_menus(
    num_days: usize,
    menu_id: &str,
    start_date: &NaiveDate,
) -> Result<(Vec<DayMenu>, Vec<Meal>), MenuError> {
    let url = format!(
        "https://legacy.cafebonappetit.com/api/2/cafes?cafe={}&date={}",
        menu_id, start_date
    );

    let response = reqwest_get_ignore_ssl(&url).await;

    if let Err(e) = response {
        return Err(MenuError::ErrorFetchingURL);
    }

    let response = response.unwrap();

    let json = response.json().await;

    if let Err(e) = json {
        return Err(MenuError::ErrorParsingJSON);
    }

    let json: serde_json::Value = json.unwrap();

    let json = json["cafes"][menu_id].clone();

    // Now, start getting the menu
    let description = json
        .get("description")
        .unwrap()
        .to_string()
        .replace("\n", "");

    let notes = pretty_parse_html(&description);

    let lat = (json.get("latitude").unwrap().to_string())
        .parse::<f64>()
        .unwrap_or(0.0);
    let long = (json.get("longitude").unwrap().to_string())
        .parse::<f64>()
        .unwrap_or(0.0);

    let lat: u64 = (lat * 1_000_000.0) as u64;
    let long: u64 = (long * 1_000_000.0) as u64;

    // Create url to build request off of
    let mut url = format!(
        "https://legacy.cafebonappetit.com/api/2/menus?cafe={}&date=",
        menu_id
    );

    let mut iter_date = start_date.clone();
    // Get the menu for each day
    for _ in 0..num_days {
        let str_date = iter_date.format("%Y-%m-%d").to_string();
        url = format!("{}{},", url, str_date);
        iter_date = iter_date.succ();
    }

    // Remove the last comma
    url.pop();

    // Get the menu
    let response = reqwest_get_ignore_ssl(&url).await;

    if let Err(e) = response {
        return Err(MenuError::ErrorFetchingURL);
    }

    let response = response.unwrap();

    let json = response.json().await;

    if let Err(e) = json {
        return Err(MenuError::ErrorParsingJSON);
    }

    let json: serde_json::Value = json.unwrap();

    // Get items to map off of
    let items = json.get("items").unwrap().as_object().unwrap();
    let mut meals: HashMap<String, Meal> = HashMap::new();

    // Parse all the items into meals
    for (_, item) in items {
        let (id, meal) = Meal::from_cafebonappetit_value(item);

        meals.insert(id.to_string(), meal);
    }

    // Parse all to-go items into meals
    let items = json.get("goitems").unwrap().as_object();
    let mut to_go_meals: Vec<Meal> = Vec::new();

    // Parse as an empty to-go items is given as an array,
    // not as an object
    if let Some(items) = items {
        for (_, item) in items {
            let (id, meal) = Meal::from_cafebonappetit_value(item);

            to_go_meals.push(meal);
        }
    }

    // Now onto the days
    let days = json.get("days").unwrap().as_array().unwrap();

    // Create the base seven menus
    let mut menus = Vec::new();
    let base_menu =
        Menu::create_base_menu(start_date.format("%Y-%m-%d").to_string(), notes, lat, long);

    // Get the menu for each day
    for (index, day) in days.iter().enumerate() {
        // Get parts of the day (breakfast, lunch, dinner)
        let dayparts = day["cafes"][menu_id]["dayparts"].as_array().unwrap()[0]
            .as_array()
            .unwrap()
            .clone();

        let mut day_menu = DayMenu::new(day["date"].as_str().unwrap().to_string());

        for part in dayparts {
            let mut new_menu = base_menu.clone();

            // Get time slot of the menu
            new_menu.parse_set_timeslot(part["label"].as_str().unwrap());

            // Get basic info
            new_menu.set_start_time(part["starttime"].as_str().unwrap());
            new_menu.set_end_time(part["endtime"].as_str().unwrap());
            new_menu.set_notes(part["message"].as_str().unwrap());

            // Now to the stations
            // Each station contains item ids that it can serve
            let stations = part["stations"].as_array().unwrap();

            for station in stations {
                let name = station["label"].to_string().replace("\"", "");
                let notes = station["note"].to_string();

                // Create station
                let mut station_menu = Station::new(name, notes);

                let items = station["items"].as_array().unwrap();

                for item in items {
                    let id = item.as_str().unwrap().trim();
                    let meal = meals.get(id);

                    let meal = meal.unwrap();

                    station_menu.add_meal(meal.clone());
                }

                // Add station to menu
                new_menu.add_station(station_menu);
            }

            // Add the menu to the list
            day_menu.add_menu(new_menu);
        }

        // Add the day menu to the list
        menus.push(day_menu);
    }

    Ok((menus, to_go_meals))
}

/// Function to scrape the menu from the SodexoMyWay website
/// This is made easier by the fact that the website stores the entire
/// menu as a single JS variable, which we can parse as JSON and
/// then convert to structs.
///
/// However, no matter the date given, the website will provide the
/// *weekly* menu, giving the week that that day is in. To solve this,
/// we can just request the next week's menu also, and parse the first
/// seven that we need.
pub async fn get_sodexomyway_menus(
    num_days: usize,
    menu_url: &str,
    start_date: &NaiveDate,
    lat: u64,
    long: u64,
) -> Result<(Vec<DayMenu>, Vec<Meal>), MenuError> {
    let first_half = format!("{}&startDate={}", menu_url, start_date.format("%m/%d/%Y"));

    let mut new_start = start_date.clone().succ();

    while new_start.weekday() != Weekday::Sun {
        new_start = new_start.succ();
    }

    let second_half = format!("{}&startDate={}", menu_url, new_start.format("%m/%d/%Y"));

    // Get both urls
    let first_response = reqwest_get_ignore_ssl(&first_half).await;
    let second_response = reqwest_get_ignore_ssl(&second_half).await;

    if let Err(e) = first_response {
        return Err(MenuError::ErrorFetchingURL);
    }

    if let Err(e) = second_response {
        return Err(MenuError::ErrorFetchingURL);
    }

    let first_response = first_response.unwrap();
    let second_response = second_response.unwrap();

    // Find line starting with "var nd = ["
    let first_text = first_response.text().await;
    let second_text = second_response.text().await;

    if let Err(e) = first_text {
        return Err(MenuError::ErrorFetchingURL);
    }

    if let Err(e) = second_text {
        return Err(MenuError::ErrorFetchingURL);
    }

    let first_text = first_text.unwrap();
    let second_text = second_text.unwrap();
    
    let mut first_var = first_text
        .split("\n")
        .find(|line| line.starts_with("<div id='nutData' data-schools='False' class='hide'>"))
        .unwrap()
        .to_string();
    let mut second_var = second_text
        .split("\n")
        .find(|line| line.starts_with("<div id='nutData' data-schools='False' class='hide'>"))
        .unwrap()
        .to_string();

    // Remove the "var nd = "
    first_var = first_var.replace("<div id='nutData' data-schools='False' class='hide'>", "").trim().to_string();
    second_var = second_var.replace("<div id='nutData' data-schools='False' class='hide'>", "").trim().to_string();

    // Remove the ;
    first_var = first_var.replace("</div>", "");
    second_var = second_var.replace("</div>", "");

    // Get the JSON from the string
    let first_json = serde_json::from_str::<serde_json::Value>(&first_var).unwrap();
    let second_json = serde_json::from_str::<serde_json::Value>(&second_var).unwrap();

    // Combine the two jsons into one.
    let mut combined: Vec<Value> = first_json.as_array().unwrap().to_vec();

    combined.extend(second_json.as_array().unwrap().iter().map(|x| x.clone()));

    let mut menus: Vec<DayMenu> = Vec::new();
    let to_go_items: Vec<Meal> = Vec::new();

    let mut combined_json_final = Vec::new();

    // Parse the json
    for (index, day) in combined.iter().enumerate() {
        let date = day["date"]
            .as_str()
            .unwrap()
            .to_string()
            .split("T")
            .collect::<Vec<&str>>()[0]
            .to_string();

        if date == start_date.format("%Y-%m-%d").to_string() {
            combined_json_final = combined.clone()[index..index + num_days].to_vec();
        } else {
            continue;
        }
    }

    // Now onto the days
    for day in combined_json_final {
        let date = day["date"]
            .as_str()
            .unwrap()
            .to_string()
            .split("T")
            .collect::<Vec<&str>>()[0]
            .to_string();

        let mut day_menu = DayMenu::new(date.clone());

        // Get the dayparts

        let dayparts = day["dayParts"].as_array().unwrap();
        let base_menu = Menu::create_base_menu(date.clone(), "".to_string(), lat, long);

        for part in dayparts {
            let mut new_menu = base_menu.clone();

            // Get time slot of the menu
            new_menu.parse_set_timeslot(part["dayPartName"].as_str().unwrap());

            let stations = part["courses"].as_array().unwrap();

            // Get basic info from first meal

            let first_meal = stations[0]["menuItems"].as_array().unwrap()[0].clone();

            new_menu.set_start_time(first_meal["startTime"].as_str().unwrap().split("T").nth(1).unwrap());
            new_menu.set_end_time(first_meal["endTime"].as_str().unwrap().split("T").nth(1).unwrap());

            for station in stations {
                let name = station["courseName"].as_str().unwrap().to_string();
                let notes = "".to_string();

                let mut station_menu = Station::new(name, notes);

                let items = station["menuItems"].as_array().unwrap();

                for item in items {
                    let name = item["formalName"].as_str().unwrap().to_string();
                    let notes = item["description"].as_str().unwrap_or("").to_string();
                    let price = item["priceWithTax"].as_f64().unwrap();
                    let dietary_options = item["allergens"].as_array().unwrap();
                    let mut dietary_options =
                        DietaryOption::from_sodexomyway_values(dietary_options);

                    if item["isVegan"].as_bool().unwrap() {
                        dietary_options.push(DietaryOption::new(FoodIngredient::Vegan, true));
                    }

                    if item["isVegetarian"].as_bool().unwrap() {
                        dietary_options.push(DietaryOption::new(FoodIngredient::Vegetarian, true));
                    }

                    if item["isPlantBased"].as_bool().unwrap() {
                        dietary_options.push(DietaryOption::new(FoodIngredient::PlantBased, true));
                    }

                    let mut meal = Meal::new(name, notes);

                    if price != 0. {
                        meal.set_cost((price * 100.) as u64);
                    }

                    station_menu.add_meal(meal.clone());
                }

                new_menu.add_station(station_menu);
            }

            day_menu.add_menu(new_menu);
        }

        menus.push(day_menu);
    }

    Ok((menus, to_go_items))
}

/// Function to get the menu from Pomona's Eatec menu.
/// It's served as a JSON file, taking no url paramaters,
/// so we cannot request a certain day/week of menus. Instead,
/// we just discard anything before the first menu that matches
/// the given start date.
pub async fn get_eatec_menu(num_days: usize, menu_url: &str, start_date: &NaiveDate) -> Result<(Vec<DayMenu>, Vec<Meal>), MenuError> {
    let mut end_date = start_date.clone();

    for _ in 0..num_days {
        end_date = end_date.succ();
    }

    // Get the json
    let response = reqwest_get_ignore_ssl(menu_url).await;

    if response.is_err() {
        return Err(MenuError::ErrorFetchingURL);
    }

    let response = response.unwrap();

    let text = response.text().await;

    if text.is_err() {
        return Err(MenuError::ErrorFetchingURL);
    }

    let mut text = text.unwrap();

    // Remove the "/**/ menuData("
    text = text.replace("/**/ menuData(", "");
    // Remove the ending ");"
    text.pop();
    text.pop();

    // Parse as JSON!
    let json = serde_json::from_str::<serde_json::Value>(&text).unwrap();

    let json_menu = json["EatecExchange"]["menu"].as_array().unwrap();

    // Get the first menu that matches the start date
    let start_index = json_menu.iter().position(
        |x| x["@servedate"].as_str().unwrap().to_string() == start_date.format("%Y%m%d").to_string(),
    ).unwrap();

    // Increment end date by one to completely cover the range
    let end_date_plus_one = end_date.succ();

    // Get the menu for the next num_days
    let end_index = json_menu.iter().skip(start_index.clone()).position(
        |x| x["@servedate"].as_str().unwrap().to_string() == end_date_plus_one.format("%Y%m%d").to_string(),
    ).unwrap_or(json_menu.len());


    let json_menu_final = json_menu[start_index..end_index].to_vec();

    // Now onto the days
    let mut current_date = start_date.clone();
    let mut current_station = json_menu_final[0]["@mealperiodname"].as_str().unwrap();

    let mut menus: Vec<DayMenu> = Vec::new();
    let mut day_menu: DayMenu = DayMenu::new(current_date.format("%Y-%m-%d").to_string());
    let mut menu: Menu = Menu::create_base_menu(current_date.format("%Y-%m-%d").to_string(), "".to_string(), 0, 0);

    for json_station in &json_menu_final {
        // If it's closed, skip the day
        if json_station["@mealperiodname"].as_str().unwrap() == "Closed" {
            current_date = current_date.succ();

            day_menu = DayMenu::new(current_date.format("%Y-%m-%d").to_string());
            menu = Menu::create_base_menu(current_date.format("%Y-%m-%d").to_string(), "".to_string(), 0, 0);

            println!("Closed on {}", current_date.format("%Y-%m-%d"));
            println!("{:?}", json_station);

            continue;
        }

        let recipes: Vec<Value> = json_station["recipes"]["recipe"].as_array().unwrap_or(
            {
                let mut empty_vec: Vec<Value> = Vec::new();

                empty_vec.push(json_station["recipes"]["recipe"].clone());

                &empty_vec.to_vec()
            }
        ).clone();

        // First, create the station
        let station_name = recipes[0]["@shortName"].as_str().unwrap();
        let station_notes = recipes[0]["@description"].as_str().unwrap();

        let mut station = Station::new(station_name.to_string(), station_notes.to_string());

        // Set name
        let name = recipes[0]["@category"].as_str().unwrap();
        station.set_name(name.to_string());

        // Add the meals
        station.add_meals(Meal::from_eatec_recipes(&recipes));

        // If it's a new date, push the last menu
        if json_station["@servedate"].as_str().unwrap().to_string() != current_date.format("%Y%m%d").to_string() {
            day_menu.add_menu(menu);

            menus.push(day_menu.clone());

            // Create new day menu & menu
            // NaiveDate::parse_from_str(station["@servedate"].as_str().unwrap(), "%Y%m%d").unwrap()
            current_date = current_date.succ();
            current_station = json_station["@mealperiodname"].as_str().unwrap();

            day_menu = DayMenu::new(current_date.format("%Y-%m-%d").to_string());

            menu = Menu::create_base_menu(current_date.format("%Y-%m-%d").to_string(), "".to_string(), 0, 0);

        } else if json_station["@mealperiodname"].as_str().unwrap() != current_station {
            if menu.time_slot != MenuTime::NA {
                day_menu.add_menu(menu);
            }

            current_station = json_station["@mealperiodname"].as_str().unwrap();

            menu = Menu::create_base_menu(current_date.format("%Y-%m-%d").to_string(), "".to_string(), 0, 0);
        }

        menu.parse_set_timeslot(current_station);
        menu.add_station(station);
    }

    // Add the last menu
    day_menu.add_menu(menu);
    menus.push(day_menu);

    Ok((menus, Vec::new()))
}
