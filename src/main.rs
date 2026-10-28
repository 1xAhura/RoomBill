use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
struct Person {
    id: String,
    name: String,
    card_number: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
struct Purchase {
    id: String,
    description: String,
    total_amount: i64,
    paid_by: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
struct Share {
    id: String,
    purchase_id: String,
    person_id: String,
    amount: i64,
    is_paid: i64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let options = SqliteConnectOptions::from_str("sqlite://data.db")?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

    init_db(&pool).await?;

    let app = Router::new()
        .route("/", get(get_database))
        .route("/persons", get(list_persons).post(add_person))
        .route("/purchases", get(list_purchases).post(add_purchase))
        .route("/shares/{id}/pay", post(pay_share))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS persons (
            id          TEXT PRIMARY KEY NOT NULL,
            name        TEXT NOT NULL,
            card_number TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS purchases (
            id           TEXT PRIMARY KEY NOT NULL,
            description  TEXT NOT NULL,
            total_amount INTEGER NOT NULL,
            paid_by      TEXT NOT NULL,
            created_at   TEXT NOT NULL,
            FOREIGN KEY (paid_by) REFERENCES persons(id)
        );

        CREATE TABLE IF NOT EXISTS shares (
            id          TEXT PRIMARY KEY NOT NULL,
            purchase_id TEXT NOT NULL,
            person_id   TEXT NOT NULL,
            amount      INTEGER NOT NULL,
            is_paid     INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (purchase_id) REFERENCES purchases(id),
            FOREIGN KEY (person_id)   REFERENCES persons(id)
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_database(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let persons = sqlx::query_as::<_, Person>("SELECT * FROM persons")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let purchases = sqlx::query_as::<_, Purchase>("SELECT * FROM purchases")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let shares = sqlx::query_as::<_, Share>("SELECT * FROM shares")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    let db = serde_json::json!({
        "persons": persons,
        "purchases": purchases,
        "shares": shares,
    });

    (StatusCode::OK, Json(db))
}

async fn list_persons(State(pool): State<SqlitePool>) -> impl IntoResponse {
    match sqlx::query_as::<_, Person>("SELECT * FROM persons")
        .fetch_all(&pool)
        .await
    {
        Ok(persons) => (StatusCode::OK, Json(persons)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct NewPerson {
    name: String,
    card_number: String,
}

async fn add_person(
    State(pool): State<SqlitePool>,
    Json(payload): Json<NewPerson>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();

    let result = sqlx::query("INSERT INTO persons (id, name, card_number) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&payload.name)
        .bind(&payload.card_number)
        .execute(&pool)
        .await;

    match result {
        Ok(_) => {
            let person = Person {
                id,
                name: payload.name,
                card_number: payload.card_number,
            };
            (StatusCode::CREATED, Json(person)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn list_purchases(State(pool): State<SqlitePool>) -> impl IntoResponse {
    match sqlx::query_as::<_, Purchase>("SELECT * FROM purchases")
        .fetch_all(&pool)
        .await
    {
        Ok(purchases) => (StatusCode::OK, Json(purchases)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct NewPurchase {
    description: String,
    total_amount: i64,
    paid_by: Uuid,
    split_between: Vec<Uuid>,
}

async fn add_purchase(
    State(pool): State<SqlitePool>,
    Json(payload): Json<NewPurchase>,
) -> impl IntoResponse {
    if payload.split_between.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "split_between cannot be empty" })),
        )
            .into_response();
    }

    let paid_by_str = payload.paid_by.to_string();
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM persons WHERE id = ?")
        .bind(&paid_by_str)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    if exists.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "paid_by person does not exist" })),
        )
            .into_response();
    }

    let purchase_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let amount = payload.total_amount / payload.split_between.len() as i64;

    if let Err(e) = sqlx::query(
        "INSERT INTO purchases (id, description, total_amount, paid_by, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&purchase_id)
    .bind(&payload.description)
    .bind(payload.total_amount)
    .bind(&paid_by_str)
    .bind(&created_at)
    .execute(&pool)
    .await
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response();
    }

    for person_id in &payload.split_between {
        let person_id_str = person_id.to_string();

        let person_exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM persons WHERE id = ?")
                .bind(&person_id_str)
                .fetch_optional(&pool)
                .await
                .unwrap_or(None);

        if person_exists.is_none() {
            continue;
        }

        let share_id = Uuid::new_v4().to_string();
        let is_paid: i64 = if *person_id == payload.paid_by { 1 } else { 0 };

        let _ = sqlx::query(
            "INSERT INTO shares (id, purchase_id, person_id, amount, is_paid) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&share_id)
        .bind(&purchase_id)
        .bind(&person_id_str)
        .bind(amount)
        .bind(is_paid)
        .execute(&pool)
        .await;
    }

    let purchase = Purchase {
        id: purchase_id,
        description: payload.description,
        total_amount: payload.total_amount,
        paid_by: paid_by_str,
        created_at,
    };

    (StatusCode::CREATED, Json(purchase)).into_response()
}

async fn pay_share(State(pool): State<SqlitePool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let id_str = id.to_string();

    let result = sqlx::query("UPDATE shares SET is_paid = 1 WHERE id = ?")
        .bind(&id_str)
        .execute(&pool)
        .await;

    match result {
        Ok(res) if res.rows_affected() > 0 => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "paid" })),
        )
            .into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "share not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
