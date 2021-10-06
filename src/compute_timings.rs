use crate::course_api::*;
use std::convert::TryInto;

pub fn compute_timings(courses: Vec<Course>) -> (Vec<Vec<i64>>, Vec<Vec<i64>>) {
    let mut day_timings: Vec<i64> = Vec::new();
    
    for _ in 0..24 {
        for _ in 0..60 {
            day_timings.push(0);
        }
    }

    let mut timings: Vec<Vec<i64>> = Vec::new();

    for _ in 0..7 {
        timings.push(day_timings.clone());
    }

    let mut start_timings = timings.clone();
    let mut end_timings = timings.clone();

    for course in courses {
        for timing in course.get_timings() {
            for day in timing.get_days() {
                start_timings[day.to_index()][timing.get_start_time_index() as usize] += course.get_seats_taken();
                end_timings[day.to_index()][timing.get_end_time_index() as usize] += course.get_seats_taken();
            }
        }
    }

    (start_timings, end_timings)
}