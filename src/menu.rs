use std::collections::HashMap;
use crate::course_api::*;
use crate::School::*;
use log::info;
use ::serde::*;
use escaper::*;
use chrono::*;
use serde_json::Value;
use crate::reqwest_get_ignore_ssl;

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
    to_go_items: Vec<Meal>
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
    lat: u64, // Multiplied by 1e7
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

    pub fn create_base_menus(num: usize, date: &NaiveDate, description: String, lat: u64, long: u64) -> Vec<Self> {
        let mut menus: Vec<Self> = Vec::new();
        let mut iter_date = date.clone();

        for _ in 0..num {
            let menu = Self::create_base_menu(iter_date.format("%Y-%m-%d").to_string(), description.clone(), lat, long);
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
            "brunch" => self.time_slot = MenuTime::Brunch,
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
    cost: Option<u64> // times 100 to enable serialization
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
        let description = item.get("description").unwrap().to_string().replace("\n", "");
        
        // Remove HTML entities
        let notes = match decode_html(&description) {
            Err(_) => description,
            Ok(s) => s,
        };


        let price_temp = item.get("price").unwrap().as_f64().unwrap_or(0.);

        let cost: Option<u64> = if price_temp == 0. {
            None
        } else {
            Some((price_temp * 100.0) as u64)
        };

        let dietary_options = item.get("cor_icon").unwrap().as_object();
        
        let dietary_options = if let Some(dietary_options) = dietary_options {
            DietaryOption::from_values(&dietary_options.into_iter().map(|(_, v)| v.clone()).collect())
        } else {
            Vec::new()
        };

        (id, Meal {
            name,
            notes,
            cost,
            dietary_options,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

pub struct DietaryOption {
    food: FoodIngredient,
    contains: bool,
}

impl DietaryOption {
    pub fn new(food: FoodIngredient, contains: bool) -> Self {
        Self {
            food,
            contains,
        }
    }

    pub fn from_values(values: &Vec<Value>) -> Vec<DietaryOption> {
        let mut dietary_options: Vec<DietaryOption> = Vec::new();

        for value in values {
            let value_str = value.as_str().unwrap();

            let food = FoodIngredient::parse_from_cafebonappetit(value_str);

            let without = value_str.contains("without");

            let dietary_option = DietaryOption::new(food, !without);
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
HMC:
    RATES: https://www.hmc.edu/bao/dining-services/
    Hoch: https://menus.sodexomyway.com/BiteMenu/Menu?menuId=15258&locationId=13147001&whereami=http://hmc.sodexomyway.com/dining-near-me/hoch-shanahan-dining-commons#
    Jay's Place: https://content-service.sodexomyway.com/media/Jays%20Place%20Menu_tcm316-23062.pdf?url=https://hmc.sodexomyway.com/
    Cafe: Starbucks items
Scripps:
    RATES: 
    Mallot: https://scripps.cafebonappetit.com/cafe/malott-dining-commons/{yyyy-mm-dd}/ ID: 2253
    Motley: TBD
CMC:
    RATES:
    Collins: https://collins-cmc.cafebonappetit.com/cafe/collins/{yyyy-mm-dd} ID: 50
    The Hub: https://collins-cmc.cafebonappetit.com/cafe/the-hub-grill/{yyyy-mm-dd} ID: 51, 52
    Anthenaeum: https://www.cmc.edu/athenaeum/weekly-menu
Pomona:
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

// All Cafebonappetit IDs
const MCCONNELL: &str = "219";
const PIT_STOP: &str = "220";
const SHAKEDOWN: &str = "NA";
const MALLOT: &str = "2253";
const COLLINS: &str = "50";
const HUB: &str = "51";
const HUB_1: &str = "52";

pub async fn get_seven_day_menus() -> Result<HashMap<School, SchoolMenu>, Box<dyn std::error::Error>> {
    let today = Local::today().naive_local();

    // Create a HashMap to store the menus
    let mut menus = HashMap::new();

    // Get the menus for each school
    info!("=== Pitzer Menus ===");
    let pitzer_menus = get_pitzer_menus(today, 7).await?;

    info!("=== Scripps Menus ===");
    let scripps_menus = get_scripps_menus(today, 7).await?;
    
    info!("=== CMC Menus ===");
    let cmc_menus = get_cmc_menus(today, 7).await?;

    /*
    info!("Getting menus for HMC");
    let hmc_menus = get_hmc_menus(today, 7).await?;
    info!("Getting menus for Pomona");
    let pomona_menus = get_pomona_menus(today, 7).await?;
    */
    // Add the menus to the HashMap
    menus.insert(Pitzer, pitzer_menus);
    menus.insert(Scripps, scripps_menus);
    menus.insert(ClaremontMckenna, cmc_menus);
    /*
    menus.insert(HarveyMudd, hmc_menus);
    menus.insert(Pomona, pomona_menus);
    */
    // Return the HashMap

    Ok(menus)
}

pub async fn get_pitzer_menus(start_date: NaiveDate, days_to_get: usize) -> Result<SchoolMenu, Box<dyn std::error::Error>> {
    let mut menus = SchoolMenu::new(Pitzer);

    // Get McConnell
    info!("Getting menus for McConnell");
    let mut mcconnell_cafe: Cafe = Cafe::new("McConnell Dining Hall".to_string(), "".to_string());
    let (meals, to_go_meals) = get_cafebonappetit_menus(days_to_get, MCCONNELL, &start_date).await?;
    mcconnell_cafe.add_menus(meals);
    mcconnell_cafe.add_to_go_meals(to_go_meals);

    menus.add_cafe(mcconnell_cafe);

    // Get Pit Stop
    info!("Getting menus for Pit Stop");
    let mut pit_stop_cafe: Cafe = Cafe::new("Pit Stop Cafe".to_string(), "".to_string());
    let (meals, to_go_meals) = get_cafebonappetit_menus(days_to_get, PIT_STOP, &start_date).await?;
    pit_stop_cafe.add_menus(meals);
    pit_stop_cafe.add_to_go_meals(to_go_meals);

    menus.add_cafe(pit_stop_cafe);

    Ok(menus)
}

pub async fn get_scripps_menus(start_date: NaiveDate, days_to_get: usize) -> Result<SchoolMenu, Box<dyn std::error::Error>> {
    let mut menus = SchoolMenu::new(Scripps);

    // Get Mallot
    info!("Getting menus for Mallot");
    let mut mallot_cafe: Cafe = Cafe::new("Mallot Dining Hall".to_string(), "".to_string());
    let (meals, to_go_meals) = get_cafebonappetit_menus(days_to_get, MALLOT, &start_date).await?;
    mallot_cafe.add_menus(meals);
    mallot_cafe.add_to_go_meals(to_go_meals);

    menus.add_cafe(mallot_cafe);

    Ok(menus)
}

pub async fn get_cmc_menus(start_date: NaiveDate, days_to_get: usize) -> Result<SchoolMenu, Box<dyn std::error::Error>> {
    let mut menus = SchoolMenu::new(Scripps);

    // Get Collins
    info!("Getting menus for Collins");
    let mut collins_cafe: Cafe = Cafe::new("Collins Dining Hall".to_string(), "".to_string());
    let (meals, to_go_meals) = get_cafebonappetit_menus(days_to_get, COLLINS, &start_date).await?;
    collins_cafe.add_menus(meals);
    collins_cafe.add_to_go_meals(to_go_meals);

    menus.add_cafe(collins_cafe);

    // Get Hub
    info!("Getting menus for The Hub");
    let mut hub_cafe: Cafe = Cafe::new("The Hub".to_string(), "".to_string());
    let (meals, to_go_meals) = get_cafebonappetit_menus(days_to_get, HUB, &start_date).await?;
    hub_cafe.add_menus(meals);
    hub_cafe.add_to_go_meals(to_go_meals);

    menus.add_cafe(hub_cafe);

    Ok(menus)
    
}

pub async fn get_cafebonappetit_menus(num: usize, menu_id: &str, start_date: &NaiveDate) -> Result<(Vec<DayMenu>, Vec<Meal>), Box<dyn std::error::Error>> {
    let url = format!("https://legacy.cafebonappetit.com/api/2/cafes?cafe={}&date={}", menu_id, start_date);

    let response = reqwest_get_ignore_ssl(&url).await?;

    let json: serde_json::Value = response.json().await?;
    let json = json["cafes"][menu_id].clone();

    // Now, start getting the menu
    let description = json.get("description").unwrap().to_string().replace("\n", "");
    
    // Remove HTML entities
    let notes = match decode_html(&description) {
        Err(_) => description,
        Ok(s) => s,
    };


    let lat = (json.get("latitude").unwrap().to_string()).parse::<f64>().unwrap_or(0.0);
    let long = (json.get("longitude").unwrap().to_string()).parse::<f64>().unwrap_or(0.0);

    let lat: u64 = (lat * 1_000_000.0) as u64;
    let long: u64 = (long * 1_000_000.0) as u64;

    // Create url to build request off of
    let mut url = format!("https://legacy.cafebonappetit.com/api/2/menus?cafe={}&date=", menu_id);

    let mut iter_date = start_date.clone();
    // Get the menu for each day
    for _ in 0..num {
        let str_date = iter_date.format("%Y-%m-%d").to_string();
        url = format!("{}{},", url, str_date);
        iter_date = iter_date.succ();
    }

    // Remove the last comma
    url.pop();

    // Get the menu
    let response = reqwest_get_ignore_ssl(&url).await?;

    let json: serde_json::Value = response.json().await?;

    // Get items to map off of
    let items = json.get("items").unwrap().as_object().unwrap();
    let mut meals: HashMap<String, Meal> = HashMap::new();

    // Parse all the items into meals
    for (_, item) in items {
        let (id, meal) = Meal::from_cafebonappetit_value(item);

        println!("ID: [{}]\n{:?}", id, meal);
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
    let base_menu = Menu::create_base_menu(start_date.format("%Y-%m-%d").to_string(), notes, lat, long);

    // Get the menu for each day
    for (index, day) in days.iter().enumerate() {

        // Get parts of the day (breakfast, lunch, dinner)
        let dayparts = day["cafes"][menu_id]["dayparts"].as_array().unwrap()[0].as_array().unwrap().clone();

        let mut day_menu = DayMenu::new(day["date"]
            .as_str()
            .unwrap()
            .to_string());

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
                let name = station["label"].to_string().replace("\"", "");;
                let notes = station["note"].to_string();

                // Create station
                let mut station_menu = Station::new(name, notes);

                let items = station["items"].as_array().unwrap();

                for item in items {
                    println!("{}", item);

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

    Ok((menus,to_go_meals))
}