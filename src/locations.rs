use csv::*;
use std::error::Error;
use log::{info, error};
use reqwest::*;
use std::collections::HashMap;
use crate::course_api::*;
use serde::Deserialize;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub async fn get_locations(courses: Vec<Course>) -> HashMap<String, (String, String)> {
    let mut locations_pairs: HashMap<String, (String, String)> = HashMap::new();

    for course in courses {
        for time in course.get_timings() {
            let loc = time.get_full_location();
            let loc_key = format!("{:?}-{}", loc.0, loc.1);

            if !locations_pairs.contains_key(&loc_key) {
                let (lat, lon) = get_lat_lon(loc).await.unwrap_or(("".to_string(), "".to_string()));
                locations_pairs.insert(loc_key.clone(), (lat.clone(), lon.clone()));
                
                info!("Got coords for {} at {}/{}", loc_key, lat, lon);
                thread::sleep(Duration::from_millis(1010));
            } else {
                info!("{} already exists", loc_key);
            }
        }
    }

    locations_pairs
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
    place_id: i64,
    licence: String,
    osm_id: i64,
    boundingbox: Vec<String>,
    lat: String,
    lon: String,
    display_name: String,
    class: String,
    r#type: String,
    importance: f64,
}

pub async fn get_lat_lon(loc: (School, String, String)) -> std::result::Result<(String, String), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .user_agent("api.5scheduler.io")
        .build()?;

    // First try, without school name
    let url = format!("https://nominatim.openstreetmap.org/search?q={},%20Claremont%2091711&format=json&email=edv121@outlook.com", loc.1);
    let res = reqwest::get(&url).await?.json::<Vec<ApiResponse>>().await;

    let mut redo = false;

    if res.is_ok() {
        let res = res.unwrap();
        if res.len() > 0 {
            return Ok((res[0].lat.clone(), res[0].lon.clone()));
        }
    }

    // Did not return, try again with school name    
    let url = format!("https://nominatim.openstreetmap.org/search?q={:?}%20{},%20Claremont%2091711&format=json&email=edv121@outlook.com", loc.0, loc.1);
    let res = reqwest::get(&url).await?.json::<Vec<ApiResponse>>().await;

    if res.is_ok() {
        let res = res.unwrap();
        if res.len() > 0 {
            return Ok((res[0].lat.to_string(), res[0].lon.to_string()));
        } else {
            return Ok(("".to_string(), "".to_string()));
        }
    } else {
        (Err(Box::new(res.unwrap_err())))
    }
}