#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate rusqlite;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::prelude::*;
use rocket::State;
use rocket_contrib::json::{Json, JsonValue};
use rusqlite::{named_params, params, Connection};

// The type to represent the ID of a message.
type ID = usize;
// We're going to store all of the messages here. No need for a DB.
type MessageMap = Mutex<HashMap<ID, String>>;
type DbConn = Mutex<Connection>;

#[derive(Serialize, Deserialize)]
struct Message {
    id: Option<ID>,
    contents: String,
}

#[derive(Serialize, Deserialize)]
struct DbMessage {
    id: i64,
    contents: String,
}

#[derive(Serialize, Deserialize)]
struct TimeMessage {
    hour: String,
    date: String,
}

fn init_database(conn: &Connection) {
    conn.execute(
        "CREATE TABLE entries (
            id      INTEGER PRIMARY KEY,
            name    TEXT NOT NULL
        )",
        params![],
    )
    .expect("create entries table");

    conn.execute_named(
        "INSERT INTO entries (id, name) VALUES (:id, :name)",
        named_params! {":id":&0, ":name": &"Rocketeer"},
    )
    .expect("insert single entry into entries table");
}

#[post("/<id>", format = "json", data = "<message>")]
fn new(id: ID, message: Json<Message>, map: State<MessageMap>) -> JsonValue {
    let mut hashmap = map.lock().expect("map lock.");
    if hashmap.contains_key(&id) {
        json!({
            "status": "error",
            "reason": "ID exists. Try put."
        })
    } else {
        hashmap.insert(id, message.0.contents);
        json!({ "status": "ok" })
    }
}

#[put("/<id>", format = "json", data = "<message>")]
fn update(id: ID, message: Json<Message>, map: State<MessageMap>) -> Option<JsonValue> {
    let mut hashmap = map.lock().unwrap();
    if hashmap.contains_key(&id) {
        hashmap.insert(id, message.0.contents);
        Some(json!({ "status": "ok" }))
    } else {
        None
    }
}

#[get("/<id>", format = "json")]
fn get(id: ID, map: State<MessageMap>) -> Option<Json<Message>> {
    let hashmap = map.lock().unwrap();
    hashmap.get(&id).map(|contents| {
        Json(Message {
            id: Some(id),
            contents: contents.clone(),
        })
    })
}

#[get("/time")]
fn get_time() -> JsonValue {
    let utc: DateTime<Utc> = Utc::now();
    json!(TimeMessage {
        hour: utc.format("%H:%M:%S").to_string(),
        date: utc.format("%Y-%m-%d").to_string(),
    })
}

#[get("/db")]
fn get_db(db_conn: State<DbConn>) -> JsonValue {
    let qr = db_conn.lock().expect("db connection lock").query_row(
        "SELECT id, name FROM entries",
        params![],
        |row| {
            Ok(DbMessage {
                id: row.get(0)?,
                contents: row.get(1)?,
            })
        },
    );
    json!(qr.unwrap())
}

#[catch(404)]
fn not_found() -> JsonValue {
    json!({
        "status": "error",
        "reason": "Resource was not found."
    })
}

fn rocket() -> rocket::Rocket {
    let conn = Connection::open_in_memory().expect("in memory db");
    init_database(&conn);
    rocket::ignite()
        .mount("/message", routes![new, update, get])
        .mount("/", routes![get_time, get_db])
        .register(catchers![not_found])
        .manage(Mutex::new(HashMap::<ID, String>::new()))
        .manage(Mutex::new(conn))
}

fn main() {
    rocket().launch();
}
