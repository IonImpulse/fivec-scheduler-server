use std::collections::HashMap;
use crate::course_api::*;
use crate::School::*;
use log::info;
use ::serde::*;
use escaper::*;
use chrono::*;
use crate::reqwest_get_ignore_ssl;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

struct SchoolMenu {
    school: School,
    menus: Vec<Menu>,
}

impl SchoolMenu {
    pub fn new(school: School) -> Self {
        Self {
            school,
            menus: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

struct Menu {
    date: String,
    description: String,
    time_slot: MenuTime,
    time_opens: String,
    time_closes: String,
    stations: Vec<Station>,
    lat: u64, // Multiplied by 1e7
    long: u64, // Multiplied by 1e7
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

struct Station {
    name: String,
    notes: String,
    meals: Vec<Meal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

struct Meal {
    name: String,
    notes: String,
    dietary_options: Vec<DietaryOption>,
    cost: Option<u64> // times 100 to enable serialization
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

struct DietaryOption {
    food: FoodIngredient,
    contains: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

enum FoodIngredient {
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
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]

enum MenuTime {
    Breakfast,
    Lunch,
    Dinner,
    Brunch,
    Night,
    Day,
}

/*
Pitzer:
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

pub async fn get_seven_day_menus() -> Result<HashMap<School, SchoolMenu>, Box<dyn std::error::Error>> {
    let today = Local::today().naive_local();

    // Create a HashMap to store the menus
    let mut menus = HashMap::new();

    // Get the menus for each school
    info!("Getting menus for Pitzer");
    let mut pitzer_menus = get_pitzer_menus(today, 7).await?;
    info!("Getting menus for HMC");
    let mut hmc_menus = get_hmc_menus(today, 7).await?;
    info!("Getting menus for Scripps");
    let mut scripps_menus = get_scripps_menus(today, 7).await?;
    info!("Getting menus for CMC");
    let mut cmc_menus = get_cmc_menus(today, 7).await?;
    info!("Getting menus for Pomona");
    let mut pomona_menus = get_pomona_menus(today, 7).await?;

    // Add the menus to the HashMap
    menus.insert(Pitzer, pitzer_menus);
    menus.insert(HarveyMudd, hmc_menus);
    menus.insert(Scripps, scripps_menus);
    menus.insert(ClaremontMckenna, cmc_menus);
    menus.insert(Pomona, pomona_menus);
    
    // Return the HashMap
    Ok(menus)
}

pub async fn get_pitzer_menus(start_date: NaiveDate, days_to_get: u32) -> Result<SchoolMenu, Box<dyn std::error::Error>> {
    const mcconnell: &str = "219";
    const pit_stop: &str = "220";
    const shakedown: &str = "NA";

    let mut menus = SchoolMenu::new(Pitzer);

    // Get McConnell
    info!("Getting menus for McConnell");
    let mcconnell_menu = get_cafebonappetit_menu(mcconnell).await?;

    Ok(menus)
}

pub async fn get_cafebonappetit_menu(menu_id: &str, date: &str) -> Result<Menu, Box<dyn std::error::Error>> {
    const base_url: &str = "https://legacy.cafebonappetit.com/api/2/cafes?cafe=";

    let url = format!("{}{}&date={}", base_url, menu_id, date);

    let response = reqwest_get_ignore_ssl(&url).await?;

    let json = response.json().await?;

    let description = json["description"].as_str().unwrap();
}