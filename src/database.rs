use crate::course_api::*;
use std::fs::{OpenOptions};
use std::io::{Write, Read, Error};
use bimap::*;

use rand::{SeedableRng};
use rand::rngs::SmallRng;
use rand::prelude::SliceRandom;

const COURSE_DATABASE_NAME: &str = "./course_cache.json";
const CODE_DATA_NAME: &str = "./code_data.json";

const POSSIBLE_CODE_CHARS: &'static [char] = &['2', '3', '4', '6', '7', '9', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'P', 'A', 'D', 'F', 'G', 'H', 'X'];
const CODE_LENGTH: u8 = 7;

pub fn load_course_database() -> Result<Vec<Course>, Error> {
    let file = OpenOptions::new().read(true).open(COURSE_DATABASE_NAME);

    if file.is_err() {
        return Ok(Vec::new());
    } else {
        let mut file = file.unwrap();

        let mut data = String::new();
    
        file.read_to_string(&mut data)?;
    
        let courses: Vec<Course> = serde_json::from_str(&data).unwrap();
        Ok(courses)
    }    
}

pub fn save_course_database(courses: Vec<Course>) -> Result<(), Error> {
    let mut writer = OpenOptions::new().create(true).write(true).open(COURSE_DATABASE_NAME)?;

    let serialized_output = serde_json::to_string(&courses).unwrap();

    writer.write(serialized_output.as_bytes())?;

    Ok(())
}

pub fn load_code_database() -> Result<BiHashMap<String, Vec<Course>>, Error> {
    let file = OpenOptions::new().read(true).open(CODE_DATA_NAME);

    if file.is_err() {
        return Ok(BiHashMap::new());
    } else {
        let mut file = file.unwrap();

        let mut data = String::new();
    
        file.read_to_string(&mut data)?;
    
        let courses: BiHashMap<String, Vec<Course>> = serde_json::from_str(&data).unwrap();
        Ok(courses)
    }
}

pub fn save_code_database(code_hashmap: BiHashMap<String, Vec<Course>>) -> Result<(), Error> {
    let mut writer = OpenOptions::new().create(true).write(true).open(CODE_DATA_NAME)?;

    let serialized_output = serde_json::to_string(&code_hashmap).unwrap();

    writer.write(serialized_output.as_bytes())?;

    Ok(())
}

pub fn generate_unique_code(course_list: Vec<Course>, code_hashmap: BiHashMap<String, Vec<Course>>) -> (String, BiHashMap<String, Vec<Course>>) {
    // Check if the database already contains course list
    if code_hashmap.contains_right(&course_list) {
        let code = code_hashmap.get_by_right(&course_list).unwrap().clone();
        return (code, code_hashmap)
    }
    
    let mut small_rng = SmallRng::from_entropy();

    loop {
        let mut attempt: Vec<char> = Vec::new();
        
        for i in 0..CODE_LENGTH {
            attempt.push(POSSIBLE_CODE_CHARS.choose(&mut small_rng).unwrap().clone());
        }

        let attempt = attempt.iter().collect::<String>();

        if code_hashmap.contains_left(&attempt) {
            continue;
        } else {
            let mut code_hashmap = code_hashmap.clone();

            code_hashmap.insert(attempt.clone(), course_list.clone());

            return (attempt, code_hashmap)
        }
    }
}

pub fn get_course_list(code: String, code_hashmap: BiHashMap<String, Vec<Course>>) -> Option<Vec<Course>> {
    let result = code_hashmap.get_by_left(&code);

    match result {
        Some(result) => Some(result.clone()),
        None => None
    }
}