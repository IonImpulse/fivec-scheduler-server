use crate::*;
use actix_web::*;
use ::serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ReturnCourses {
    timestamp: u64,
    courses: Vec<Course>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReturnCourseList {
    code: String,
    courses: SharedCourseList,
}

/// A simple cache for courses
/// @returns all courses in the cache for all schools at current term
#[get("/fullUpdate")]
pub async fn update_all_courses(path: web::Path<()>) -> HttpResponse {
    let lock = MEMORY_DATABASE.lock().await;

    let courses = lock.course_cache.clone();
    let last_change = lock.last_change.clone();

    drop(lock);

    HttpResponse::Ok().json(ReturnCourses{ timestamp:last_change, courses })
}

/// Since the course cache updates only occasionally, this endpoint is used to
/// update only if the local cache is out of date.
#[get("/updateIfStale/{unix_timestamp_seconds}")]
pub async fn update_if_stale(path: web::Path<u64>) -> HttpResponse {
    let unix_timestamp_seconds = path.into_inner();

    let lock = MEMORY_DATABASE.lock().await;

    if &lock.last_change != &unix_timestamp_seconds {
        info!("Serving course update!");
        let courses = lock.course_cache.clone();
        let last_change = lock.last_change.clone();

        drop(lock);

        HttpResponse::Ok().json(ReturnCourses{ timestamp:last_change, courses })
    } else {
        info!("No course update needed!");
        HttpResponse::Ok().json("No update needed")
    }
}

#[post("/getUniqueCode")]
pub async fn get_unique_code(post: web::Json<(Vec<Course>, Vec<Course>)>,) -> HttpResponse {
    let course_list_tuple = post.into_inner();
    
    let mut local_courses = course_list_tuple.0;
    let mut custom_courses = course_list_tuple.1;

    local_courses.sort_by(|a, b| a.get_identifier().cmp(&b.get_identifier()));
    custom_courses.sort_by(|a, b| a.get_identifier().cmp(&b.get_identifier()));

    let shared_course_list = SharedCourseList::new(local_courses, custom_courses);

    let lock = MEMORY_DATABASE.lock().await;

    let code_cache = lock.code_cache.clone();

    drop(lock);

    let (code, updated_code_cache) = generate_unique_code(shared_course_list, code_cache);

    let mut lock = MEMORY_DATABASE.lock().await;

    lock.code_cache = updated_code_cache;

    drop(lock);

    HttpResponse::Ok().json(code)
}

#[get("/getCourseListByCode/{code}")]
pub async fn get_course_list_by_code(path: web::Path<String>) -> HttpResponse {
    let code = path.into_inner().to_uppercase();
    
    let lock = MEMORY_DATABASE.lock().await;

    let code_cache = lock.code_cache.clone();

    drop(lock);

    let result = get_course_list(code.clone(), code_cache);

    match result {
        Some(result) => HttpResponse::Ok().json(ReturnCourseList { code, courses: result }),
        None => HttpResponse::Ok().json("Invalid code"),
    }
}