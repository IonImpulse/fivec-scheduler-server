// ----------------------------------------------------------------------------
// This file contains the code for scraping the descriptions from all
// 5 different colleges. The 2 graduate universities are not scraped.
//
// This will take a lot of time to do.
//
// Hopefully this solves the problem of the 5C's not having a public API
// that actually updates EVERYTHING.
// ----------------------------------------------------------------------------

use std::error::Error;
use log::{info, error};
use reqwest::*;
use std::collections::HashMap;
use crate::course_api::*;

// Simple pair that can be used to merge into actual course data
// "Classic Identifier" refers to the college's way of identifying the course:
// "ASAM126 HM" instead of "ASAM-126-HM-{section_num}"
pub struct IdentifierDescriptionPair {
    pub classic_identifier: String,
    pub description: String,
}

impl IdentifierDescriptionPair {
    pub fn new(
        classic_identifier: String,
        description: String,
    ) -> IdentifierDescriptionPair {
        IdentifierDescriptionPair {
            classic_identifier,
            description,
        }
    }
}

const SPLIT_AT: &str = "<td colspan=\"2\">&#160;</td>";

// Serves only HMC classes
pub fn hmc_url(page_num: u64) -> String {
    format!("http://catalog.hmc.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=18&expand=1&navoid=892&print=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn cmc_url(page_num: u64) -> String {
    format!("http://catalog.claremontmckenna.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=29&expand=1&navoid=4499&print=1&filter[exact_match]=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn pomona_url(page_num: u64) -> String {
    format!("http://catalog.pomona.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=40&expand=1&navoid=8092&print=1&filter[exact_match]=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn scripps_url(page_num: u64) -> String {
    format!("http://catalog.scrippscollege.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=25&expand=1&navoid=3143&print=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn pitzer_url(page_num: u64) -> String {
    format!("http://catalog.pitzer.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=17&expand=1&navoid=1376&print=1&filter[exact_match]=1#acalog_template_course_filter", page_num)
}

pub fn extract_identifier_description_pair(
    html: String,
    style: u8,
) -> Result<Vec<IdentifierDescriptionPair>> {
    let bottom_half = html.split(SPLIT_AT).nth(1).unwrap();
    let usable_html = bottom_half.split(SPLIT_AT).nth(0).unwrap();

    let html_vec = usable_html
        .split("\n")
        .filter(|x| x.contains("</a></h3><h3>"));

    let mut return_vec: Vec<IdentifierDescriptionPair> = Vec::new();

    for line in html_vec {
        let current_line = line.replace("</a></h3><h3>", "");

        let mut split_line = current_line.split("</h3>").collect::<Vec<&str>>();

        let identifier = split_line
            .remove(0)
            .split(" - ")
            .nth(0)
            .unwrap()
            .trim()
            .to_string();
        
        
        if split_line.len() == 0 || identifier.replace("-", "").trim() == "" {
            return Ok(return_vec)
        }

        let description;

        if style == 0 {
            let temp = split_line[0]
                .split("<strong>Description:</strong>")
                .nth(1);
            if temp.is_none() {
                return Ok(return_vec)
            } else {
                description = temp
                    .unwrap()
                    .split("<br><br>")
                    .nth(0)
                    .unwrap()
                    .trim()
                    .to_string();
            }
        } else if style == 1 {
            let temp = split_line[0]
                .split("<br><br>")
                .nth(0);

            if temp.is_none() {
                return Ok(return_vec)
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
            
        } else if style == 2 {
            description = split_line[0]
                .split("<br><br>")
                .nth(0) 
                .unwrap()
                .trim()
                .to_string();

        } else if style == 3 {
            if split_line[0].to_lowercase().contains("when offered") && split_line[0].contains("<br><br>") {
                description = split_line[0]
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
        
        } else if style == 4 {
            let temp = split_line[0]
                .split("<strong>Description:</strong>")
                .nth(1);
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
        
        info!("[{}]\n{}\n", identifier, description);

        return_vec.push(IdentifierDescriptionPair::new(identifier, description));
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

pub async fn scrape_url(url_fn: fn(u64) -> String, style: u8) -> Result<Vec<IdentifierDescriptionPair>> {
    let mut return_vec = Vec::new();
    let mut page_num = 1;
    let mut continue_scraping = true;

    while continue_scraping {
        info!("Scraping page {}", page_num);
        let response = reqwest_get_ignore_ssl(&url_fn(page_num))
            .await?
            .text()
            .await?;

        let extracted_pairs = extract_identifier_description_pair(response, style);
        
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

pub fn merge_descriptions(schools_vec: Vec<Vec<IdentifierDescriptionPair>>) -> HashMap<String, String> {
    let mut return_map: HashMap<String, String> = HashMap::new();
    let schools_vec_flat = schools_vec.into_iter().flatten();

    for pair in schools_vec_flat {
        if return_map.contains_key(&pair.classic_identifier) {
            if return_map.get(&pair.classic_identifier).unwrap().len() < pair.description.len() {
                return_map.insert(pair.classic_identifier, pair.description);
            }
        } else {
            return_map.insert(pair.classic_identifier, pair.description);
        }
    }

    return_map
}

pub async fn scrape_all_descriptions() -> Result<HashMap<String, String>> {
    let mut return_vec: Vec<IdentifierDescriptionPair> = Vec::new();

    info!("Scraping HMC descriptions");
    let hmc_courses = scrape_url(hmc_url, 0).await?;
    info!("Scraping CMC descriptions");
    let cmc_courses = scrape_url(cmc_url, 2).await?;
    info!("Scraping Pomona descriptions");
    let pomona_courses = scrape_url(pomona_url, 3).await?;
    info!("Scraping Scripps descriptions");
    let scripps_courses = scrape_url(scripps_url, 1).await?;
    info!("Scraping Pitzer descriptions");
    let pitzer_courses = scrape_url(pitzer_url, 4).await?;

    Ok(merge_descriptions(vec![hmc_courses, cmc_courses, pomona_courses, scripps_courses, pitzer_courses]))
}

pub fn merge_description_into_courses(
    courses: Vec<Course>,
    descriptions: HashMap<String, String>,
) -> Vec<Course> {
    let mut return_vec: Vec<Course> = Vec::new();

    for course in courses {
        if descriptions.contains_key(&course.get_desc_api_str()) {
            let mut new_course = course.clone();
            new_course.set_description(descriptions.get(&course.get_desc_api_str()).unwrap().to_string());
            return_vec.push(new_course);
        } else {
            return_vec.push(course);
        }
    }

    return_vec
}

pub fn merge_courses(previous: Vec<Course>, new: Vec<Course>) -> Vec<Course> {
    let mut return_vec: Vec<Course> = Vec::new();
    info!("Running over {} x {}", previous.len(), new.len());
    for new_course in new {
        for previous_course in &previous {
            if new_course.get_identifier() == previous_course.get_identifier() {
                let mut final_course = new_course.clone();
                info!("[{}]", final_course.get_identifier());
                if new_course.get_description().len() < previous_course.get_description().len() {
                    final_course.set_description(previous_course.get_description());
                    info!("Setting new description!");
                }

                return_vec.push(final_course);
            }
        }
    }

    return_vec
}