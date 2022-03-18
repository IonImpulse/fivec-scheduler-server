// ----------------------------------------------------------------------------
// This file contains the code for scraping the descriptions from all
// 5 different colleges. The 2 graduate universities are not scraped.
//
// This will take a lot of time to do.
//
// Hopefully this solves the problem of the 5C's not having a public API
// that actually updates EVERYTHING.
// ----------------------------------------------------------------------------

use crate::course_api::*;
use crate::School::*;

use ::serde::*;
use escaper::*;
use lazy_static::lazy_static;
use log::{error, info, warn};
use regex::Regex;
use reqwest::*;
use rust_fuzzy_search::*;
use std::ascii::AsciiExt;
use std::collections::HashMap;
use std::error::Error;
use std::f32::consts::PI;
use std::ops::Index;

// Remove HTML while preserving what is inside of
// links/bolded
lazy_static! {
    static ref RE_HTML: Regex = Regex::new(r"<[^>]+>").unwrap();
    static ref RE_SPACES: Regex = Regex::new(r"\s+").unwrap();
}

pub fn pretty_parse_html(s: &str) -> String {
    let s = RE_HTML.replace_all(s, "").to_string();
    let s = s.replace("\\n", "");
    let s = s.replace("\n", " ");
    let s = s.replace("\\", "");
    let s = s.replace("\"", "");
    let s = s.replace(|c: char| !c.is_ascii(), " ");
    let s = RE_SPACES.replace_all(&s, " ").to_string();

    // Remove HTML entities
    let f = match decode_html(&s) {
        Err(_) => s,
        Ok(s) => s,
    };

    f.trim().to_string()
}

// Simple pair that can be used to merge into actual course data
// "Classic Title" refers to the college's way of identifying the course:
// "ASAM126 HM" instead of "ASAM-126-HM-{section_num}"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourseDescription {
    pub title: String,
    pub identifier: String,
    pub description: String,
    pub source: School,
    pub credits: u64,
    pub instructors: Vec<String>,
    pub offered: String,
    pub prerequisites: String,
    pub corequisites: String,
    pub currently_offered: bool,
    pub fee: u64,
}

impl CourseDescription {
    pub fn new(
        title: String,
        identifier: String,
        description: String,
        source: School,
        credits: u64,
        instructors: Vec<String>,
        offered: String,
        prerequisites: String,
        corequisites: String,
        currently_offered: bool,
    ) -> CourseDescription {
        CourseDescription {
            title,
            identifier,
            description,
            source,
            credits,
            instructors,
            offered,
            prerequisites,
            corequisites,
            currently_offered,
            fee: 0,
        }
    }

    pub fn set_instructors(&mut self, instructors: Vec<String>) {
        self.instructors = instructors;
    }

    pub fn set_offered(&mut self) {
        self.currently_offered = true;
    }

    pub fn set_source(&mut self, source: School) {
        self.source = source;
    }

    pub fn get_source(&self) -> &School {
        &self.source
    }

    pub fn set_fee(&mut self, fee: u64) {
        self.fee = fee;
    }

    pub fn get_fee(&self) -> u64 {
        self.fee
    }
}

