//! strike-hub: accounts, sessions, friends, presence and the match server registry.
//!
//! One binary, one SQLite file, plain HTTP. Everything an operator needs is reachable
//! from the game client; there is no web UI.
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const SESSION_TTL: i64 = 30 * 24 * 3600;
const TICKET_TTL: i64 = 90;
const HEARTBEAT_TTL: i64 = 30;
const PRESENCE_TTL: i64 = 90;
const RESERVED: [&str; 5] = ["admin", "server", "bot", "console", "system"];

#[derive(Clone)]
struct Hub {
    db: Arc<Mutex<Connection>>,
    name: String,
    server_key: String,
    server_bin: Option<String>,
    server_host: String,
    hub_url: String,
    ports: Arc<Mutex<u16>>,
    children: Arc<Mutex<Vec<tokio::process::Child>>>,
}

type HubError = (StatusCode, Json<Value>);

fn err(status: StatusCode, code: &str, message: impl Into<String>) -> HubError {
    (
        status,
        Json(json!({ "error": code, "message": message.into() })),
    )
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn migrate(db: &Connection) {
    db.execute_batch(
        "PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS accounts (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            username_lower TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'player',
            created_at INTEGER NOT NULL,
            last_login_at INTEGER,
            banned_until INTEGER,
            ban_reason TEXT
        );
        CREATE TABLE IF NOT EXISTS sessions (
            token TEXT PRIMARY KEY,
            account_id INTEGER NOT NULL,
            device_label TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            last_seen_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS friendships (
            requester_id INTEGER NOT NULL,
            addressee_id INTEGER NOT NULL,
            state TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (requester_id, addressee_id)
        );
        CREATE TABLE IF NOT EXISTS presence (
            account_id INTEGER PRIMARY KEY,
            state TEXT NOT NULL,
            server_id TEXT,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS servers (
            server_id TEXT PRIMARY KEY,
            host TEXT NOT NULL,
            port INTEGER NOT NULL,
            mode TEXT NOT NULL,
            map TEXT NOT NULL,
            phase TEXT NOT NULL,
            players INTEGER NOT NULL DEFAULT 0,
            max_players INTEGER NOT NULL,
            started_at INTEGER NOT NULL,
            last_heartbeat INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tickets (
            ticket TEXT PRIMARY KEY,
            account_id INTEGER NOT NULL,
            server_id TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            consumed_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS stats (
            account_id INTEGER PRIMARY KEY,
            kills INTEGER NOT NULL DEFAULT 0,
            deaths INTEGER NOT NULL DEFAULT 0,
            matches INTEGER NOT NULL DEFAULT 0,
            wins INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_by INTEGER,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY,
            actor_id INTEGER,
            action TEXT NOT NULL,
            target TEXT,
            payload TEXT,
            at INTEGER NOT NULL
        );",
    )
    .expect("migrations apply");
}

#[derive(Clone, Serialize)]
struct Account {
    id: i64,
    username: String,
    display_name: String,
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
}

fn load_account(db: &Connection, id: i64, with_email: bool) -> Option<Account> {
    db.query_row(
        "SELECT id, username, display_name, role, email FROM accounts WHERE id = ?1",
        params![id],
        |r| {
            Ok(Account {
                id: r.get(0)?,
                username: r.get(1)?,
                display_name: r.get(2)?,
                role: r.get(3)?,
                email: if with_email { Some(r.get(4)?) } else { None },
            })
        },
    )
    .optional()
    .ok()
    .flatten()
}

fn account_by_username(db: &Connection, username: &str) -> Option<i64> {
    db.query_row(
        "SELECT id FROM accounts WHERE username_lower = ?1",
        params![username.to_lowercase()],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

fn authenticate(hub: &Hub, headers: &HeaderMap) -> Result<Account, HubError> {
    let bearer = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "session_expired", "Not signed in"))?;
    let db = hub.db.lock().unwrap();
    let row: Option<(i64, i64)> = db
        .query_row(
            "SELECT account_id, expires_at FROM sessions WHERE token = ?1",
            params![bearer],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .unwrap();
    let Some((account_id, expires)) = row else {
        return Err(err(
            StatusCode::UNAUTHORIZED,
            "session_expired",
            "Session expired, sign in again",
        ));
    };
    if expires < now() {
        db.execute("DELETE FROM sessions WHERE token = ?1", params![bearer])
            .ok();
        return Err(err(
            StatusCode::UNAUTHORIZED,
            "session_expired",
            "Session expired, sign in again",
        ));
    }
    let banned: Option<i64> = db
        .query_row(
            "SELECT banned_until FROM accounts WHERE id = ?1",
            params![account_id],
            |r| r.get(0),
        )
        .unwrap_or(None);
    if banned.is_some_and(|until| until > now()) {
        return Err(err(
            StatusCode::FORBIDDEN,
            "banned",
            "This account is banned",
        ));
    }
    db.execute(
        "UPDATE sessions SET last_seen_at = ?1 WHERE token = ?2",
        params![now(), bearer],
    )
    .ok();
    load_account(&db, account_id, true).ok_or_else(|| {
        err(
            StatusCode::UNAUTHORIZED,
            "session_expired",
            "Account missing",
        )
    })
}

fn require_role(account: &Account, roles: &[&str]) -> Result<(), HubError> {
    if roles.contains(&account.role.as_str()) {
        Ok(())
    } else {
        Err(err(StatusCode::FORBIDDEN, "forbidden", "Not allowed"))
    }
}

fn require_server_key(hub: &Hub, key: &str) -> Result<(), HubError> {
    if key == hub.server_key {
        Ok(())
    } else {
        Err(err(StatusCode::FORBIDDEN, "forbidden", "Bad server key"))
    }
}

fn audit(db: &Connection, actor: Option<i64>, action: &str, target: &str, payload: Value) {
    db.execute(
        "INSERT INTO audit_log (actor_id, action, target, payload, at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![actor, action, target, payload.to_string(), now()],
    )
    .ok();
}

// ---------------------------------------------------------------- auth

#[derive(Deserialize)]
struct RegisterBody {
    username: String,
    email: String,
    password: String,
    #[serde(default)]
    device_label: String,
}

fn validate_username(username: &str) -> Result<(), HubError> {
    let ok = (3..=20).contains(&username.chars().count())
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !RESERVED.contains(&username.to_lowercase().as_str());
    if ok {
        Ok(())
    } else {
        Err(err(
            StatusCode::BAD_REQUEST,
            "invalid_username",
            "Usernames are 3 to 20 letters, digits or underscores",
        ))
    }
}

fn validate_password(password: &str, username: &str) -> Result<(), HubError> {
    let len = password.chars().count();
    if !(8..=128).contains(&len) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "weak_password",
            "Passwords are 8 to 128 characters",
        ));
    }
    if password.to_lowercase().contains(&username.to_lowercase()) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "weak_password",
            "Password must not contain the username",
        ));
    }
    Ok(())
}

