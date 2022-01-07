use ::serde::*;
use chrono::*;
use reqwest::*;
use rand::{thread_rng, Rng};
use std::{thread, time};
use crate::database::*;
use crate::scrape_descriptions::*;
use crate::locations::*;
use std::fs::OpenOptions;
use std::io::{Error, Read, Write};

const SCHEDULE_API_URL: &str = "https://webapps.cmc.edu/course-search/search.php?";
const TIME_FMT: &str = "%I:%M%p";

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CourseStatus {
    Open,
    Closed,
    Reopened,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Day {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
    NA,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum School {
    ClaremontMckenna,
    Pitzer,
    Pomona,
    HarveyMudd,
    Scripps,
    Keck,
    ClaremontGraduate,
    NA,
}

impl School {
    pub fn new_from_string(s: &str) -> School {
        match s {
            "CM" | "CMC" => School::ClaremontMckenna,
            "PZ" | "PIZ" => School::Pitzer,
            "PO" | "POM" => School::Pomona,
            "HM" | "HMC" => School::HarveyMudd,
            "SC" | "SCP" => School::Scripps,
            "KG" | "KEC" => School::Keck,
            "CG" | "CGU" => School::ClaremontGraduate,
            _ => School::NA,
        }
    }     
}

impl Day {
    pub fn new_from_char(c: char) -> Self {
        match c {
            'M' => Day::Monday,
            'T' => Day::Tuesday,
            'W' => Day::Wednesday,
            'R' => Day::Thursday,
            'F' => Day::Friday,
            'S' => Day::Saturday,
            'U' => Day::Sunday,
            _ => Day::NA,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            Day::Monday => 'M',
            Day::Tuesday => 'T',
            Day::Wednesday => 'W',
            Day::Thursday => 'R',
            Day::Friday => 'F',
            Day::Saturday => 'S',
            Day::Sunday => 'U',
            _ => '0',
        }
    }

    pub fn to_index(&self) -> usize {
        match self {
            Day::Monday => 0,
            Day::Tuesday => 1,
            Day::Wednesday => 2,
            Day::Thursday => 3,
            Day::Friday => 4,
            Day::Saturday => 5,
            Day::Sunday => 6,
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Location {
    school: School,
    building: String,
    room: String,
}

impl Location {
    pub fn get_minimal_location(&self) -> String {
        format!("{} {}", self.building, self.room)
    }

    pub fn get_full_location(&self) -> (School, String, String) {
        (self.school.clone(), self.building.clone(), self.room.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CourseTiming {
    days: Vec<Day>,
    start_time: NaiveTime,
    end_time: NaiveTime,
    location: Location,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SharedCourseList {
    local_courses: Vec<Course>,
    custom_courses: Vec<Course>,
}

impl SharedCourseList {
    pub fn new(local_courses: Vec<Course>, custom_courses: Vec<Course>) -> Self {
        SharedCourseList {
            local_courses,
            custom_courses,
        }
    }
}

impl CourseTiming {
    pub fn get_days_code(&self) -> String {
        self.days.iter().map(|x| x.to_char()).collect::<String>()
    }
    pub fn get_minimal_location(&self) -> String {
        self.location.get_minimal_location()
    }

    pub fn get_full_location(&self) -> (School, String, String) {
        self.location.get_full_location()
    }

    pub fn get_start_time_str(&self) -> String {
        format!("{}", self.start_time.format(TIME_FMT))
    }

    pub fn get_end_time_str(&self) -> String {
        format!("{}", self.end_time.format(TIME_FMT))
    }

    pub fn get_days(&self) -> Vec<Day> {
        self.days.clone()
    }

    pub fn get_start_time_index(&self) -> u32 {
        self.start_time.hour() + (self.start_time.minute() * 60)
    }

    pub fn get_end_time_index(&self) -> u32 {
        self.end_time.hour() + (self.end_time.minute() * 60)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Course {
    identifier: String,
    id: String,
    code: String,
    dept: String,
    section: String,
    title: String,
    max_seats: i64,
    seats_taken: i64,
    seats_remaining: i64,
    credits: u64, // Credits * 100, so 3.50 credits is 350, allowing for a decimal
    credits_hmc: u64,
    status: CourseStatus,
    timing: Vec<CourseTiming>,
    instructors: Vec<String>,
    notes: String,
    description: String,
}

impl Course {
    pub fn create_identifier(code: String, id: String, dept: String, section: String) -> String {
        format!("{}-{}-{}-{}", code, id, dept, section)
    }

    pub fn get_identifier(&self) -> String {
        format!("{}-{}-{}-{}", self.code, self.id, self.dept, self.section)
    }

    pub fn get_id(&self) -> &String {
        &self.id
    }

    pub fn get_code(&self) -> &String {
        &self.code
    }

    pub fn get_dept(&self) -> &String {
        &self.dept
    }

    pub fn get_section(&self) -> &String {
        &self.section
    }

    pub fn get_title(&self) -> &String {
        &self.title
    }

    pub fn get_max_seats(&self) -> &i64 {
        &self.max_seats
    }

    pub fn get_seats_taken(&self) -> &i64 {
        &self.seats_taken
    }

    pub fn get_seats_remaining(&self) -> &i64 {
        &self.seats_remaining
    }

    pub fn get_credits(&self) -> &u64 {
        &self.credits
    }

    pub fn get_timings(&self) -> Vec<CourseTiming> {
        self.timing.clone()
    }

    pub fn get_timing_minimal(&self) -> String {
        let mut string_timings = String::new();

        for time in &self.timing {
            string_timings = format!(
                "{}\n{} | {} - {} at {}",
                string_timings,
                time.get_days_code(),
                time.get_start_time_str(),
                time.get_end_time_str(),
                time.get_minimal_location()
            );
        }

        string_timings
    }

    pub fn get_desc_api_str(&self) -> String {
        format!("{}{} {}", self.code, self.id, self.dept)
    }

    pub fn get_desc_scrape_str(&self) -> String {
        format!("{}{}{}", self.code, self.id, self.dept)
    }

    pub fn set_description(&mut self, description: String) {
        self.description = description
    }

    pub fn get_description(&self) -> String {
        self.description.clone()
    }
}

pub fn get_rows_clean(raw_text: &String) -> Option<Vec<String>> {
    // Split at start of table and end, taking only the rows
    let rows: Vec<&str> = raw_text
        .split_at(raw_text.find("<tbody>")?)
        .1
        .split_at(raw_text.find("</tbody>")?)
        .0
        .lines()
        .collect();

    // Clean up each row and convert to a string
    let clean_rows = rows.iter().map(|row| row.trim().to_string()).collect();

    Some(clean_rows)
}

pub fn group_rows_as_courses(rows: Vec<String>) -> Vec<Vec<String>> {
    let mut courses = Vec::new();
    let mut current_course = Vec::new();

    for row in rows {
        if row.contains("<td>") && row.trim() != "" {
            current_course.push(
                row.replace("<td>", "")
                    .replace("</td>", "")
                    .replace("&nbsp;", "")
                    .replace("&amp;", "&"),
            );
        } else if row.contains("</tr>") {
            if !current_course.is_empty() {
                courses.push(current_course);
            }

            current_course = Vec::new();
        }
    }

    courses
}

pub fn html_group_to_course(group: Vec<String>) -> Course {
    // Get dept
    let first_number = group[0].chars().position(|c| c.is_numeric()).unwrap();
    let (code, second_half) = group[0].split_at(first_number);

    let code = code.trim().to_string();

    let mut second_half: Vec<&str> = second_half.split_whitespace().collect();

    let mut id: String;
    let mut dept: String;
    let mut section: String;
    // Did they forget to put a space?
    if second_half.len() == 3 {
        id = second_half[0].to_string();
        id = id[..id.len() - 2].to_string();
        dept = second_half[0][id.len()..].to_string();
        section = second_half[2].to_string();
    } else {
        // Get the rest of the course info
        id = second_half.get(0).unwrap().to_string();
        dept = second_half.get(1).unwrap().to_string();
        section = second_half.get(3).unwrap_or(&"typo").to_string();
    }


    if section == "typo" {
        section = dept;
        dept = format!("{}{}", id.pop().unwrap(), id.pop().unwrap());
    }

    // Get full title of course
    let title: String = group[1]
        .split(">")
        .nth(1)
        .unwrap()
        .split("<")
        .nth(0)
        .unwrap()
        .to_string();

    // Get seating numbers
    let seats_remaining = group[2]
        .split("/")
        .next()
        .unwrap()
        .trim()
        .parse::<i64>()
        .unwrap();
    let max_seats = group[2]
        .split("/")
        .nth(1)
        .unwrap()
        .trim()
        .split_whitespace()
        .next()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let seats_taken = max_seats - seats_remaining;

    // Get status
    let status: CourseStatus;

    if group[2].contains("(Open)") {
        status = CourseStatus::Open;
    } else if group[2].contains("Closed") {
        status = CourseStatus::Closed;
    } else {
        status = CourseStatus::Reopened;
    }

    // Get credits
    let mut credits: u64 = (group[3].trim().parse::<f64>().unwrap_or(0.) * 100.).floor() as u64;

    // Get timing(s)
    let mut timing = Vec::new();

    let timing_list = group[4].split("<BR>").collect::<Vec<&str>>();

    let mut at_hmc = false;

    for t in timing_list {
        let mut split = t.split_whitespace();

        let days: Vec<Day> = split
            .nth(0)
            .unwrap()
            .chars()
            .map(|c| Day::new_from_char(c))
            .collect();

        let mut timing_split = split.nth(0).unwrap_or("12:00AM-12:00AM").split("-");

        let start_time =
            NaiveTime::parse_from_str(timing_split.nth(0).unwrap_or("12:00AM").trim(), TIME_FMT)
                .unwrap_or(NaiveTime::from_hms(0, 0, 0));
        let end_time =
            NaiveTime::parse_from_str(timing_split.nth(0).unwrap_or("12:00AM").trim(), TIME_FMT)
                .unwrap_or(NaiveTime::from_hms(0, 0, 0));

        let mut split = t.split("/");

        let mut location_string = split.nth(1).unwrap().trim().split(",");

        // Convert two char school code to school pub enum
        let school = School::new_from_string(&location_string.nth(0).unwrap().trim()[0..2]);

        if school == School::HarveyMudd {
            at_hmc = true;
        }

        let building = location_string
            .nth(0)
            .unwrap_or("N/A")
            .trim()
            .trim_end_matches(",")
            .to_string();

        let room = location_string
            .nth(0)
            .unwrap_or("N/A")
            .trim()
            .trim_end_matches(",")
            .to_string();

        let location = Location {
            school,
            building,
            room,
        };

        timing.push(CourseTiming {
            days,
            start_time,
            end_time,
            location,
        });
    }

    // Get instructors
    let instructors: Vec<String> = group[5].split("<BR>").map(|x| {
        let to_return: String;

        if x.contains(",") {
            let temp_instructor: Vec<&str> = x.split(",").collect();
            to_return = format!("{} {}", temp_instructor[1], temp_instructor[0]);
        } else {
            to_return = x.to_string();
        }
        
        to_return.trim().to_string()
    }).collect();

    // Get notes
    let notes = group[6].trim().to_string().replace("<BR>", "\n");

    // Create identifier 
    let identifier = Course::create_identifier(code.clone(), id.clone(), dept.clone(), section.clone());

    let credits_hmc: u64;

    if at_hmc {
        credits_hmc = credits;
        credits = credits / 3;
    } else {
        credits_hmc = credits * 3;
    }


    Course {
        identifier,
        id,
        title,
        code,
        section,
        dept,
        seats_taken,
        seats_remaining,
        max_seats,
        instructors,
        notes,
        status,
        credits,
        credits_hmc,
        timing,
        description: "".to_string(),
    }
}

pub fn get_term(raw_text: &String) -> Option<&str> {
    let term = raw_text.split("</h4>").nth(0);

    if term.is_none() {
        return term;
    } else {
        return term.unwrap().split("<h4>Course Search Results - ").nth(1);
    }
}

pub async fn get_all_courses() -> Result<(String, Vec<Course>)> {
    // Get data from CMC API
    let response = reqwest::get(SCHEDULE_API_URL).await?;
    let data = response.text().await?;

    // Get term
    let term = get_term(&data);
    
    // Clean raw html data
    let html_rows = get_rows_clean(&data);

    if html_rows.is_none() || term.is_none() {
        return Ok(("".to_string(), Vec::new()))
    }

    // Group rows into courses
    let html_grouped_rows = group_rows_as_courses(html_rows.unwrap());

    // Convert each group of rows into a Course
    let courses: Vec<Course> = html_grouped_rows
        .into_iter()
        .map(|x| html_group_to_course(x))
        .collect();

    Ok((term.unwrap().to_string(), courses))
}

pub async fn test_full_update() {
    let course_tuple = get_all_courses().await.unwrap();
    println!("{:?}", course_tuple.1);
    let all_courses = course_tuple.1;
    let term = course_tuple.0;
    
    /*
    let locations = get_locations(all_courses.clone()).await;

    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .open("locations.json").unwrap();

    let serialized_output = serde_json::to_string(&locations).unwrap();

    writer.write(serialized_output.as_bytes());
    */
    let all_descriptions = scrape_all_descriptions().await.unwrap();

    let all_courses_with_descriptions = merge_description_into_courses(all_courses, all_descriptions);

    save_course_database(all_courses_with_descriptions.clone()).unwrap();

    assert_eq!(all_courses_with_descriptions, load_course_database().unwrap())
}