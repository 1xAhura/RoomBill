use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Person {
    id: Uuid,
    name: String,
    card_number: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Purchase {
    id: Uuid,
    description: String,
    total_amount: i64,
    paid_by: Uuid,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Share {
    id: Uuid,
    purchase_id: Uuid,
    person_id: Uuid,
    amount: i64,
    is_paid: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Database {
    persons: Vec<Person>,
    purchases: Vec<Purchase>,
    shares: Vec<Share>,
}
type Db = Arc<Mutex<Database>>;

impl Person {
    fn new(name: String, card_number: String) -> Self {
        Person {
            id: Uuid::new_v4(),
            name,
            card_number,
        }
    }
}

impl Purchase {
    fn new(description: String, total_amount: i64, paid_by: Uuid) -> Self {
        Purchase {
            id: Uuid::new_v4(),
            description,
            total_amount,
            paid_by,
            created_at: Utc::now(),
        }
    }
}

impl Share {
    fn new(purchase_id: Uuid, person_id: Uuid, amount: i64, is_paid: bool) -> Self {
        Share {
            id: Uuid::new_v4(),
            purchase_id,
            person_id,
            amount,
            is_paid,
        }
    }
}

#[tokio::main]
async fn main() {
    let db = Arc::new(Mutex::new(Database {
        persons: Vec::new(),
        purchases: Vec::new(),
        shares: Vec::new(),
    }));

    let app = Router::new()
        .route("/", get(get_database))
        .route("/persons", get(list_persons).post(add_person))
        .route("/purchases", get(list_purchases).post(add_purchase))
        .route("/shares/{id}/pay", post(pay_share))
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn get_database(State(db): State<Db>) -> impl IntoResponse {
    let database = db.lock().unwrap().clone();
    (StatusCode::OK, Json(database))
}

async fn list_persons(State(db): State<Db>) -> impl IntoResponse {
    let persons = db.lock().unwrap().persons.clone();
    (StatusCode::OK, Json(persons))
}

#[derive(Debug, Deserialize, Clone)]
struct NewPerson {
    name: String,
    card_number: String,
}

async fn add_person(State(db): State<Db>, Json(person): Json<NewPerson>) -> impl IntoResponse {
    let mut database = db.lock().unwrap();

    database
        .persons
        .push(Person::new(person.name, person.card_number));

    (StatusCode::CREATED, Json(database.persons.clone()))
}

async fn list_purchases(State(db): State<Db>) -> impl IntoResponse {
    let purchases = db.lock().unwrap().purchases.clone();

    (StatusCode::OK, Json(purchases))
}

#[derive(Debug, Deserialize)]
struct NewPurchase {
    description: String,
    total_amount: i64,
    paid_by: Uuid,
    split_between: Vec<Uuid>,
}

async fn add_purchase(
    State(db): State<Db>,
    Json(new_purchase): Json<NewPurchase>,
) -> impl IntoResponse {
    let mut database = db.lock().unwrap();

    // Check 1: split_between must not be empty
    if new_purchase.split_between.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "split_between cannot be empty" })),
        )
            .into_response();
    }

    // Check 2: paid_by must exist
    if !database
        .persons
        .iter()
        .any(|p| p.id == new_purchase.paid_by)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "paid_by person does not exist" })),
        )
            .into_response();
    }

    let purchase = Purchase::new(
        new_purchase.description,
        new_purchase.total_amount,
        new_purchase.paid_by,
    );

    let amount = new_purchase.total_amount / new_purchase.split_between.len() as i64;

    for person_id in new_purchase.split_between {
        if database.persons.iter().any(|p| p.id == person_id) {
            let is_paid = person_id == purchase.paid_by;
            let new_share = Share::new(purchase.id, person_id, amount, is_paid);
            database.shares.push(new_share);
        }
    }

    database.purchases.push(purchase);

    (StatusCode::CREATED, Json(database.purchases.clone())).into_response()
}

async fn pay_share(State(db): State<Db>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let mut database = db.lock().unwrap();

    if let Some(share) = database.shares.iter_mut().find(|s| s.id == id) {
        share.is_paid = true;
        return (StatusCode::OK, Json(database.shares.clone()));
    }

    (StatusCode::NOT_FOUND, Json(Vec::<Share>::new()))
}