fn session_response(
    db: &Connection,
    hub: &Hub,
    account_id: i64,
    device: &str,
    first_admin: bool,
) -> Value {
    let token = token();
    db.execute(
        "INSERT INTO sessions (token, account_id, device_label, created_at, last_seen_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?4, ?5)",
        params![token, account_id, device, now(), now() + SESSION_TTL],
    )
    .unwrap();
    db.execute(
        "UPDATE accounts SET last_login_at = ?1 WHERE id = ?2",
        params![now(), account_id],
    )
    .ok();
    db.execute(
        "INSERT INTO presence (account_id, state, server_id, updated_at) VALUES (?1, 'online', NULL, ?2)
         ON CONFLICT(account_id) DO UPDATE SET state = 'online', updated_at = ?2",
        params![account_id, now()],
    )
    .ok();
    let account = load_account(db, account_id, true).unwrap();
    json!({
        "session": token,
        "expires_at": now() + SESSION_TTL,
        "account": account,
        "hub": { "name": hub.name, "registration": "open" },
        "first_admin": first_admin,
    })
}

async fn register(
    State(hub): State<Hub>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<Value>, HubError> {
    validate_username(&body.username)?;
    let email = body.email.trim().to_lowercase();
    if !email.contains('@') || email.len() < 5 {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "invalid_email",
            "Enter a valid email",
        ));
    }
    validate_password(&body.password, &body.username)?;
    let hash = Argon2::default()
        .hash_password(body.password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map_err(|_| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "Hashing failed",
            )
        })?
        .to_string();
    let db = hub.db.lock().unwrap();
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))
        .unwrap();
    let role = if count == 0 { "admin" } else { "player" };
    if account_by_username(&db, &body.username).is_some() {
        return Err(err(
            StatusCode::CONFLICT,
            "username_taken",
            "That username is taken",
        ));
    }
    let email_taken: Option<i64> = db
        .query_row(
            "SELECT id FROM accounts WHERE email = ?1",
            params![email],
            |r| r.get(0),
        )
        .optional()
        .unwrap();
    if email_taken.is_some() {
        return Err(err(
            StatusCode::CONFLICT,
            "email_taken",
            "That email already has an account",
        ));
    }
    db.execute(
        "INSERT INTO accounts (username, username_lower, email, display_name, password_hash, role, created_at)
         VALUES (?1, ?2, ?3, ?1, ?4, ?5, ?6)",
        params![body.username, body.username.to_lowercase(), email, hash, role, now()],
    )
    .unwrap();
    let id = db.last_insert_rowid();
    db.execute("INSERT INTO stats (account_id) VALUES (?1)", params![id])
        .ok();
    audit(
        &db,
        Some(id),
        "register",
        &body.username,
        json!({ "role": role }),
    );
    Ok(Json(session_response(
        &db,
        &hub,
        id,
        &body.device_label,
        role == "admin",
    )))
}

