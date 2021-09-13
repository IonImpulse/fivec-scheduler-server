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
pub fn HMC_URL(page_num: u64) -> String {
    format!("catalog.hmc.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=18&expand=1&navoid=892&print=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn CMC_URL(page_num: u64) -> String {
    format!("https://catalog.claremontmckenna.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=29&expand=1&navoid=4499&print=1&filter[exact_match]=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn POMONA_URL(page_num: u64) -> String {
    format!("https://catalog.pomona.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=40&expand=1&navoid=8092&print=1&filter[exact_match]=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn SCRIPPS_URL(page_num: u64) -> String {
    format!("catalog.scrippscollege.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=25&expand=1&navoid=3143&print=1#acalog_template_course_filter", page_num)
}

// Serves ALL classes, but redirects to other colleges for descriptions
pub fn PITZER_URL(page_num: u64) -> String {
    format!("catalog.pitzer.edu/content.php?filter[27]=-1&filter[29]=&filter[course_type]=-1&filter[keyword]=&filter[32]=1&filter[cpage]={}&cur_cat_oid=17&expand=1&navoid=1376&print=1&filter[exact_match]=1#acalog_template_course_filter", page_num)
}

pub fn scrape_description_html(
    html: String,
) -> Result<Vec<IdentifierDescriptionPair>, Box<dyn Error>> {
    let bottom_half = html.split(SPLIT_AT).nth(1).unwrap();
    let usable_html = bottom_half.split(SPLIT_AT).nth(0).unwrap();

    let html_vec = usable_html
        .split("\n")
        .filter(|x| x.contains("Description") && !x.contains("for course description."));

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

        let description = split_line[0]
            .split("<strong>Description:</strong>")
            .nth(0)
            .unwrap()
            .split("<br><br><strong>Pre")
            .nth(0)
            .unwrap()
            .trim()
            .to_string();

        return_vec.push(IdentifierDescriptionPair::new(identifier, description));
    }

    Ok(return_vec)
}