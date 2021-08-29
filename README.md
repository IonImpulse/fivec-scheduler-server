# 5cheduler Server & API [WIP]
This repository contains the code that runs in a VPS and server www.5cheduler.com.
Additionally, this server also exposes an API that anyone can use for their own projects (within reason).
Currently, the following API methods are available:
### `GET` /update
`@returns` a JSON object with every single course in the Claremont colleges, guaranteed freshness of 1 minute or less.
Example entry in JSON object returned:
```json
{
    "id":"010A",
    "code":"AFRI",
    "dept":"AF",
    "section":"01",
    "title":"Intro to Africana Studies",
    "max_seats":20,
    "seats_taken":18,
    "seats_remaining":2,
    "credits":0,
    "status":"Closed",
    "timing":
        [
            {"days":
                ["Tuesday","Thursday"],
                "start_time":"09:35:00",
                "end_time":"10:50:00",
                "location":
                    {"school":"Pomona","building":"LeBus Court","room":"113"}
            }
        ],
    "instructors":["Finley, Jessyka"],
    "notes":"Instructor permission required.",
    "description":""
}
```