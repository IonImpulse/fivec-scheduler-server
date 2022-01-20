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
use std::collections::HashMap;
use std::error::Error;
use std::f32::consts::PI;
use std::ops::Index;

// Simple pair that can be used to merge into actual course data
// "Classic Title" refers to the college's way of identifying the course:
// "ASAM126 HM" instead of "ASAM-126-HM-{section_num}"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourseDescription {
    pub title: String,
    pub identifier: String,
    pub description: String,
    pub source: School,
}

impl CourseDescription {
    pub fn new(
        title: String,
        identifier: String,
        description: String,
        source: School,
    ) -> CourseDescription {
        CourseDescription {
            title,
            identifier,
            description,
            source,
        }
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

pub fn extract_title_description_pair(
    html: String,
    style: School,
) -> Result<Vec<CourseDescription>> {
    let html_vec = html.split("\n").filter(|x| x.contains("</a></h3><h3>"));

    let mut return_vec: Vec<CourseDescription> = Vec::new();

    for line in html_vec {
        let current_line = line.replace("</a></h3><h3>", "");

        let mut split_line = current_line.split("</h3>").collect::<Vec<&str>>();

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

        let description;

        if &style == &HarveyMudd {
            let temp = split_line[1].split("<strong>Description:</strong>").nth(1);
            if temp.is_none() {
                return Ok(return_vec);
            } else {
                description = temp
                    .unwrap()
                    .split("<br><br>")
                    .nth(0)
                    .unwrap()
                    .trim()
                    .to_string();
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
            }
        } else if &style == &ClaremontMckenna {
            description = split_line[1]
                .split("<br><br>")
                .nth(0)
                .unwrap()
                .trim()
                .to_string();
        } else if &style == &Pomona {
            if split_line[1].to_lowercase().contains("when offered")
                && split_line[1].contains("<br><br>")
            {
                description = split_line[1]
                    .split("<br><br>")
                    .nth(1)
                    .unwrap()
                    .split("</strong><br>")
                    .nth(0)
                    .unwrap()
                    .trim()
                    .to_string();
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

        // Remove HTML while preserving what is inside of
        // links/bolded
        lazy_static! {
            static ref RE_HTML: Regex = Regex::new(r"<[^>]+>").unwrap();
            static ref RE_SPACES: Regex = Regex::new(r"\s+").unwrap();
        }

        let description = RE_HTML.replace_all(&description, "").to_string();

        // Replace all multiple spaces with a single one
        let description = RE_SPACES.replace_all(&description, " ").to_string();

        // Remove HTML entities
        let description = match decode_html(&description) {
            Err(_) => description,
            Ok(s) => s,
        };

        info!("[{}] [{}]", title, identifier);

        return_vec.push(CourseDescription::new(
            title,
            identifier,
            description.to_string(),
            style.clone(),
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

        let extracted_pairs = extract_title_description_pair(response, style.clone());

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
        let mut found = return_map
            .iter()
            .position(|r| r.title == course_desc.title && r.identifier == course_desc.identifier);
        if found.is_some() {
            if return_map[found.unwrap()].description.len() < course_desc.description.len() {
                return_map[found.unwrap()] = course_desc;
            }
        } else {
            return_map.push(course_desc);
        }
    }

    return_map
}

pub async fn scrape_all_descriptions() -> Result<Vec<CourseDescription>> {
    info!("Scraping HMC descriptions");
    let hmc_courses = scrape_url(hmc_url, HarveyMudd).await?;
    info!("Scraping CMC descriptions");
    let cmc_courses = scrape_url(cmc_url, ClaremontMckenna).await?;
    info!("Scraping Pomona descriptions");
    let pomona_courses = scrape_url(pomona_url, Pomona).await?;
    info!("Scraping Scripps descriptions");
    let scripps_courses = scrape_url(scripps_url, Scripps).await?;
    info!("Scraping Pitzer descriptions");
    let pitzer_courses = scrape_url(pitzer_url, Pitzer).await?;

    Ok(merge_descriptions(vec![
        hmc_courses,
        cmc_courses,
        pomona_courses,
        scripps_courses,
        pitzer_courses,
    ]))
}

pub fn merge_description_into_courses(
    courses: Vec<Course>,
    descriptions: Vec<CourseDescription>,
) -> Vec<Course> {
    let mut return_vec: Vec<Course> = Vec::new();

    for course in courses {
        let mut new_course = course.clone();

        // Find the description for this course
        let description = find_description(course, &descriptions);

        if description.is_some() {
            new_course.set_description(description.unwrap().description);
        }

        return_vec.push(new_course);
    }

    let total_added_descs = return_vec
        .clone()
        .into_iter()
        .filter(|x| x.get_description().len() > 1)
        .count();
    println!(
        "{}/{} courses have descriptions!",
        total_added_descs,
        return_vec.len()
    );

    return_vec
}

pub fn find_description(
    course: Course,
    course_descriptions: &Vec<CourseDescription>,
) -> Option<CourseDescription> {
    // Only use from own catalog
    let course_descriptions: Vec<CourseDescription> = course_descriptions
        .iter()
        .filter(|x| Some(x.source.clone()) == course.get_school())
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

    let course_identifier = course.get_desc_scrape_str();

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
            .filter(|d| d.identifier == course_identifier)
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
            // Otherwise, search the matching identifiers for titles
            let matching_titles = find_fuzzy_title(&course_title, &matching_identifiers);

            if course_title.contains("dante") {
                println!("{:?}", matching_titles);
            }

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
            }
        }

        return_vec.push(final_course);
    }

    return_vec
}
