use rocket::{http::Status, Route};

pub fn routes() -> Vec<Route> {
    routes![ping]
}

#[get("/")]
fn ping() -> Status {
    Status::NoContent
}
