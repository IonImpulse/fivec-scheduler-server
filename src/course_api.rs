use crate::database::*;
use crate::http::Method;
use crate::scrape_descriptions::*;
use crate::menu::*;
use ::serde::*;
use chrono::*;
use regex::Regex;
use reqwest::header::*;
use reqwest::*;
use serde_json::Value;
use std::collections::HashMap;

const SCHEDULE_API_URL: &str = "https://webapps.cmc.edu/course-search/search.php?";

const POM_API: &str = "https://jicsweb.pomona.edu/api/";
const POM_COURSES: &str = "Courses/";
const POM_TERMS: &str = "Terms/";
const POM_COURSE_AREAS: &str = "Courseareas/";
const POM_HEADERS: &str = "text/json; charset=utf-8";
const COURSE_REGEX: &str = r"([A-Z]+){1} *([0-9]+[ -Z]{0,3}){1} {0,2}([A-Z]{2})?-([0-9]*)";

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
        (
            self.school.clone(),
            self.building.clone(),
            self.room.clone(),
        )
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
    prerequisites: String,
    corequisites: String,
    offered: String,
    perm_count: u64,
    fee: u64,
}

impl Course {
    pub fn create_identifier(code: String, id: String, dept: String, section: String) -> String {
        format!("{}-{}-{}-{}", code, id, dept, section)
    }

