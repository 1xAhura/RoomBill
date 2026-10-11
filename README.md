# RoomBill

A simple backend API for managing shared expenses in a dormitory.

When someone buys something for the room, they can log the purchase and split the cost between the residents. Each person can later mark their share as paid.

## Current Features

- Add people (roommates)
- Create a purchase and split it equally between selected people
- Automatically mark the payer's share as paid
- Mark individual shares as paid
- In-memory storage (data is lost on restart)

## Tech Stack

- Rust
- Axum
- Serde
- Chrono
- Uuid

## Running the Project

```bash
cargo run
```

The server starts on `http://127.0.0.1:3000`.

## API Endpoints

| Method | Endpoint              | Description                          |
|--------|-----------------------|--------------------------------------|
| GET    | `/`                   | Return the entire database (debug)   |
| GET    | `/persons`            | List all persons                     |
| POST   | `/persons`            | Add a new person                     |
| GET    | `/purchases`          | List all purchases                   |
| POST   | `/purchases`          | Create a new purchase and its shares |
| POST   | `/shares/{id}/pay`    | Mark a share as paid                 |

### Example: Add a person

```bash
curl -X POST http://127.0.0.1:3000/persons \
  -H "Content-Type: application/json" \
  -d '{"name": "Ali", "card_number": "6037-XXXX-XXXX-XXXX"}'
```

### Example: Create a purchase

```bash
curl -X POST http://127.0.0.1:3000/purchases \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Toilet paper and detergent",
    "total_amount": 120000,
    "paid_by": "PERSON_UUID_HERE",
    "split_between": ["UUID1", "UUID2", "UUID3", "UUID4"]
  }'
```

### Example: Mark a share as paid

```bash
curl -X POST http://127.0.0.1:3000/shares/SHARE_UUID_HERE/pay
```

## Project Structure

The current version keeps all data in memory using an `Arc<Mutex<Database>>`. This is intentional for the first version and will be replaced with a real database.

## Roadmap

### Phase 1 – Persistence
- [ ] Migrate from in-memory storage to SQLite
- [ ] Use SQLx for database access
- [ ] Add database migrations

### Phase 2 – Better API
- [ ] List shares of a specific person
- [ ] Calculate current balances (who owes whom)
- [ ] Better error handling and validation
- [ ] Support custom (non-equal) splits

### Phase 3 – Authentication
- [ ] User registration and login
- [ ] JWT or session-based authentication
- [ ] Protect endpoints so users only see relevant data

### Phase 4 – Frontend
- [ ] Simple web interface
- [ ] Ability to create purchases and mark shares as paid without manually handling UUIDs

### Phase 5 – Polish
- [ ] Jalali (Solar) date support in responses
- [ ] Deployment
- [ ] Basic tests