#[derive(Deserialize)]
struct LoginBody {
    identifier: String,
    password: String,
    #[serde(default)]
    device_label: String,
}

async fn login(
    State(hub): State<Hub>,
    Json(body): Json<LoginBody>,
) -> Result<Json<Value>, HubError> {
    let db = hub.db.lock().unwrap();
    let ident = body.identifier.trim();
    let row: Option<(i64, String, Option<i64>, Option<String>)> = if ident.contains('@') {
        db.query_row(
            "SELECT id, password_hash, banned_until, ban_reason FROM accounts WHERE email = ?1",
            params![ident.to_lowercase()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
    } else {
        db.query_row(
            "SELECT id, password_hash, banned_until, ban_reason FROM accounts WHERE username_lower = ?1",
            params![ident.to_lowercase()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
    }
    .optional()
    .unwrap();
    // Same work whether or not the account exists.
    let dummy = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$4Q7c3F4C0f1rj4xQe2kZ5bZ4vXg1pUmkq3B2i1e7n2A";
    let (id, hash, banned, reason) = row.unwrap_or((0, dummy.to_string(), None, None));
    let parsed = PasswordHash::new(&hash).ok();
    let valid = parsed
        .map(|h| {
            Argon2::default()
                .verify_password(body.password.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false);
    if id == 0 || !valid {
        return Err(err(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "Wrong username, email or password",
        ));
    }
    if banned.is_some_and(|until| until > now()) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(
                json!({ "error": "banned", "message": reason.unwrap_or_else(|| "Banned".into()), "until": banned }),
            ),
        ));
    }
    Ok(Json(session_response(
        &db,
        &hub,
        id,
        &body.device_label,
        false,
    )))
}

async fn me(State(hub): State<Hub>, headers: HeaderMap) -> Result<Json<Value>, HubError> {
    let account = authenticate(&hub, &headers)?;
    Ok(Json(
        json!({ "account": account, "hub": { "name": hub.name } }),
    ))
}

async fn logout(State(hub): State<Hub>, headers: HeaderMap) -> Result<Json<Value>, HubError> {
    let account = authenticate(&hub, &headers)?;
    let token = headers["authorization"].to_str().unwrap()[7..].to_string();
    let db = hub.db.lock().unwrap();
    db.execute("DELETE FROM sessions WHERE token = ?1", params![token])
        .ok();
    db.execute(
        "UPDATE presence SET state = 'offline', server_id = NULL, updated_at = ?1 WHERE account_id = ?2",
        params![now(), account.id],
    )
    .ok();
    Ok(Json(json!({ "ok": true })))
}

async fn health(State(hub): State<Hub>) -> Json<Value> {
    Json(json!({ "name": hub.name, "version": env!("CARGO_PKG_VERSION"), "registration": "open" }))
}

// ---------------------------------------------------------------- friends

fn relationship(db: &Connection, me: i64, other: i64) -> &'static str {
    let row: Option<(i64, String)> = db
        .query_row(
            "SELECT requester_id, state FROM friendships
             WHERE (requester_id = ?1 AND addressee_id = ?2) OR (requester_id = ?2 AND addressee_id = ?1)",
            params![me, other],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .unwrap();
    match row {
        None => "none",
        Some((_, s)) if s == "accepted" => "friends",
        Some((_, s)) if s == "blocked" => "blocked",
        Some((req, _)) if req == me => "pending_out",
        Some(_) => "pending_in",
    }
}

fn presence_of(db: &Connection, id: i64) -> Value {
    let row: Option<(String, Option<String>, i64)> = db
        .query_row(
            "SELECT state, server_id, updated_at FROM presence WHERE account_id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .unwrap();
    match row {
        Some((state, server, at)) if at + PRESENCE_TTL > now() => {
            let server_info = server.as_ref().and_then(|sid| {
                db.query_row(
                    "SELECT mode, map FROM servers WHERE server_id = ?1",
                    params![sid],
                    |r| Ok(json!({ "server_id": sid, "mode": r.get::<_, String>(0)?, "map": r.get::<_, String>(1)? })),
                )
                .optional()
                .unwrap()
            });
            json!({ "state": state, "server": server_info })
        }
        Some((_, _, at)) => json!({ "state": "offline", "last_seen": at }),
        None => json!({ "state": "offline" }),
    }
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn friends_search(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    let q = query.q.trim().to_lowercase();
    if q.chars().count() < 2 || q.contains('@') {
        return Ok(Json(json!({ "results": [] })));
    }
    let db = hub.db.lock().unwrap();
    let mut stmt = db
        .prepare(
            "SELECT id, username, display_name FROM accounts
             WHERE username_lower LIKE ?1 AND id != ?2 AND (banned_until IS NULL OR banned_until < ?3)
             ORDER BY CASE WHEN username_lower LIKE ?4 THEN 0 ELSE 1 END, username_lower LIMIT 10",
        )
        .unwrap();
    let rows: Vec<Value> = stmt
        .query_map(params![format!("%{q}%"), me.id, now(), format!("{q}%")], |r| {
            let id: i64 = r.get(0)?;
            Ok((id, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        })
        .unwrap()
        .flatten()
        .filter_map(|(id, username, display)| {
            let rel = relationship(&db, me.id, id);
            (rel != "blocked").then(|| {
                json!({ "username": username, "display_name": display, "relationship": rel, "presence": presence_of(&db, id) })
            })
        })
        .collect();
    Ok(Json(json!({ "results": rows })))
}

async fn friends_list(State(hub): State<Hub>, headers: HeaderMap) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    let db = hub.db.lock().unwrap();
    let mut stmt = db
        .prepare(
            "SELECT requester_id, addressee_id, state FROM friendships
             WHERE (requester_id = ?1 OR addressee_id = ?1) AND state != 'blocked'",
        )
        .unwrap();
    let mut friends = Vec::new();
    let mut pending_in = Vec::new();
    let mut pending_out = Vec::new();
    for row in stmt
        .query_map(params![me.id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .unwrap()
        .flatten()
    {
        let (req, addr, state) = row;
        let other = if req == me.id { addr } else { req };
        let Some(acc) = load_account(&db, other, false) else {
            continue;
        };
        let entry = json!({ "username": acc.username, "display_name": acc.display_name, "presence": presence_of(&db, other) });
        if state == "accepted" {
            friends.push(entry);
        } else if req == me.id {
            pending_out.push(entry);
        } else {
            pending_in.push(entry);
        }
    }
    Ok(Json(
        json!({ "friends": friends, "pending_in": pending_in, "pending_out": pending_out }),
    ))
}

#[derive(Deserialize)]
struct FriendBody {
    username: String,
}

async fn friends_request(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Json(body): Json<FriendBody>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    let db = hub.db.lock().unwrap();
    let other = account_by_username(&db, &body.username)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "not_found", "No such player"))?;
    if other == me.id {
        return Err(err(StatusCode::BAD_REQUEST, "invalid", "That is you"));
    }
    match relationship(&db, me.id, other) {
        "none" => {
            db.execute(
                "INSERT INTO friendships (requester_id, addressee_id, state, created_at, updated_at)
                 VALUES (?1, ?2, 'pending', ?3, ?3)",
                params![me.id, other, now()],
            )
            .unwrap();
            Ok(Json(json!({ "relationship": "pending_out" })))
        }
        "pending_in" => {
            db.execute(
                "UPDATE friendships SET state = 'accepted', updated_at = ?3 WHERE requester_id = ?2 AND addressee_id = ?1",
                params![me.id, other, now()],
            )
            .unwrap();
            Ok(Json(json!({ "relationship": "friends" })))
        }
        rel => Ok(Json(json!({ "relationship": rel }))),
    }
}

async fn friends_accept(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Json(body): Json<FriendBody>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    let db = hub.db.lock().unwrap();
    let other = account_by_username(&db, &body.username)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "not_found", "No such player"))?;
    let changed = db
        .execute(
            "UPDATE friendships SET state = 'accepted', updated_at = ?3
             WHERE requester_id = ?2 AND addressee_id = ?1 AND state = 'pending'",
            params![me.id, other, now()],
        )
        .unwrap();
    if changed == 0 {
        return Err(err(
            StatusCode::NOT_FOUND,
            "not_found",
            "No pending request from that player",
        ));
    }
    Ok(Json(json!({ "relationship": "friends" })))
}

async fn friends_decline(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Json(body): Json<FriendBody>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    let db = hub.db.lock().unwrap();
    let other = account_by_username(&db, &body.username)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "not_found", "No such player"))?;
    db.execute(
        "DELETE FROM friendships WHERE requester_id = ?2 AND addressee_id = ?1 AND state = 'pending'",
        params![me.id, other],
    )
    .unwrap();
    Ok(Json(json!({ "relationship": "none" })))
}

async fn friends_remove(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    let db = hub.db.lock().unwrap();
    let other = account_by_username(&db, &username)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "not_found", "No such player"))?;
    db.execute(
        "DELETE FROM friendships WHERE (requester_id = ?1 AND addressee_id = ?2) OR (requester_id = ?2 AND addressee_id = ?1)",
        params![me.id, other],
    )
    .unwrap();
    Ok(Json(json!({ "relationship": "none" })))
}

