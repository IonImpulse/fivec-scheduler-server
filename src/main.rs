use actix_web::*;
use actix_cors::*;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use regex::internal::Inst;

use std::time::{Duration, SystemTime, UNIX_EPOCH, Instant};
use std::process::exit;
use std::collections::HashMap;
use log::*;
use tokio::sync::Mutex;
use lazy_static::*;
use std::{sync::Arc, thread};
use bimap::*;
use rand::{thread_rng, Rng};

mod course_api;
mod database;
mod routes;
mod scrape_descriptions;
mod compute_timings;
mod locations;
mod menu;

use course_api::*;
use database::*;
use routes::*;
use scrape_descriptions::*;
use compute_timings::*;
use locations::*;
use menu::*;

pub struct MemDatabase {
    pub course_cache: Vec<Course>,
    pub last_change: u64,
    pub code_cache: BiHashMap<String, SharedCourseList>,
    pub locations_cache: HashMap<String, (String, String)>,
    pub descriptions_cache: Vec<CourseDescription>,
    pub menu_cache: HashMap<School, SchoolMenu>,
    pub term: String,
    pub ten_minute_log: Vec<Instant>,
}

impl MemDatabase {
    fn new() -> Self {
        Self {
            course_cache: Vec::new(),
            last_change: get_unix_timestamp(),
            code_cache: BiHashMap::new(),
            locations_cache: HashMap::new(),
            descriptions_cache: Vec::new(),
            menu_cache: HashMap::new(),
            term: "".to_string(),
            ten_minute_log: Vec::new(),
        }
    }

    fn add_ten_log(&mut self) {
        let now = Instant::now();
        self.ten_minute_log.push(now);
        while self.ten_minute_log.len() > 0 && self.ten_minute_log[0].elapsed() > Duration::from_secs(600) {
            self.ten_minute_log.remove(0);
        }
    }
}

// GLOBAL database variable
// Not the best way of doing this but it's hard with actix
lazy_static! {
    pub static ref MEMORY_DATABASE: Arc<Mutex<MemDatabase>> =
        Arc::new(Mutex::new(MemDatabase::new()));
}

// Debug vs release address
#[cfg(debug_assertions)]
const ADDRESS: &str = "127.0.0.1:8080";
#[cfg(not(debug_assertions))]
const ADDRESS: &str = "0.0.0.0:8080";


// Seconds per API update
const API_UPDATE_INTERVAL: u64 = 1200;
const DESCRIPTION_INTERVAL_MULTIPLIER: u64 = 100;
const MENU_INTERVAL_MULTIPLIER: u64 = 100;

pub fn get_unix_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

