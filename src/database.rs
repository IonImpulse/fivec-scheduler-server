use crate::course_api::*;
use crate::menu::SchoolMenu;
use crate::scrape_descriptions::*;
use bimap::*;
use std::fs::OpenOptions;
use std::io::{Error, Read, Write};
use std::collections::HashMap;
use rand::prelude::SliceRandom;
use rand::rngs::SmallRng;
use rand::SeedableRng;

const COURSE_DATABASE_NAME: &str = "./course_cache.json";
const CODE_DATA_NAME: &str = "./code_data.json";
const AREAS_DATA_NAME: &str = "./areas_data.json";
const LOCATION_NAME: &str = "./locations.json";
const DESCRIPTION_NAME: &str = "./descriptions.json";
const MENU_DATABASE_NAME: &str = "./menu_cache.json";

const POSSIBLE_CODE_CHARS: &'static [char] = &[
    '2', '3', '4', '6', '7', '9', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'P', 'A', 'D', 'F', 'G', 'H',
    'X',
];
const CODE_LENGTH: u8 = 7;

pub fn load_course_database() -> Result<Vec<Course>, Error> {
    let file = OpenOptions::new().read(true).open(COURSE_DATABASE_NAME);

    if file.is_err() {
        return Ok(Vec::new());
    } else {
        let mut file = file.unwrap();

        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let courses: Vec<Course> = from_slice_lenient(&data.as_bytes()).unwrap();
        Ok(courses)
    }
}

pub fn save_course_database(courses: Vec<Course>) -> Result<(), Error> {
    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .open(COURSE_DATABASE_NAME)?;

    let serialized_output = serde_json::to_string(&courses).unwrap();

    writer.write(serialized_output.as_bytes())?;

    Ok(())
}

pub fn load_areas_database() -> Result<Vec<CourseArea>, Error> {
    let file = OpenOptions::new().read(true).open(AREAS_DATA_NAME);

    if file.is_err() {
        return Ok(Vec::new());
    } else {
        let mut file = file.unwrap();

        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let areas: Vec<CourseArea> = from_slice_lenient(&data.as_bytes()).unwrap();
        Ok(areas)
    }
}

pub fn save_areas_database(areas: Vec<CourseArea>) -> Result<(), Error> {
    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .open(AREAS_DATA_NAME)?;

    let serialized_output = serde_json::to_string(&areas).unwrap();

    writer.write(serialized_output.as_bytes())?;

    Ok(())
}

pub fn load_code_database() -> Result<BiHashMap<String, SharedCourseList>, Error> {
    let file = OpenOptions::new().read(true).open(CODE_DATA_NAME);

    if file.is_err() {
        return Ok(BiHashMap::new());
    } else {
        let mut file = file.unwrap();

        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let courses: BiHashMap<String, SharedCourseList> = from_slice_lenient(&data.as_bytes()).unwrap();
        Ok(courses)
    }
}

pub fn save_code_database(code_hashmap: BiHashMap<String, SharedCourseList>) -> Result<(), Error> {
    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .open(CODE_DATA_NAME)?;

    let serialized_output = serde_json::to_string(&code_hashmap).unwrap();

    writer.write(serialized_output.as_bytes())?;

    Ok(())
}

pub fn load_locations_database() -> Result<HashMap<String, (String, String)>, Error> {
    let file = OpenOptions::new().read(true).open(LOCATION_NAME);

    if file.is_err() {
        return Ok(HashMap::new());
    } else {
        let mut file = file.unwrap();

        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let locations: HashMap<String, (String, String)> = from_slice_lenient(&data.as_bytes()).unwrap();
        Ok(locations)
    }
}

pub fn load_descriptions_database() -> Result<Vec<CourseDescription>, Error> {
    let file = OpenOptions::new().read(true).open(DESCRIPTION_NAME);

    if file.is_err() {
        return Ok(Vec::new());
    } else {
        let mut file = file.unwrap();

        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let descriptions: Vec<CourseDescription> = from_slice_lenient(&data.as_bytes()).unwrap();
        Ok(descriptions)
    }
}

pub fn save_descriptions_database(descriptions: Vec<CourseDescription>) -> Result<(), Error> {
    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .open(DESCRIPTION_NAME)?;

    let serialized_output = serde_json::to_string(&descriptions).unwrap();

    writer.write(serialized_output.as_bytes())?;

    Ok(())
}

pub fn load_menu_database() -> Result<HashMap<School, SchoolMenu>, Error> {
    let file = OpenOptions::new().read(true).open(MENU_DATABASE_NAME);

    if file.is_err() {
        return Ok(HashMap::new());
    } else {
        let mut file = file.unwrap();

        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let menus: HashMap<School, SchoolMenu> = from_slice_lenient(&data.as_bytes()).unwrap();
        Ok(menus)
    }
}

pub fn save_menu_datebase(menus: HashMap<School, SchoolMenu>) -> Result<(), Error> {
    let mut writer = OpenOptions::new() 
        .create(true)
        .write(true)
        .open(MENU_DATABASE_NAME)?;

    let serialized_output = serde_json::to_string(&menus).unwrap();

    writer.write(serialized_output.as_bytes())?;

    Ok(())
}

pub fn generate_unique_code(
    shared_course_list: SharedCourseList,
    code_hashmap: BiHashMap<String, SharedCourseList>,
) -> (String, BiHashMap<String, SharedCourseList>) {
    // Check if the database already contains course list
    if code_hashmap.contains_right(&shared_course_list) {
        let code = code_hashmap.get_by_right(&shared_course_list).unwrap().clone();
        return (code, code_hashmap);
    }
    let mut small_rng = SmallRng::from_entropy();

    loop {
        let mut attempt: Vec<char> = Vec::new();
        for _ in 0..CODE_LENGTH {
            attempt.push(POSSIBLE_CODE_CHARS.choose(&mut small_rng).unwrap().clone());
        }

        let attempt = attempt.iter().collect::<String>();

        if code_hashmap.contains_left(&attempt) {
            continue;
        } else {
            let mut code_hashmap = code_hashmap.clone();

            code_hashmap.insert(attempt.clone(), shared_course_list.clone());

            return (attempt, code_hashmap);
        }
    }
}

pub fn get_course_list(
    code: String,
    code_hashmap: BiHashMap<String, SharedCourseList>,
) -> Option<SharedCourseList> {
    let result = code_hashmap.get_by_left(&code);

    match result {
        Some(result) => Some(result.clone()),
        None => None,
    }
}

fn from_slice_lenient<'a, T: ::serde::Deserialize<'a>>(
    v: &'a [u8],
) -> Result<T, serde_json::Error> {
    let mut cur = std::io::Cursor::new(v);
    let mut de = serde_json::Deserializer::new(serde_json::de::IoRead::new(&mut cur));
    ::serde::Deserialize::deserialize(&mut de)
    // note the lack of: de.end()
}
