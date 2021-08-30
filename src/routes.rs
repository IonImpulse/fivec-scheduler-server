use crate::*;
use actix_web::*;

/// A simple cache for courses
/// @returns all courses in the cache for all schools at current term
#[get("/fullupdate")]
pub async fn update_all_courses(path: web::Path<()>) -> HttpResponse {
    let lock = MEMORY_DATABASE.lock().await;

    let courses = lock.course_cache.clone();
    let last_change = lock.last_change.clone();

    drop(lock);

    HttpResponse::Ok().json((last_change, courses))
}

#[get("/updateIfStale/{unix_timestamp_seconds}")]
pub async fn update_if_stale(path: web::Path<u64>) -> HttpResponse {
    let unix_timestamp_seconds = path.into_inner();

    let lock = MEMORY_DATABASE.lock().await;

    if &lock.last_change != &unix_timestamp_seconds {
        let courses = lock.course_cache.clone();
        let last_change = lock.last_change.clone();

        drop(lock);

        HttpResponse::Ok().json((last_change, courses))
    } else {
        HttpResponse::Ok().json("No update needed")
    }
}