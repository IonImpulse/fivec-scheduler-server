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

/// Since the course cache updates only occasionally, this endpoint is used to
/// update only if the local cache is out of date.
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

#[post("/getUniqueCode")]
pub async fn get_unique_code(post: web::Json<Vec<Course>>,) -> HttpResponse {
    let course_list = post.into_inner();
    
    let lock = MEMORY_DATABASE.lock().await;

    let code_cache = lock.code_cache.clone();

    drop(lock);

    let (code, updated_code_cache) = generate_unique_code(course_list, code_cache);

    let mut lock = MEMORY_DATABASE.lock().await;

    lock.code_cache = updated_code_cache;

    drop(lock);

    HttpResponse::Ok().json(code)
}

#[get("/getCourseListByCode/{code}")]
pub async fn get_course_list_by_code(path: web::Path<String>) -> HttpResponse {
    let code = path.into_inner();
    
    let lock = MEMORY_DATABASE.lock().await;

    let code_cache = lock.code_cache.clone();

    drop(lock);

    let result = get_course_list(code, code_cache);

    match result {
        Some(result) => HttpResponse::Ok().json(result),
        None => HttpResponse::Ok().json("Invalid code"),
    }
}