#[derive(Deserialize)]
struct PresenceBody {
    state: String,
}

async fn presence(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Json(body): Json<PresenceBody>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    let db = hub.db.lock().unwrap();
    // A match server's heartbeat owns in_match; the client only reports menu states.
    db.execute(
        "INSERT INTO presence (account_id, state, server_id, updated_at) VALUES (?1, ?2, NULL, ?3)
         ON CONFLICT(account_id) DO UPDATE SET state = CASE WHEN presence.state = 'in_match' AND ?2 != 'in_menu' THEN presence.state ELSE ?2 END,
             server_id = CASE WHEN ?2 = 'in_menu' THEN NULL ELSE presence.server_id END, updated_at = ?3",
        params![me.id, body.state, now()],
    )
    .unwrap();
    Ok(Json(json!({ "ok": true })))
}

// ---------------------------------------------------------------- servers and matchmaking

#[derive(Deserialize)]
struct RegisterServerBody {
    key: String,
    server_id: String,
    host: String,
    port: u16,
    mode: String,
    map: String,
    max_players: u32,
}

async fn server_register(
    State(hub): State<Hub>,
    Json(body): Json<RegisterServerBody>,
) -> Result<Json<Value>, HubError> {
    require_server_key(&hub, &body.key)?;
    let db = hub.db.lock().unwrap();
    db.execute(
        "INSERT INTO servers (server_id, host, port, mode, map, phase, players, max_players, started_at, last_heartbeat)
         VALUES (?1, ?2, ?3, ?4, ?5, 'warmup', 0, ?6, ?7, ?7)
         ON CONFLICT(server_id) DO UPDATE SET host = ?2, port = ?3, mode = ?4, map = ?5, max_players = ?6, last_heartbeat = ?7",
        params![body.server_id, body.host, body.port, body.mode, body.map, body.max_players, now()],
    )
    .unwrap();
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct HeartbeatBody {
    key: String,
    phase: String,
    players: Vec<i64>,
}

async fn server_heartbeat(
    State(hub): State<Hub>,
    Path(id): Path<String>,
    Json(body): Json<HeartbeatBody>,
) -> Result<Json<Value>, HubError> {
    require_server_key(&hub, &body.key)?;
    let db = hub.db.lock().unwrap();
    db.execute(
        "UPDATE servers SET phase = ?2, players = ?3, last_heartbeat = ?4 WHERE server_id = ?1",
        params![id, body.phase, body.players.len() as i64, now()],
    )
    .unwrap();
    for account in &body.players {
        db.execute(
            "INSERT INTO presence (account_id, state, server_id, updated_at) VALUES (?1, 'in_match', ?2, ?3)
             ON CONFLICT(account_id) DO UPDATE SET state = 'in_match', server_id = ?2, updated_at = ?3",
            params![account, id, now()],
        )
        .ok();
    }
    db.execute(
        "UPDATE presence SET state = 'online', server_id = NULL WHERE server_id = ?1 AND account_id NOT IN (SELECT value FROM json_each(?2))",
        params![id, serde_json::to_string(&body.players).unwrap()],
    )
    .ok();
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct VerifyBody {
    key: String,
    ticket: String,
}

async fn server_verify_ticket(
    State(hub): State<Hub>,
    Path(id): Path<String>,
    Json(body): Json<VerifyBody>,
) -> Result<Json<Value>, HubError> {
    require_server_key(&hub, &body.key)?;
    let db = hub.db.lock().unwrap();
    let row: Option<(i64, i64, Option<i64>)> = db
        .query_row(
            "SELECT account_id, expires_at, consumed_at FROM tickets WHERE ticket = ?1 AND server_id = ?2",
            params![body.ticket, id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .unwrap();
    let Some((account_id, expires, consumed)) = row else {
        return Err(err(StatusCode::NOT_FOUND, "not_found", "Unknown ticket"));
    };
    if consumed.is_some() || expires < now() {
        return Err(err(
            StatusCode::GONE,
            "code_expired",
            "Ticket already used or expired",
        ));
    }
    db.execute(
        "UPDATE tickets SET consumed_at = ?1 WHERE ticket = ?2",
        params![now(), body.ticket],
    )
    .ok();
    let account = load_account(&db, account_id, false).unwrap();
    Ok(Json(json!({ "account": account })))
}

#[derive(Deserialize)]
struct ReportBody {
    key: String,
    results: Vec<ReportEntry>,
}

#[derive(Deserialize)]
struct ReportEntry {
    account_id: i64,
    kills: i64,
    deaths: i64,
    won: bool,
}

async fn server_report(
    State(hub): State<Hub>,
    Path(id): Path<String>,
    Json(body): Json<ReportBody>,
) -> Result<Json<Value>, HubError> {
    require_server_key(&hub, &body.key)?;
    let db = hub.db.lock().unwrap();
    for entry in &body.results {
        db.execute(
            "INSERT INTO stats (account_id, kills, deaths, matches, wins) VALUES (?1, ?2, ?3, 1, ?4)
             ON CONFLICT(account_id) DO UPDATE SET kills = kills + ?2, deaths = deaths + ?3, matches = matches + 1, wins = wins + ?4",
            params![entry.account_id, entry.kills, entry.deaths, entry.won as i64],
        )
        .ok();
    }
    audit(
        &db,
        None,
        "match_report",
        &id,
        json!({ "players": body.results.len() }),
    );
    Ok(Json(json!({ "ok": true })))
}

fn server_rows(db: &Connection) -> Vec<Value> {
    let mut stmt = db
        .prepare("SELECT server_id, host, port, mode, map, phase, players, max_players FROM servers WHERE last_heartbeat > ?1")
        .unwrap();
    stmt.query_map(params![now() - HEARTBEAT_TTL], |r| {
        Ok(json!({
            "server_id": r.get::<_, String>(0)?, "host": r.get::<_, String>(1)?, "port": r.get::<_, i64>(2)?,
            "mode": r.get::<_, String>(3)?, "map": r.get::<_, String>(4)?, "phase": r.get::<_, String>(5)?,
            "players": r.get::<_, i64>(6)?, "max_players": r.get::<_, i64>(7)?,
        }))
    })
    .unwrap()
    .flatten()
    .collect()
}

async fn servers_list(State(hub): State<Hub>, headers: HeaderMap) -> Result<Json<Value>, HubError> {
    authenticate(&hub, &headers)?;
    let db = hub.db.lock().unwrap();
    Ok(Json(json!({ "servers": server_rows(&db) })))
}

fn issue_ticket(db: &Connection, account: i64, server: &Value) -> Value {
    let ticket = token();
    db.execute(
        "INSERT INTO tickets (ticket, account_id, server_id, expires_at) VALUES (?1, ?2, ?3, ?4)",
        params![
            ticket,
            account,
            server["server_id"].as_str().unwrap(),
            now() + TICKET_TTL
        ],
    )
    .unwrap();
    json!({ "server": server, "ticket": ticket })
}

#[derive(Deserialize)]
struct FindBody {
    mode: String,
    #[serde(default)]
    map: Option<String>,
}

fn workspace_dir() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| ".".into());
    for _ in 0..8 {
        if dir.join("assets").join("maps").is_dir() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    std::path::PathBuf::from(".")
}

async fn spawn_server(hub: &Hub, mode: &str, map: &str) -> Result<Value, HubError> {
    let Some(bin) = hub.server_bin.clone() else {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "no_servers",
            "No match server is running and the hub cannot start one",
        ));
    };
    let port = {
        let mut next = hub.ports.lock().unwrap();
        let p = *next;
        *next += 1;
        p
    };
    let server_id = format!("srv-{}-{}", port, &token()[..6]);
    let child = tokio::process::Command::new(&bin)
        .args([
            "--hub",
            &hub.hub_url,
            "--server-id",
            &server_id,
            "--port",
            &port.to_string(),
            "--mode",
            mode,
            "--map",
            map,
            "--advertise",
            &hub.server_host,
        ])
        .current_dir(workspace_dir())
        .env("STRIKE_SERVER_KEY", &hub.server_key)
        .env_remove("CARGO_MANIFEST_DIR")
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| {
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "spawn_failed",
                format!("Could not start match server: {e}"),
            )
        })?;
    hub.children.lock().unwrap().push(child);
    println!("hub: spawned {server_id} on port {port} ({mode} on {map})");
    for _ in 0..600 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let db = hub.db.lock().unwrap();
        if let Some(row) = server_rows(&db)
            .into_iter()
            .find(|s| s["server_id"] == server_id)
        {
            return Ok(row);
        }
    }
    Err(err(
        StatusCode::GATEWAY_TIMEOUT,
        "spawn_timeout",
        "Match server did not start in time",
    ))
}

