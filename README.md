# 5scheduler Server & API
This repository contains the code that runs in a VPS and serves https://www.5scheduler.io.
Additionally, this server also exposes an API that anyone can use for their own projects (within reason).

Live API at: https://api.5scheduler.io


Currently, the following API methods are available:
### `GET` /fullUpdate
`@returns` a JSON object with every single course in the Claremont colleges, guaranteed freshness of 1 minute or less.

Example:
```json
{
"timestamp": 1632558607,
"courses": [
  COURSE,
  COURSE,
  COURSE,
]
}
```

Where each COURSE is a JSON object, example below:
```json
{
  "identifier": "AFRI-010A-AF-01",
  "id": "010A",
  "code": "AFRI",
  "dept": "AF",
  "section": "01",
  "title": "Intro to Africana Studies",
  "max_seats": 20,
  "seats_taken": 20,
  "seats_remaining": 0,
  "credits": 0,
  "status": "Closed",
  "timing": [
    {
      "days": [
        "Tuesday",
        "Thursday"
      ],
      "start_time": "09:35:00",
      "end_time": "10:50:00",
      "location": {
        "school": "Pomona",
        "building": "LeBus Court",
        "room": "113"
      }
    }
  ],
  "instructors": [
    " Jessyka Finley"
  ],
  "notes": "Instructor permission required.",
  "description": "This class will serve as a general introduction to Africana Studies. Africana studies, while still relatively young, has a vibrant history that traces the lives and scholarship of people from African descent. Its complex and latent development in academia follows from the socio-political marginalization of people within the African diaspora. Nevertheless, resilience and perseverance will be repeated themes as we study how, through different techniques and modes of understanding, people of the African diaspora have continually challenged the western hegemony of academic study."
},
```

**Some things to note about courses**
- Each *identifier* is unique, and should be used to select courses
- The *status* of each course can be "Open", "Closed", or "Reopened"
- In the *timing* list inside the course object, there can be multiple timing objects. As such,
  - Each timing object has a start time and end time formatted as a 24 hour HH:MM:SS timestamp
  - Each timing object has a list of days that those start/end times will apply to, consisting of
    - "Monday", "Tuesday", "Wednesday", "Thursday", or "Friday" 
  - If a class does not have a set time, both the start time and end time will be 00:00:00

### `GET` /updateIfStale/{unix_timestamp}
`@params` timestamp from last update

`@returns` an updated list if there has been a change since the timestamp, otherwise returns *"No update needed"*

### `POST` /getUniqueCode
`@contents` JSON list of courses to get a code for

`@returns` a unique 7-character case-insensitive code that maps to that *exact* course list

Supports custom courses

### `GET` /getCourseListByCode/{code}
`@params` 7-character code

`@returns` JSON list of courses if code exists and is valid, otherwise returns *"Invalid code"*