// Serves only HMC classes
pub fn hmc_url(page_num: u64) -> String {
    format!("https://catalog.hmc.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=18&expand=1&navoid=892&print=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn cmc_url(page_num: u64) -> String {
    format!("https://catalog.claremontmckenna.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=29&expand=1&navoid=4499&print=1&filter[exact_match]=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn pomona_url(page_num: u64) -> String {
    format!("https://catalog.pomona.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=40&expand=1&navoid=8092&print=1&filter[exact_match]=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn scripps_url(page_num: u64) -> String {
    format!("https://catalog.scrippscollege.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=25&expand=1&navoid=3143&print=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn pitzer_url(page_num: u64) -> String {
    format!("https://catalog.pitzer.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=17&expand=1&navoid=1376&print=1&filter[exact_match]=1#acalog_template_course_filter", page_num)
}

pub fn between(source: &str, start: &str, end: &str) -> String {
    let start_pos = &source[source.find(start).unwrap() + start.len()..];

    start_pos[..start_pos.find(end).unwrap_or(start_pos.len())]
        .trim()
        .to_string()
}

pub fn between_multiple(source: &str, start: &str, end: Vec<&str>) -> String {
    let start_pos = &source[source.find(start).unwrap() + start.len()..];

    let mut end_pos = start_pos.len();

    for e in end {
        let end_pos_temp = start_pos.find(e).unwrap_or(start_pos.len());

        if end_pos_temp < end_pos {
            end_pos = end_pos_temp;
        }
    }

    start_pos[..end_pos].trim().to_string()
}

pub fn extract_description(html: String, style: School) -> Result<Vec<CourseDescription>> {
    let mut html_vec: Vec<String> = html.split("\n").map(|x| x.to_string()).collect();

    let start_indexes = html_vec
        .iter()
        .enumerate()
        .filter(|(index, x)| x.contains("</a></h3><h3>"));

    let mut return_vec: Vec<CourseDescription> = Vec::new();

    for (index, line) in start_indexes {
        // Skip lines that don't contain a course description
        if line.to_lowercase().contains("see")
            && line.to_lowercase().contains("catalog")
            && line.to_lowercase().contains("college")
        {
            continue;
        }

        let current_line = line.replace("</a></h3><h3>", "");
        let split_line = current_line.split("</h3>").collect::<Vec<&str>>();

        let mut identifier = split_line
            .clone()
            .remove(0)
            .split(" -")
            .nth(0)
            .unwrap()
            .to_string();

        let mut title = split_line
            .clone()
            .remove(0)
            .split(" -")
            .nth(1)
            .unwrap()
            .replace("&", "and")
            .replace("#8217;", "'")
            .to_string();

        identifier.retain(|c| !c.is_whitespace());
        title = title.trim().to_string();

        if split_line.len() == 0 || identifier.replace("-", "").trim() == "" {
            warn!("Empty line or identifier!");
            return Ok(return_vec);
        }

        identifier = convert_course_code_to_identifier(&identifier);

        // Guaranteed to exist
        let description;

        // Not guaranteed to exist
        let mut when_offered = String::new();
        let mut credits: f64 = 0.0;
        let mut prerequisites = String::new();
        let mut instructors = Vec::new();
        let mut corequisites = String::new();

        if &style == &HarveyMudd {
            let temp = split_line[1].split("<strong>Description:</strong>").nth(1);
            if temp.is_none() {
                return Ok(return_vec);
            } else {
                description = temp
                    .unwrap()
                    .split("<br>")
                    .nth(0)
                    .unwrap()
                    .trim()
                    .to_string();

                credits = between(split_line[1], "<strong>Credit(s):</strong> ", "<br>")
                    .parse::<f64>()
                    .unwrap_or(0.0);

                instructors = between(
                    split_line[1],
                    "<br><br><strong>Instructor(s):</strong>",
                    "<br><br>",
                )
                .split(",")
                .map(|x| x.trim().to_string())
                .collect::<Vec<String>>();

                if split_line[1].contains("<strong>Offered:</strong>") {
                    when_offered = between(split_line[1], "<strong>Offered:</strong>", "<br><br>");
                }

                if split_line[1].contains("<strong>Prerequisite(s):</strong>") {
                    prerequisites =
                        between(split_line[1], "<strong>Prerequisite(s):</strong>", "<br>");
                }

                if split_line[1].contains("<strong>Corequisite(s):</strong>") {
                    corequisites =
                        between(split_line[1], "<strong>Corequisite(s):</strong>", "<br>");
                }
            }
        } else if &style == &Scripps {
            let temp = split_line[1].split("<br><br>").nth(0);

            if temp.is_none() {
                return Ok(return_vec);
            } else {
                description = temp
                    .unwrap()
                    .split("<hr>")
                    .nth(1)
                    .unwrap()
                    .replace("<strong>", "")
                    .trim()
                    .to_string();

                let target;

                if html_vec[index + 1].contains("Credit:</strong>") {
                    target = html_vec[index + 1].as_str();
                } else {
                    target = split_line[1];
                }

                if target.contains("<strong>Course Credit:</strong>") {
                    credits = between(target, "<strong>Course Credit:</strong>", "<br>")
                        .parse::<f64>()
                        .unwrap_or(0.0);
                }

                if target.contains("<strong>Offered:</strong>") {
                    when_offered = between(target, "<strong>Offered:</strong>", "<p><br>");
                }

                if target.contains("<strong>Prerequisite(s):</strong>") {
                    if target.contains("</p>") {
                        prerequisites =
                            between(target, "<strong>Prerequisite(s):</strong>", "</p>");
                    } else {
                        prerequisites =
                            between(target, "<strong>Prerequisite(s):</strong>", "<br>");
                    }
                }
            }
        } else if &style == &ClaremontMckenna {
            description = split_line[1]
                .split("<br><br>")
                .nth(0)
                .unwrap()
                .trim()
                .to_string();

            let mut target = "".to_string();

            println!("{:?}", split_line);

            if split_line[1].contains("<br>Credit:") {
                target = split_line[1].to_string();
            } else {
                let mut i = index;
                while !target.contains("<br>Offered:") {
                    target = format!("{}{}", target, html_vec[i].as_str());
                    i += 1;
                }
            }

            credits = between(&target, "<br>Credit: ", "<br>")
                .parse::<f64>()
                .unwrap_or(0.0);

            when_offered = between(&target, "<br><br>Offered: ", "<br><br>");
        } else if &style == &Pomona {
            if split_line[1].to_lowercase().contains("when offered")
                && split_line[1].contains("<br><br>")
            {
                let mut target = String::new();

                if split_line[1].contains("<hr>") {
                    target = split_line[1].to_string();
                } else {
                    let mut i = index;

                    while !target.contains("<hr>") {
                        target = format!("{}{}", target, html_vec[i].as_str());
                        i += 1;
                    }

                    target = target.replace("<br><br>", " ");

                    target = format!("<br><br>{}", target)
                }

                description = between(target.as_str(), "<br><br>", "<hr>");

                when_offered = between(split_line[1], "</strong>", "<br>");

                if split_line[1].contains("<strong>Instructor(s):") {
                    instructors = between(
                        split_line[1],
                        "<br><strong>Instructor(s):</strong>",
                        "<br><strong>Credit:</strong>",
                    )
                    .split(";")
                    .map(|x| x.trim().to_string())
                    .collect::<Vec<String>>();
                }

                credits = between(split_line[1], "<br><strong>Credit:</strong>", "<br><br>")
                    .parse::<f64>()
                    .unwrap_or(0.0);
                ();
            } else {
                description = String::from("");
            }
        } else if &style == &Pitzer {
            let temp = split_line[1].split("<strong>Description:</strong>").nth(1);
            if temp.is_none() {
                description = String::from("");
            } else {
                description = temp
                    .unwrap()
                    .split("<br><br>")
                    .nth(0)
                    .unwrap()
                    .trim()
                    .to_string();
            }
        } else {
            description = String::from("");
        }
        let description = RE_HTML.replace_all(&description, "").to_string();
        let prerequisites = RE_HTML.replace_all(&prerequisites, "").to_string();
        let corequisites = RE_HTML.replace_all(&corequisites, "").to_string();

        // Replace all multiple spaces with a single one
        let description = RE_SPACES.replace_all(&description, " ").to_string();
        let prerequisites = RE_SPACES.replace_all(&prerequisites, " ").to_string();
        let corequisites = RE_SPACES.replace_all(&corequisites, " ").to_string();

        // Remove HTML entities
        let description = match decode_html(&description) {
            Err(_) => description,
            Ok(s) => s,
        };

        let prerequisites = match decode_html(&prerequisites) {
            Err(_) => prerequisites,
            Ok(s) => s,
        };

        let corequisites = match decode_html(&corequisites) {
            Err(_) => corequisites,
            Ok(s) => s,
        };

        info!("[{}] [{}]", title, identifier);

        let credits = (credits * 100.0) as u64;

        return_vec.push(CourseDescription::new(
            title,
            identifier,
            description.to_string(),
            style.clone(),
            credits,
            instructors,
            when_offered,
            prerequisites,
            corequisites,
            false,
        ));
    }

    Ok(return_vec)
}

pub async fn reqwest_get_ignore_ssl(url: &str) -> Result<reqwest::Response> {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get(url)
        .send()
        .await
}

pub async fn scrape_url(
    url_fn: fn(u64) -> String,
    style: School,
) -> Result<Vec<CourseDescription>> {
    let mut return_vec = Vec::new();
    let mut page_num = 1;
    let mut continue_scraping = true;

    while continue_scraping {
        info!("Scraping page {}", page_num);
        let response = reqwest_get_ignore_ssl(&url_fn(page_num))
            .await?
            .text()
            .await?;

        let extracted_pairs = extract_description(response, style.clone());

        if extracted_pairs.is_err() {
            error!("Error! {}", extracted_pairs.err().unwrap());
            continue_scraping = false;
        } else {
            let mut extracted_pairs = extracted_pairs.unwrap();

            if extracted_pairs.len() == 0 {
                continue_scraping = false;
            } else {
                return_vec.append(&mut extracted_pairs);
            }
        }

        page_num += 1;
    }
    Ok(return_vec)
}

pub fn merge_descriptions(schools_vec: Vec<Vec<CourseDescription>>) -> Vec<CourseDescription> {
    let mut return_map: Vec<CourseDescription> = Vec::new();
    let schools_vec_flat = schools_vec.into_iter().flatten();

    for course_desc in schools_vec_flat {
        // First search for all indexes for identifier
        let mut indexes: Vec<usize> = Vec::new();
        for (i, course) in return_map.iter().enumerate() {
            if course.identifier.contains(&course_desc.identifier) {
                indexes.push(i);
            }
        }

        if indexes.len() > 1 {
            let title_find = return_map
                .iter()
                .position(|r| r.title.contains(&course_desc.title));

            let index_to_set = if let Some(index) = title_find {
                index
            } else {
                indexes.remove(0)
            };
            
            if return_map[index_to_set].source == School::NA {
                return_map[index_to_set].currently_offered = course_desc.currently_offered;
                return_map[index_to_set].source = course_desc.source;
            }

            if return_map[index_to_set].description.len() < course_desc.description.len() {
                return_map[index_to_set].description = course_desc.description;
            }
            
        } else {
            return_map.push(course_desc);
        }
    }

    return_map
}

pub fn process_req(text_source: String, string: String) -> (String, String) {
    let mut req = between(&text_source, &string, ".");

    let source = text_source
        .replace(&string, "")
        .replace(req.as_str(), "")
        .replace("..", ".")
        .replace(". .", ".")
        .trim()
        .to_string();

    req = req.replace("s:", "").replace(":", "").trim().to_string();

    (source, req)
}

pub fn find_reqs(descs: &mut Vec<CourseDescription>) -> Vec<CourseDescription> {
    for mut desc in descs.iter_mut() {
        // Check for prerequisites in description
        if desc.description.contains("Prerequisite") {
            let r = process_req(desc.description.clone(), "Prerequisite".to_string());
            desc.description = r.0;
            desc.prerequisites = RE_SPACES.replace_all(&r.1, " ").to_string();
        } else if desc.description.contains("Prereq") {
            let r = process_req(desc.description.clone(), "Prereq".to_string());
            desc.description = r.0;
            desc.prerequisites = RE_SPACES.replace_all(&r.1, " ").to_string();
        }

        // Check for corequisites in description
        if desc.description.contains("Corequisite") {
            let r = process_req(desc.description.clone(), "Corequisite".to_string());
            desc.description = r.0;
            desc.corequisites = RE_SPACES.replace_all(&r.1, " ").to_string();
        } else if desc.description.contains("Coreq") {
            let r = process_req(desc.description.clone(), "Coreq".to_string());
            desc.description = r.0;
            desc.corequisites = RE_SPACES.replace_all(&r.1, " ").to_string();
        }

        desc.description = RE_SPACES.replace_all(&desc.description, " ").to_string();
    }

    descs.to_vec()
}

pub fn find_fees(descs: &mut Vec<CourseDescription>) -> Vec<CourseDescription> {
    for mut desc in descs.iter_mut() {
        if desc.description.contains("$") {
            // find the first $
            let fee = between_multiple(&desc.description, "$", vec![" ", ".", ","]);

            let parsed_fee = fee.parse::<u64>();

            if parsed_fee.is_ok() {
                desc.set_fee(parsed_fee.unwrap());
            }
        }
    }

    descs.to_vec()
}

pub fn convert_courses_to_descs(courses: Vec<PartialPomCourse>) -> Vec<CourseDescription> {
    let mut return_vec = Vec::new();

    for course in courses {
        return_vec.push(CourseDescription{
            title: course.title.clone(),
            identifier: course.identifier.clone(),
            description: course.description.clone(),
            source: School::NA,
            credits: course.credits.clone(),
            instructors: Vec::new(),
            offered:"".to_string(),
            currently_offered: true,
            prerequisites: String::new(),
            corequisites: String::new(),
            fee: 0,
        })
    }

    return_vec    
}

pub async fn scrape_all_descriptions() -> Result<Vec<CourseDescription>> {
    info!("Scraping Pomona API for current courses");
    //let courses = full_pomona_update().await.unwrap();

    //let converted_courses = convert_courses_to_descs(courses);

    info!("Scraping HMC descriptions");
    let hmc_courses = scrape_url(hmc_url, HarveyMudd).await?;
    info!("Scraping CMC descriptions");
    let cmc_courses = scrape_url(cmc_url, ClaremontMckenna).await?;
    info!("Scraping Scripps descriptions");
    let scripps_courses = scrape_url(scripps_url, Scripps).await?;
    info!("Scraping Pitzer descriptions");
    let pitzer_courses = scrape_url(pitzer_url, Pitzer).await?;
    info!("Scraping Pomona descriptions");
    let pomona_courses = scrape_url(pomona_url, Pomona).await?;

    let mut all_descs = merge_descriptions(vec![
        //converted_courses,
        hmc_courses,
        cmc_courses,
        pomona_courses,
        scripps_courses,
        pitzer_courses,
    ]);

    // Find prerequisites and corequisites
    all_descs = find_reqs(&mut all_descs);
    
    // Find fees
    all_descs = find_fees(&mut all_descs);

    Ok(all_descs)
}

pub fn merge_description_and_courses(
    courses: Vec<Course>,
    descriptions: Vec<CourseDescription>,
) -> (Vec<Course>, Vec<CourseDescription>) {
    let mut courses_vec: Vec<Course> = Vec::new();

    let mut descs_vec: Vec<CourseDescription> = descriptions.clone();

    for course in courses {
        let mut new_course = course.clone();

        // Find the description for this course
        let description = find_description(course, &descriptions);

        if description.is_some() {
            let mut desc = description.unwrap();

            println!("{}", desc.title);

            let index = descs_vec
                .iter()
                .position(|r| &r.title == &desc.title && r.identifier.contains(&desc.identifier))
                .unwrap();

            desc.set_instructors(new_course.get_instructors());
            desc.set_offered();

            if desc.get_source() == &School::NA {
                desc.set_source(new_course.get_school().unwrap_or(School::NA));
            }

            if desc.credits == 0 {
                desc.credits = new_course.get_credits().clone();
            }

            new_course.set_description(desc.description.clone());
            new_course.set_prerequisites(desc.prerequisites.clone());
            new_course.set_corequisites(desc.corequisites.clone());
            new_course.set_fee(desc.fee.clone());

            descs_vec[index] = desc;
        }

        courses_vec.push(new_course);
    }

    let total_added_descs = courses_vec
        .clone()
        .into_iter()
        .filter(|x| x.get_description().len() > 1)
        .count();
    println!(
        "{}/{} courses have descriptions, out of {} catalog entries!",
        total_added_descs,
        courses_vec.len(),
        descs_vec.len()
    );

    // Print number of courses & catalog entries with fees
    let course_fees = courses_vec
        .clone()
        .into_iter()
        .filter(|x| x.get_fee() > 0)
        .count();
    
    let catalog_fees = descs_vec
        .clone()
        .into_iter()
        .filter(|x| x.get_fee() > 0)
        .count();
    
    println!(
        "{} courses and {} catalog entries have fees!",
        course_fees,
        catalog_fees
    );

    // Get reqs from notes
    for mut course in courses_vec.iter_mut() {
        let notes = course.get_notes().clone();

        if notes.contains("Prerequisite") {
            let r = process_req(notes.clone(), "Prerequisite".to_string());
            course.set_notes(r.0);
            course.set_prerequisites(r.1);
        } else if notes.contains("Prereq") {
            let r = process_req(notes.clone(), "Prereq".to_string());
            course.set_notes(r.0);
            course.set_prerequisites(r.1);
        }

        if notes.contains("Corequisite") {
            let r = process_req(notes.clone(), "Corequisite".to_string());
            course.set_notes(r.0);
            course.set_corequisites(r.1);
        } else if notes.contains("Coreq") {
            let r = process_req(notes.clone(), "Coreq".to_string());
            course.set_notes(r.0);
            course.set_corequisites(r.1);
        } else if notes.contains("Co-requisite") {
            let r = process_req(notes.clone(), "Co-requisite".to_string());
            course.set_notes(r.0);
            course.set_corequisites(r.1);
        } else if notes.contains("Co-req") {
            let r = process_req(notes.clone(), "Co-req".to_string());
            course.set_notes(r.0);
            course.set_corequisites(r.1);
        }
    }

    (courses_vec, descs_vec)
}

pub fn find_description(
    course: Course,
    course_descriptions: &Vec<CourseDescription>,
) -> Option<CourseDescription> {
    // Only use from own catalog
    let course_descriptions: Vec<CourseDescription> = course_descriptions
        .iter()
        .filter(|x| Some(x.source.clone()) == course.get_school() || x.source == School::NA)
        .map(|x| x.clone())
        .collect();

    let mut course_title = course
        .get_title()
        .clone()
        .to_lowercase()
        .replace("&", "and");

    if course_title.contains(" - ") {
        course_title = course_title.split("-").collect::<Vec<&str>>()[1]
            .trim()
            .to_string();
    }

    let course_identifier = course.get_identifier();

    // Check for exact match first
    let exact_title_match = course_descriptions
        .iter()
        .filter(|x| &x.title.to_lowercase() == &course_title)
        .collect::<Vec<&CourseDescription>>();

    if exact_title_match.is_empty() {
        // Find all matching identifiers
        let matching_identifiers: Vec<CourseDescription> = course_descriptions
            .iter()
            .cloned()
            .filter(|d| course_identifier.contains(&d.identifier))
            .collect();

        // If none, search the entire list for titles:
        if matching_identifiers.is_empty() {
            // Then find all matching titles
            let matching_titles = find_fuzzy_title(&course_title, &course_descriptions);

            // If none, return None
            if matching_titles.is_empty() {
                return None;
            } else {
                // Return best if found
                return Some(
                    course_descriptions
                        .iter()
                        .find(|d| d.title == matching_titles[0])
                        .unwrap()
                        .clone(),
                );
            }
        } else {
            if matching_identifiers.len() == 1 {
                return Some(matching_identifiers[0].clone());
            } else {
                // Otherwise, search the matching identifiers for titles
                let matching_titles = find_fuzzy_title(&course_title, &matching_identifiers);

                // If none, search through all descriptions for titles
                if matching_titles.is_empty() {
                    let matching_titles = find_fuzzy_title(&course_title, &course_descriptions);

                    // If none, return None
                    if matching_titles.is_empty() {
                        return None;
                    } else {
                        // Return best if found
                        return Some(
                            course_descriptions
                                .iter()
                                .find(|d| d.title == matching_titles[0])
                                .unwrap()
                                .clone(),
                        );
                    }
                } else {
                    // Otherwise, return best if found
                    return Some(
                        course_descriptions
                            .iter()
                            .find(|d| d.title == matching_titles[0])
                            .unwrap()
                            .clone(),
                    );
                }
            }
        }
    } else {
        return Some(exact_title_match[0].clone());
    }
}

pub fn find_fuzzy_title(title: &str, search_list: &Vec<CourseDescription>) -> Vec<String> {
    let list = search_list
        .iter()
        .map(|x| x.title.as_str())
        .collect::<Vec<&str>>();

    let matching_titles = fuzzy_search_best_n(&title, &list, 1);

    return matching_titles
        .iter()
        .map(|x| x.0.to_string())
        .collect::<Vec<String>>();
}

pub fn merge_courses(previous: Vec<Course>, new: Vec<Course>) -> Vec<Course> {
    let mut return_vec: Vec<Course> = Vec::new();
    info!("Running over {} x {}", previous.len(), new.len());
    for new_course in new {
        let mut final_course = new_course.clone();

        for previous_course in &previous {
            if final_course.get_title() == previous_course.get_title() {
                if final_course.get_description().len() < previous_course.get_description().len() {
                    final_course.set_description(previous_course.get_description());
                }

                if final_course.get_notes() != previous_course.get_notes() {
                    final_course.set_notes(previous_course.get_notes().to_string());
                }

                if final_course.get_prerequisites().len()
                    < previous_course.get_prerequisites().len()
                {
                    final_course.set_prerequisites(previous_course.get_prerequisites());
                }

                if final_course.get_corequisites().len() < previous_course.get_corequisites().len()
                {
                    final_course.set_corequisites(previous_course.get_corequisites());
                }
            }
        }

        return_vec.push(final_course);
    }

    return_vec
}