async fn match_find(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Json(body): Json<FindBody>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    let map = body.map.unwrap_or_else(|| "dust2".into());
    let existing = {
        let db = hub.db.lock().unwrap();
        server_rows(&db).into_iter().find(|s| {
            s["mode"] == body.mode
                && s["players"].as_i64() < s["max_players"].as_i64()
                && s["phase"] != "finished"
        })
    };
    let server = match existing {
        Some(s) => s,
        None => spawn_server(&hub, &body.mode, &map).await?,
    };
    let db = hub.db.lock().unwrap();
    Ok(Json(issue_ticket(&db, me.id, &server)))
}

#[derive(Deserialize)]
struct JoinBody {
    server_id: String,
}

async fn match_join(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Json(body): Json<JoinBody>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    let db = hub.db.lock().unwrap();
    let server = server_rows(&db)
        .into_iter()
        .find(|s| s["server_id"] == body.server_id)
        .ok_or_else(|| {
            err(
                StatusCode::NOT_FOUND,
                "not_found",
                "That match is no longer running",
            )
        })?;
    if server["players"].as_i64() >= server["max_players"].as_i64() {
        return Err(err(StatusCode::CONFLICT, "full", "That match is full"));
    }
    Ok(Json(issue_ticket(&db, me.id, &server)))
}

