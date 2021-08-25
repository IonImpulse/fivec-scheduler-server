use crate::course_api::*;
use ron;
use std::fs::OpenOptions;
use std::io::{Write, Read, Error};


const DATABASE_NAME: &str = "./course_cache.ron";

pub fn load_course_database() -> Result<Vec<Course>, Error> {
    let file = OpenOptions::new().read(true).open(DATABASE_NAME);

    if file.is_err() {
        return Ok(Vec::new());
    } else {
        let mut file = file.unwrap();

        let mut data = String::new();
    
        file.read_to_string(&mut data)?;
    
        let courses: Vec<Course> = ron::from_str(&data).unwrap();
        Ok(courses)
    }    
}

pub fn save_course_database(courses: Vec<Course>) -> Result<(), Error> {
    let mut writer = OpenOptions::new().create(true).write(true).open(DATABASE_NAME)?;

    let serialized_output = ron::to_string(&courses).unwrap();

    writer.write(serialized_output.as_bytes())?;

    Ok(())
}