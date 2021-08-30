use actix_web::*;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::mpsc;
use std::process::exit;
use env_logger::*;
use log::*;
use tokio::*;
use tokio::sync::Mutex;
use lazy_static::*;
use std::{collections::*, env, sync::Arc, thread};

mod course_api;
mod database;
mod routes;

use course_api::*;
use database::*;
use routes::*;


pub struct MemDatabase {
    pub course_cache: Vec<Course>,
    pub last_change: u64,
}

impl MemDatabase {
    fn new() -> Self {
        Self {
            course_cache: Vec::new(),
            last_change: get_unix_timestamp(),
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
const API_UPDATE_INTERVAL: u64 = 60;
const DESCRIPTION_INTERVAL_MULTIPLIER: u64 = 60;
const FILE_CHANCE_MULTIPLIER: u64 = 30;

pub fn get_unix_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}


async fn update_loop() -> std::io::Result<()> {
    let mut number_of_repeated_errors: u64 = 0;
    let mut time_until_description_update = DESCRIPTION_INTERVAL_MULTIPLIER;
    let mut time_until_file_save = FILE_CHANCE_MULTIPLIER;

    loop {
        info!("Starting schedule API update...");

        info!("Retrieving course info...");
        let course_update = get_all_courses().await;

        if course_update.is_err() {
            number_of_repeated_errors += 1;
            error!("Error getting courses: {}", course_update.unwrap_err());
        } else {
            number_of_repeated_errors = 0;
            info!("Successfully updated courses!");
            let mut final_course_update: Vec<Course> = course_update.unwrap();

            if time_until_description_update == 0 {
                info!("Retreiving description info... (may take several minutes)");

                time_until_description_update = DESCRIPTION_INTERVAL_MULTIPLIER;
                let course_desc_update = get_all_descriptions(final_course_update.clone()).await;

                if course_desc_update.is_err() {
                    number_of_repeated_errors += 1;
                    error!("Error getting descriptions: {}", course_desc_update.unwrap_err());
                } else {
                    number_of_repeated_errors = 0;
                    final_course_update = course_desc_update.unwrap();
                }

            }
            info!("Saving courses to memory...");

            let mut lock = MEMORY_DATABASE.lock().await;

            lock.course_cache = final_course_update;
            lock.last_change = get_unix_timestamp();

            drop(lock);
            
            info!("Saved courses to memory!")
            
        }
        info!("Finished schedule update!");

        if time_until_file_save == 0 {
            time_until_file_save = FILE_CHANCE_MULTIPLIER;

            info!("Saving cache to file...");

            let lock = MEMORY_DATABASE.lock().await;

            let _ = save_course_database(lock.course_cache.clone());

            drop(lock);

            info!("Saved cache to file!");
        } else {
            time_until_file_save -= 1;
        }

        if number_of_repeated_errors > 5 {
            warn!("Errors have reached dangerous levels!! Currently at {} repeated errors...", number_of_repeated_errors);
        }

        thread::sleep(Duration::from_secs(API_UPDATE_INTERVAL));
    }
}

/// Main function to run both actix_web server adn API update loop
/// API update loops lives inside a tokio thread while the actix_web
/// server is run in the main thread and blocks until done.
async fn async_main() -> std::io::Result<()> {
    info!("Loading database(s)...");
    
    // Load databases if they exist
    let mut lock = MEMORY_DATABASE.lock().await;
    lock.course_cache = load_course_database().unwrap();
    drop(lock);

    info!("Database(s) loaded!");
    
    tokio::spawn(async move {
        let _ = update_loop().await;
    });

    let mut builder =
        SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("/etc/letsencrypt/live/api.5cheduler.com/privkey.pem", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("/etc/letsencrypt/live/api.5cheduler.com/fullchain.pem").unwrap();

    HttpServer::new(|| {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .service(update_all_courses)
            .service(update_if_stale)
    })
    .bind_openssl(ADDRESS, builder)
    .unwrap()
    .run()
    .await
}


fn main() {
    std::env::set_var("RUST_LOG", "info,trace");
    env_logger::init();

    ctrlc::set_handler(move || {
        info!("Exiting...");
        thread::sleep(Duration::from_secs(2));
        exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    info!("5cheduler Server starting up...");

    actix_web::rt::System::with_tokio_rt(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .thread_name("main-tokio")
            .build()
            .unwrap()
    })
    .block_on(async_main());
}