// ---------------------------------------------------------------- admin

async fn admin_accounts(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    require_role(&me, &["admin", "moderator"])?;
    let db = hub.db.lock().unwrap();
    let like = format!(
        "%{}%",
        q.get("q").cloned().unwrap_or_default().to_lowercase()
    );
    let mut stmt = db
        .prepare("SELECT id, username, email, role, banned_until, ban_reason, created_at FROM accounts WHERE username_lower LIKE ?1 ORDER BY id LIMIT 50")
        .unwrap();
    let rows: Vec<Value> = stmt
        .query_map(params![like], |r| {
            Ok(json!({
                "id": r.get::<_, i64>(0)?, "username": r.get::<_, String>(1)?, "email": r.get::<_, String>(2)?,
                "role": r.get::<_, String>(3)?, "banned_until": r.get::<_, Option<i64>>(4)?,
                "ban_reason": r.get::<_, Option<String>>(5)?, "created_at": r.get::<_, i64>(6)?,
            }))
        })
        .unwrap()
        .flatten()
        .collect();
    Ok(Json(json!({ "accounts": rows })))
}

#[derive(Deserialize)]
struct RoleBody {
    role: String,
}

async fn admin_role(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(body): Json<RoleBody>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    require_role(&me, &["admin"])?;
    if !["player", "moderator", "admin"].contains(&body.role.as_str()) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid", "Unknown role"));
    }
    let db = hub.db.lock().unwrap();
    let admins: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM accounts WHERE role = 'admin'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let current: String = db
        .query_row(
            "SELECT role FROM accounts WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .unwrap_or_default();
    if current == "admin" && body.role != "admin" && admins <= 1 {
        return Err(err(
            StatusCode::CONFLICT,
            "last_admin",
            "Cannot demote the last admin",
        ));
    }
    db.execute(
        "UPDATE accounts SET role = ?1 WHERE id = ?2",
        params![body.role, id],
    )
    .unwrap();
    audit(
        &db,
        Some(me.id),
        "set_role",
        &id.to_string(),
        json!({ "role": body.role }),
    );
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct BanBody {
    #[serde(default)]
    hours: Option<i64>,
    #[serde(default)]
    reason: String,
}

async fn admin_ban(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(body): Json<BanBody>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    require_role(&me, &["admin", "moderator"])?;
    let until = now() + body.hours.unwrap_or(24 * 365 * 100) * 3600;
    let db = hub.db.lock().unwrap();
    db.execute(
        "UPDATE accounts SET banned_until = ?1, ban_reason = ?2 WHERE id = ?3",
        params![until, body.reason, id],
    )
    .unwrap();
    db.execute("DELETE FROM sessions WHERE account_id = ?1", params![id])
        .ok();
    audit(
        &db,
        Some(me.id),
        "ban",
        &id.to_string(),
        json!({ "until": until, "reason": body.reason }),
    );
    Ok(Json(json!({ "ok": true, "until": until })))
}

async fn admin_unban(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    require_role(&me, &["admin", "moderator"])?;
    let db = hub.db.lock().unwrap();
    db.execute(
        "UPDATE accounts SET banned_until = NULL, ban_reason = NULL WHERE id = ?1",
        params![id],
    )
    .unwrap();
    audit(&db, Some(me.id), "unban", &id.to_string(), json!({}));
    Ok(Json(json!({ "ok": true })))
}

async fn admin_settings_get(
    State(hub): State<Hub>,
    headers: HeaderMap,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    require_role(&me, &["admin", "moderator"])?;
    let db = hub.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT key, value FROM settings ORDER BY key")
        .unwrap();
    let map: serde_json::Map<String, Value> = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .flatten()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();
    Ok(Json(Value::Object(map)))
}

#[derive(Deserialize)]
struct SettingBody {
    key: String,
    value: String,
}

async fn admin_settings_put(
    State(hub): State<Hub>,
    headers: HeaderMap,
    Json(body): Json<SettingBody>,
) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    require_role(&me, &["admin"])?;
    let db = hub.db.lock().unwrap();
    db.execute(
        "INSERT INTO settings (key, value, updated_by, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(key) DO UPDATE SET value = ?2, updated_by = ?3, updated_at = ?4",
        params![body.key, body.value, me.id, now()],
    )
    .unwrap();
    audit(
        &db,
        Some(me.id),
        "set_setting",
        &body.key,
        json!({ "value": body.value }),
    );
    Ok(Json(json!({ "ok": true })))
}