async fn update_loop() -> std::io::Result<()> {
    let mut number_of_repeated_errors: u64 = 0;
    let mut time_until_description_update = 1;
    let mut time_until_menu_update = 0;

    loop {
        info!("Starting schedule API update...");

        info!("Retrieving course info...");
        let course_update = full_pomona_update().await;
        
        info!("Retrieving terms...");
        let terms = get_terms().await.unwrap();
        info!("Retrieving perm numbers...");
        let perm_numbers = get_perm_numbers(&terms[0].Key).await.unwrap();
        
        let number_of_courses: usize;
        let mut term_update = "".to_string();

        if course_update.is_err() {
            number_of_repeated_errors += 1;
            number_of_courses = 0;
            error!("Error getting courses: {}", course_update.unwrap_err());
        } else {
            let course_tuple = course_update.unwrap();

            let mut final_course_update = course_tuple.1;
            term_update = course_tuple.0;

            if final_course_update.is_empty() {
                number_of_repeated_errors += 1;
                number_of_courses = 0;
                error!("No courses found!");
            } else {
                final_course_update = merge_perms_into_courses(final_course_update, perm_numbers);

                number_of_repeated_errors = 0;
                info!("Successfully updated courses!");
                number_of_courses = final_course_update.len();
                
                if time_until_description_update == 0 {
                    info!("Retreiving description info... (may take several minutes)");
                
                    time_until_description_update = DESCRIPTION_INTERVAL_MULTIPLIER;
                    
                    let course_desc_update = scrape_all_descriptions().await;
                    
                    if let Ok(course_desc_update) = course_desc_update {
                        number_of_repeated_errors = 0;

                        let merged = merge_description_and_courses(final_course_update.clone(), course_desc_update.clone());

                        let descriptions = merged.1;

                        save_descriptions_database(descriptions.clone()).unwrap();
                        save_course_database(final_course_update.clone()).unwrap();

                        let mut lock = MEMORY_DATABASE.lock().await;
                        lock.descriptions_cache = descriptions;
                        drop(lock);

                        info!("Successfully updated descriptions!");
                    } else {
                        number_of_repeated_errors += 1;
                        error!("Error getting descriptions: {:?}", course_desc_update.unwrap_err());
                    }
                
                } else {
                    time_until_description_update -= 1;
                }

                if time_until_menu_update == 0 {
                    info!("Retreiving menu info...");
                
                    time_until_menu_update = MENU_INTERVAL_MULTIPLIER;
                    
                    let menu_update = get_seven_day_menus().await;
                    
                    if let Ok(menu_update) = menu_update {
                        number_of_repeated_errors = 0;

                        save_menu_datebase(menu_update.clone()).unwrap();

                        let mut lock = MEMORY_DATABASE.lock().await;
                        lock.menu_cache = menu_update;
                        drop(lock);

                        info!("Successfully updated menus!");
                    } else {
                        number_of_repeated_errors += 1;
                        error!("Error getting menus: {:?}", menu_update.unwrap_err());
                    }
                
                } else {
                    time_until_menu_update -= 1;
                }

                info!("Saving courses to memory...");
                
                let lock = MEMORY_DATABASE.lock().await;
                let previous_courses = lock.course_cache.clone();
                drop(lock);

                info!("Merging courses...");
                let final_course_update = merge_courses(previous_courses, final_course_update);
                info!("Merged!");

                let mut lock = MEMORY_DATABASE.lock().await;
    
                lock.course_cache = final_course_update;
                lock.last_change = get_unix_timestamp();
                lock.term = term_update.clone();
    
                drop(lock);
                
                info!("Saved courses to memory!");
                
                info!("Saving caches to file...");

                let lock = MEMORY_DATABASE.lock().await;

                let _ = save_course_database(lock.course_cache.clone());
                let _ = save_code_database(lock.code_cache.clone());

                drop(lock);

                info!("Saved cache to file!");
            }   
        }

        info!("Finished schedule update with {} courses for term {}!", number_of_courses, term_update);

        // Jitter to avoid rate limiting (possibly)
        let mut jitter = thread_rng().gen_range(0..100);

        if number_of_repeated_errors > 1 {
            warn!("Currently at {} repeated errors...\nAdding time to update interval", number_of_repeated_errors);
            
            jitter += 600 + (number_of_repeated_errors * 120);
        }

        
        thread::sleep(Duration::from_secs(API_UPDATE_INTERVAL + jitter));
    }
}

/// Main function to run both actix_web server and API update loop
/// API update loops lives inside a tokio thread while the actix_web
/// server is run in the main thread and blocks until done.
async fn async_main() -> std::io::Result<()> {
    info!("Loading database(s)...");
    
    //test_menu_update().await;
    //test_full_update().await;
    // Load databases if they exist
    let mut lock = MEMORY_DATABASE.lock().await;
    lock.course_cache = load_course_database().unwrap();
    lock.code_cache = load_code_database().unwrap();
    lock.locations_cache = load_locations_database().unwrap();
    lock.descriptions_cache = load_descriptions_database().unwrap();
    lock.menu_cache = load_menu_database().unwrap();
    drop(lock);

    info!("Database(s) loaded!");
    
    tokio::spawn(async move {
        let _ = update_loop().await;
    });

    let mut builder =
        SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("/etc/letsencrypt/live/api.5scheduler.io/privkey.pem", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("/etc/letsencrypt/live/api.5scheduler.io/fullchain.pem").unwrap();


    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allow_any_method()
            .send_wildcard()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(actix_web::middleware::Compress::new(http::ContentEncoding::Gzip))
            .wrap(actix_web::middleware::Logger::default())
            .service(update_all_courses)
            .service(update_if_stale)
            .service(get_unique_code)
            .service(get_course_list_by_code)
            .service(get_locations_database)
            .service(get_status)
            .service(get_full_year_catalog)
            .service(get_catalog_if_stale)
            .service(get_menus)
    })
    .bind_openssl(ADDRESS, builder)
    .unwrap()
    .run()
    .await
}


fn main() {
    std::env::set_var("RUST_LOG", "info, actix_web=trace");
    env_logger::init();

    ctrlc::set_handler(move || {
        info!("Exiting...");
        thread::sleep(Duration::from_secs(2));
        exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    info!("5scheduler Server starting up...");

    let _ = actix_web::rt::System::with_tokio_rt(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .thread_name("main-tokio")
            .build()
            .unwrap()
    })
    .block_on(async_main());
}