    pub fn get_identifier(&self) -> &String {
        &self.identifier
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

    pub fn get_notes(&self) -> &String {
        &self.notes
    }

    pub fn set_notes(&mut self, notes: String) {
        self.notes = notes.replace("..", ".");
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

    pub fn get_school(&self) -> Option<School> {
        let timing = self.timing.get(0);

        if let Some(timing) = timing {
            return Some(timing.location.school.clone());
        } else {
            return None;
        }
    }

    pub fn get_instructors(&self) -> Vec<String> {
        self.instructors.clone()
    }

    pub fn set_prerequisites(&mut self, prerequisites: String) {
        self.prerequisites = Course::format_reqs(prerequisites)
    }

    pub fn get_prerequisites(&self) -> String {
        self.prerequisites.clone()
    }

    pub fn set_corequisites(&mut self, corequisites: String) {
        self.corequisites = Course::format_reqs(corequisites)
    }

    pub fn get_corequisites(&self) -> String {
        self.corequisites.clone()
    }

    pub fn set_fee(&mut self, fee: u64) {
        self.fee = fee
    }

    pub fn get_fee(&self) -> u64 {
        self.fee
    }

    pub fn format_reqs(rec: String) -> String {
        let mut reqs = rec;

        for (i, c) in reqs.clone().chars().enumerate() {
            if i > 0 && i < reqs.len() - 1 {
                // If a lowercase letter is followed by a capital letter, add a space
                // EX: andE80 => and E80
                if c.is_uppercase() && reqs.chars().nth(i - 1).unwrap().is_lowercase() {
                    reqs = format!("{} {}", reqs[..i].to_string(), c);
                }

                // If there's no space between a comman and a letter, add a space
                if c.is_alphanumeric() && reqs.chars().nth(i - 1).unwrap() == ',' {
                    reqs = format!("{} {}", reqs[..i].to_string(), c);
                }

                // If there *is* space between a letter and a comma, remove the space
                if c == ',' && reqs.chars().nth(i - 1).unwrap() == ' ' {
                    reqs = format!("{}{}", reqs[..i].to_string(), c);
                }
            }
        }

        reqs = reqs.trim_start_matches(", ").to_string();

        reqs = reqs.replace(" ", " ");

        reqs
    }

    pub fn set_perm_count(&mut self, perm_count: u64) {
        self.perm_count = perm_count;
    }

    pub fn new_from_pomona_api(pom: serde_json::Value) -> Course {
        let course_code = pom["CourseCode"].as_str().unwrap().to_string();

        let identifier = convert_course_code_to_identifier(&course_code);
        
        let identifier_split: Vec<&str> = identifier.split("-").collect();
        println!("{}",identifier);

        let title = pom["Name"].as_str().unwrap_or("").trim().to_string();

        let description = pom["Description"].as_str().unwrap_or("").to_string();

        let mut credits = (pom["Credits"].as_str().unwrap_or("").replace("\"","").parse::<f64>().unwrap_or(0.)) as u64 * 100;

        let mut instrutors = Vec::new();

        for instructor in pom["Instructors"].as_array().unwrap_or(&vec![]) {
            let name = instructor["Name"].as_str().unwrap_or("").to_string();
            // Rearrange the name to be in the format "First Last"
            let split = name.split(",").collect::<Vec<&str>>();

            let mut name = String::new();

            if split.len() == 2 {
                name = format!("{} {}", split[1].trim(), split[0].trim());
            } else {
                name = format!("{}", split[0].trim());
            }

            instrutors.push(name);
        }

        let perm_number = pom["PermCount"].as_str().unwrap_or("0").replace("\"","");

        let perm_count = perm_number.parse::<u64>().unwrap_or(0);

        let max_seats = pom["SeatsTotal"].as_str().unwrap_or("0").replace("\"","");
        let max_seats = max_seats.parse::<i64>().unwrap_or(0);

        let seats_taken = pom["SeatsFilled"].as_str().unwrap_or("0").replace("\"","");
        let seats_taken = seats_taken.parse::<i64>().unwrap_or(0);

        let seats_remaining = max_seats - seats_taken;


         // Get status
        let status: CourseStatus;
        let pom_status = pom["CourseStatus"].as_str().unwrap_or("");

        if pom_status == "Open" {
            status = CourseStatus::Open;
        } else if pom_status.contains("Closed") {
            status = CourseStatus::Closed;
        } else {
            status = CourseStatus::Reopened;
        }


        let mut timing = Vec::new();

        for time in pom["Schedules"].as_array().unwrap() {
            let school = time["Campus"]
                .as_str()
                .unwrap_or("")
                .split(" ")
                .nth(0).unwrap_or("");

            let school = School::new_from_string(school);

            let building = time["Building"].as_str().unwrap_or("").to_string();
            let meet_time = time["MeetTime"].as_str().unwrap_or("").to_string();

            let times = meet_time.split(".").nth(0).unwrap_or("").split("-").collect::<Vec<&str>>();

            println!("{:?}", times);
            
            let mut start_time = times.get(0).unwrap_or(&"12:00AM").to_string();
            let end_time = times.get(1).unwrap_or(&"12:00AM").to_string();

            if start_time.len() == 5 {
                start_time = format!("{}{}", start_time, end_time[5..].to_string());
            }

            let start_time =
            NaiveTime::parse_from_str(start_time.trim(), TIME_FMT)
                .unwrap_or(NaiveTime::from_hms(0, 0, 0));

            let end_time =
            NaiveTime::parse_from_str(end_time.trim(), TIME_FMT)
                .unwrap_or(NaiveTime::from_hms(0, 0, 0));

            let days = time["Weekdays"].as_str().unwrap_or("").to_string();

            let days: Vec<Day> = days
                .chars()
                .map(|c| Day::new_from_char(c))
                .collect();

            let mut room_building = meet_time.split_once(".").unwrap_or(("", meet_time.as_str())).1.to_string();
            
            if room_building.contains("(") {
                room_building = room_building.split_once("(").unwrap().0.to_string();
            }

            let room = room_building.split_once("Room").unwrap_or(("", room_building.as_str())).1.trim().to_string();

            let location = Location {
                building,
                room,
                school,
            };


            let time = CourseTiming {
                start_time,
                end_time,
                location,
                days,
            };

            timing.push(time);
        }


        let credits_hmc: u64;

        if pom["PrimaryAssociation"].as_str().unwrap_or("") == "HM" {
            credits_hmc = credits;
            credits = credits / 3;
        } else {
            credits_hmc = credits * 3;
        }

        Course {
            identifier: identifier.clone(),
            id: identifier_split[0].to_string(),
            code: identifier_split[1].to_string(),
            section: identifier_split[3].to_string(),
            dept: pom["Department"].as_str().unwrap_or("").to_string(),
            title,
            description,
            credits,
            max_seats,
            seats_taken,
            seats_remaining,
            timing,
            instructors: instrutors,
            prerequisites: String::new(),
            corequisites: String::new(),
            fee: 0,
            perm_count,
            notes: String::new(),
            status,
            offered: "".to_string(),
            credits_hmc,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PartialPomCourse {
    pub course_code: String,
    pub identifier: String,
    pub title: String,
    pub description: String,
    pub perm_count: u64,
    pub credits: u64,
}

impl PartialPomCourse {
    pub fn new_from_area_pom(pom: serde_json::Value) -> PartialPomCourse {
        let re = Regex::new(COURSE_REGEX).unwrap();

        let course_code = pom["CourseCode"].as_str().unwrap().to_string();

        let identifier = convert_course_code_to_identifier(&course_code);

        let title = pom["Name"].as_str().unwrap_or("").trim().to_string();

        let description = pom["Description"].as_str().unwrap_or("").to_string();

        let credits = (pom["Credits"].as_str().unwrap_or("").replace("\"","").parse::<f64>().unwrap_or(0.)) as u64 * 100;

        PartialPomCourse {
            course_code,
            identifier,
            title,
            description,
            perm_count: 0,
            credits,
        }
    }
}

pub fn convert_course_code_to_identifier(code: &str) -> String {
    let mut normalized_course_code = code.to_string();
    normalized_course_code.retain(|c| !c.is_whitespace());

    let mut other: String;
    let section: String;

    if normalized_course_code.contains("-") {
        let mut split = normalized_course_code.split("-");
        other = split.nth(0).unwrap().to_string();
        section = split.nth(0).unwrap().to_string();
    } else {
        other = normalized_course_code;
        section = "".to_string();
    }

    let dept = other[other.len() - 2..other.len()].to_string();
    other = other[..other.len() - 2].to_string();

    let mut split_point = 0;
    let mut found_letter = false;
    let mut found_num = false;

    for (i, c) in other.chars().enumerate() {
        if c.is_alphabetic() {
            found_letter = true;
        } else if c.is_numeric() {
            found_num = true;
        }

        if found_letter && found_num {
            split_point = i;
            break;
        }
    }

    let code = &other[..split_point];
    let id = &other[split_point..];

    if section == "" {
        format!("{}-{}-{}", code, id, dept)
    } else {
        format!("{}-{}-{}-{}", code, id, dept, section)    
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
    let instructors: Vec<String> = group[5]
        .split("<BR>")
        .map(|x| {
            let to_return: String;

            if x.contains(",") {
                let temp_instructor: Vec<&str> = x.split(",").collect();
                to_return = format!("{} {}", temp_instructor[1], temp_instructor[0]);
            } else {
                to_return = x.to_string();
            }

            to_return.trim().to_string()
        })
        .collect();

    // Get notes
    let notes = group[6]
        .trim()
        .to_string()
        .replace("<BR>", ". ")
        .replace("..", ".");

    // Create identifier
    let identifier =
        Course::create_identifier(code.clone(), id.clone(), dept.clone(), section.clone());

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
        prerequisites: "".to_string(),
        corequisites: "".to_string(),
        offered: "".to_string(),
        perm_count: 0,
        fee: 0,
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
        return Ok(("".to_string(), Vec::new()));
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

pub fn merge_locations(
    current_locations: HashMap<String, (String, String)>,
    new_locations: HashMap<String, (String, String)>,
) -> HashMap<String, (String, String)> {
    let mut locations = current_locations;

    for (key, value) in new_locations {
        if !locations.contains_key(&key) {
            locations.insert(key, value);
        }
    }

    locations
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseArea {
    Code: String,
    Description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Term {
    pub Description: String,
    pub Key: String,
    Session: String,
    SubSession: String,
    Year: String,
}

pub async fn get_areas() -> std::result::Result<Vec<CourseArea>, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, POM_HEADERS.parse().unwrap());

    // Get data from POM API
    let client = reqwest::Client::new();
    let data = client
        .request(Method::GET, format!("{}{}", POM_API, POM_COURSE_AREAS))
        .headers(headers)
        .send()
        .await?;

    let data = data.text().await?;

    // Deserialize json
    let areas: Vec<CourseArea> = serde_json::from_str(&data)?;

    println!("Got {} areas", areas.len());
    println!("{:?}", areas);

    Ok(areas)
}

pub async fn get_terms() -> std::result::Result<Vec<Term>, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, POM_HEADERS.parse().unwrap());

    // Get data from POM API
    let client = reqwest::Client::new();
    let data = client
        .request(Method::GET, format!("{}{}", POM_API, POM_TERMS))
        .headers(headers)
        .send()
        .await?;

    let data = data.text().await?;

    // Deserialize json
    let terms: Vec<Term> = serde_json::from_str(&data)?;

    println!("Got {} terms", terms.len());
    println!("{:?}", terms[0]);

    Ok(terms)
}

pub async fn get_perm_numbers(term_key: &str) -> std::result::Result<HashMap<String, u64>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, POM_HEADERS.parse().unwrap());

    // Get all courses
    let all_courses_data = client.request(Method::GET, format!("{}{}{}", POM_API, POM_COURSES, term_key)).headers(
        headers.clone())
        .send()
        .await?;
    let all_courses_data = all_courses_data.text().await?;

    let all_courses: Value = serde_json::from_str(&all_courses_data)?;

    let mut perm_numbers = HashMap::new();

    for course in all_courses.as_array().unwrap() {
        let perm_number = course["PermCount"].as_str().unwrap_or("0").replace("\"","");
        let course_code = course["CourseCode"].as_str().unwrap();
        let identifier = convert_course_code_to_identifier(course_code);

        perm_numbers.insert(identifier, perm_number.parse::<u64>().unwrap_or(0));
    }

    Ok(perm_numbers)
}

pub async fn get_pom_courses(
    areas: Vec<CourseArea>,
    term: Term,
) -> std::result::Result<Vec<Course>, Box<dyn std::error::Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, POM_HEADERS.parse().unwrap());

    // Get data from POM API
    let client = reqwest::Client::new();

    let mut courses: Vec<Course> = Vec::new();

    for area in areas {
        println!("Getting courses for area {}", area.Code);

        if area.Code.contains("/") {
            continue;
        }

        let data = client
            .request(
                Method::GET,
                format!("{}{}{}/{}", POM_API, POM_COURSES, term.Key, area.Code),
            )
            .headers(headers.clone())
            .send()
            .await?;

        let data = data.text().await?;

        if data.is_empty() {
            println!("No courses found for area {}", area.Code);
            continue;
        }

        // Deserialize json
        let courses_pom: Value = serde_json::from_str(&data)?;
        let courses_pom = courses_pom.as_array().unwrap();
        
        for course_pom in courses_pom {
            let course = Course::new_from_pomona_api(course_pom.clone());
            
            // Don't push duplicates
            if courses.iter().find(|x| x.get_identifier() == course.get_identifier()).is_none() {
                courses.push(course);
            } else {
                println!("Duplicate course found: {}", course.get_identifier());
            }
        }
    }

    let courses = find_reqs_courses(&mut courses);

    Ok(courses)
}

pub async fn full_pomona_update() -> Result<(String, Vec<Course>)> {
    // First, get the course areas from the API
    let areas = get_areas().await.unwrap();

    // Then, get the course terms
    let terms = get_terms().await.unwrap();

    // Then, get the courses for each area
    let courses = get_pom_courses(areas, terms[0].clone()).await.unwrap();

    Ok((terms[0].clone().Description, courses))
}

pub fn merge_perms_into_courses(courses: Vec<Course>, perm_hashmap: HashMap<String, u64>) -> Vec<Course> {
    let courses = courses.iter().map(|course| {
        let perm_count = perm_hashmap.get(course.get_identifier()).unwrap_or(&0);
        let mut new_course = course.clone();
        new_course.perm_count = *perm_count;
        new_course
    }).collect::<Vec<Course>>();

    courses
}

pub async fn test_full_update() {
    let course_tuple = full_pomona_update().await.unwrap();
    println!("{:?}", course_tuple.1);
    let all_courses = course_tuple.1;
    let term = course_tuple.0;

    let terms = get_terms().await.unwrap();
    let perm_map = get_perm_numbers(&terms[0].Key).await.unwrap();
    println!("{:?}", perm_map);

    let all_courses = merge_perms_into_courses(all_courses, perm_map);

    save_course_database(all_courses.clone()).unwrap();

    /*
    let current_locations = load_locations_database().unwrap();

    let new_locations = get_locations(all_courses.clone()).await;

    let new_locations = merge_locations(current_locations, new_locations);

    let mut writer = OpenOptions::new()
        .create(true)
        .write(true)
        .open("locations.json").unwrap();

    let serialized_output = serde_json::to_string(&new_locations).unwrap();

    writer.write(serialized_output.as_bytes());

     */

    let mut all_descriptions = scrape_all_descriptions().await.unwrap();

    //let mut all_descriptions = load_descriptions_database().unwrap();

    let all_descriptions = find_reqs(&mut all_descriptions);

    println!("Scraped {} descriptions", all_descriptions.len());

    let final_courses = merge_description_and_courses(all_courses, all_descriptions);

    let courses = final_courses.0;
    let descriptions = final_courses.1;

    save_course_database(courses.clone()).unwrap();
    save_descriptions_database(descriptions.clone()).unwrap();

    assert_eq!(courses, load_course_database().unwrap())
}

pub async fn test_menu_update() {
    let menus = get_seven_day_menus().await.unwrap();

    // Save them to json
    save_menu_datebase(menus.clone()).unwrap();

}