async fn admin_audit(State(hub): State<Hub>, headers: HeaderMap) -> Result<Json<Value>, HubError> {
    let me = authenticate(&hub, &headers)?;
    require_role(&me, &["admin"])?;
    let db = hub.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT actor_id, action, target, payload, at FROM audit_log ORDER BY id DESC LIMIT 100").unwrap();
    let rows: Vec<Value> = stmt
        .query_map([], |r| {
            Ok(json!({ "actor_id": r.get::<_, Option<i64>>(0)?, "action": r.get::<_, String>(1)?, "target": r.get::<_, Option<String>>(2)?, "payload": r.get::<_, Option<String>>(3)?, "at": r.get::<_, i64>(4)? }))
        })
        .unwrap()
        .flatten()
        .collect();
    Ok(Json(json!({ "entries": rows })))
}

async fn fallback() -> Response {
    err(StatusCode::NOT_FOUND, "not_found", "No such endpoint").into_response()
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let database = std::env::var("STRIKE_HUB_DB").unwrap_or_else(|_| "hub.db".into());
    let db = Connection::open(&database).expect("open database");
    migrate(&db);
    if args.get(1).map(String::as_str) == Some("bootstrap-admin") {
        let user = args
            .get(2)
            .expect("usage: strike-hub bootstrap-admin <username>");
        let changed = db
            .execute(
                "UPDATE accounts SET role = 'admin' WHERE username_lower = ?1",
                params![user.to_lowercase()],
            )
            .unwrap();
        println!(
            "{}",
            if changed == 1 {
                "promoted"
            } else {
                "no such account"
            }
        );
        return;
    }
    let listen = std::env::var("STRIKE_HUB_LISTEN").unwrap_or_else(|_| "0.0.0.0:7777".into());
    let hub_url = std::env::var("STRIKE_HUB_URL")
        .unwrap_or_else(|_| format!("http://127.0.0.1:{}", listen.rsplit(':').next().unwrap()));
    let hub = Hub {
        db: Arc::new(Mutex::new(db)),
        name: std::env::var("STRIKE_HUB_NAME").unwrap_or_else(|_| "Open Strike Hub".into()),
        server_key: std::env::var("STRIKE_SERVER_KEY").unwrap_or_else(|_| {
            let key = token();
            println!(
                "hub: generated STRIKE_SERVER_KEY={key} (set it to run match servers separately)"
            );
            key
        }),
        server_bin: std::env::var("STRIKE_SERVER_BIN").ok(),
        server_host: std::env::var("STRIKE_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
        hub_url,
        ports: Arc::new(Mutex::new(
            std::env::var("STRIKE_SERVER_PORT_START")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(27015),
        )),
        children: Arc::new(Mutex::new(Vec::new())),
    };
    let app = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/me", get(me))
        .route("/v1/auth/logout", post(logout))
        .route("/v1/friends", get(friends_list))
        .route("/v1/friends/search", get(friends_search))
        .route("/v1/friends/request", post(friends_request))
        .route("/v1/friends/accept", post(friends_accept))
        .route("/v1/friends/decline", post(friends_decline))
        .route("/v1/friends/{username}", delete(friends_remove))
        .route("/v1/presence", post(presence))
        .route("/v1/servers", get(servers_list))
        .route("/v1/servers/register", post(server_register))
        .route("/v1/servers/{id}/heartbeat", post(server_heartbeat))
        .route("/v1/servers/{id}/verify_ticket", post(server_verify_ticket))
        .route("/v1/servers/{id}/report", post(server_report))
        .route("/v1/match/find", post(match_find))
        .route("/v1/match/join", post(match_join))
        .route("/v1/admin/accounts", get(admin_accounts))
        .route("/v1/admin/accounts/{id}/role", post(admin_role))
        .route("/v1/admin/accounts/{id}/ban", post(admin_ban))
        .route("/v1/admin/accounts/{id}/unban", post(admin_unban))
        .route(
            "/v1/admin/settings",
            get(admin_settings_get).put(admin_settings_put),
        )
        .route("/v1/admin/audit", get(admin_audit))
        .fallback(fallback)
        .with_state(hub.clone());
    println!(
        "hub: {} listening on {listen}, database {database}",
        hub.name
    );
    let listener = tokio::net::TcpListener::bind(&listen).await.expect("bind");
    axum::serve(listener, app).await.unwrap();
}
