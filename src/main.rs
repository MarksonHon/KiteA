// ============================================================
//  KiteA – v2rayA-like Web frontend for Shoes proxy
//  Single-file Rust backend
// ============================================================

use actix_cors::Cors;
use actix_web::{
    delete, get, middleware, post, put,
    web::{self, Data, Json, Path as WebPath},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use log::{info, warn};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::{
    env,
    process::{Child, Command},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

// ─── embedded frontend ────────────────────────────────────────────────────────
const INDEX_HTML: &str = include_str!("../static/index.html");

// ─── global JWT secret ────────────────────────────────────────────────────────
static JWT_SECRET: OnceCell<String> = OnceCell::new();

fn jwt_secret() -> &'static str {
    JWT_SECRET.get().expect("JWT_SECRET not initialised")
}

// ─── database models ──────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct User {
    id:            String,
    username:      String,
    #[serde(skip_serializing)]
    password_hash: String,
    role:          String, // "admin" | "user"
    created_at:    String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct Node {
    id:          String,
    name:        String,
    protocol:    String, // vmess | vless | ss | trojan | socks5 | http | …
    server:      String,
    port:        i64,
    config_yaml: String,  // raw shoes listener/upstream YAML snippet
    enabled:     bool,
    created_at:  String,
    updated_at:  String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct Setting {
    key:   String,
    value: String,
}

// ─── request / response DTOs ──────────────────────────────────────────────────
#[derive(Deserialize)]
struct RegisterReq {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginReq {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResp {
    token:    String,
    username: String,
    role:     String,
}

#[derive(Deserialize)]
struct CreateNodeReq {
    name:        String,
    protocol:    String,
    server:      String,
    port:        i64,
    config_yaml: String,
}

#[derive(Deserialize)]
struct UpdateNodeReq {
    name:        Option<String>,
    protocol:    Option<String>,
    server:      Option<String>,
    port:        Option<i64>,
    config_yaml: Option<String>,
    enabled:     Option<bool>,
}

#[derive(Deserialize)]
struct ChangePwdReq {
    old_password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct SaveSettingReq {
    key:   String,
    value: String,
}

#[derive(Serialize)]
struct ApiResp<T: Serialize> {
    ok:   bool,
    data: Option<T>,
    msg:  Option<String>,
}

impl<T: Serialize> ApiResp<T> {
    fn ok(data: T) -> HttpResponse {
        HttpResponse::Ok().json(ApiResp { ok: true, data: Some(data), msg: None })
    }
    fn err(msg: impl Into<String>) -> HttpResponse {
        HttpResponse::BadRequest().json(ApiResp::<()> {
            ok:   false,
            data: None,
            msg:  Some(msg.into()),
        })
    }
    fn unauthenticated() -> HttpResponse {
        HttpResponse::Unauthorized().json(ApiResp::<()> {
            ok:   false,
            data: None,
            msg:  Some("Unauthorised".into()),
        })
    }
}

// ─── JWT ─────────────────────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    sub:  String, // user id
    name: String, // username
    role: String,
    exp:  i64,
}

fn make_token(user: &User) -> Result<String> {
    let exp = (Utc::now() + Duration::hours(24)).timestamp();
    let claims = Claims {
        sub:  user.id.clone(),
        name: user.username.clone(),
        role: user.role.clone(),
        exp,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )?;
    Ok(token)
}

fn verify_token(token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    )
    .map(|t: TokenData<Claims>| t.claims)
    .ok()
}

fn extract_claims(req: &HttpRequest) -> Option<Claims> {
    let bearer = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))?;
    verify_token(bearer.trim())
}

// ─── shoes process manager ────────────────────────────────────────────────────
struct ShoesProcess {
    child:       Option<Child>,
    config_path: String,
    binary_path: String,
}

impl ShoesProcess {
    fn new(binary_path: String, config_path: String) -> Self {
        Self { child: None, config_path, binary_path }
    }

    fn is_running(&mut self) -> bool {
        if let Some(c) = self.child.as_mut() {
            c.try_wait().map(|s| s.is_none()).unwrap_or(false)
        } else {
            false
        }
    }

    fn start(&mut self, yaml: &str) -> Result<()> {
        if self.is_running() {
            return Err(anyhow::anyhow!("Shoes is already running"));
        }
        std::fs::write(&self.config_path, yaml)?;
        let child = Command::new(&self.binary_path)
            .arg(&self.config_path)
            .spawn()?;
        self.child = Some(child);
        info!("Shoes process started (pid={})", self.child.as_ref().unwrap().id());
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut c) = self.child.take() {
            c.kill().ok();
            c.wait().ok();
            info!("Shoes process stopped.");
        }
        Ok(())
    }
}

type SharedProcess = Arc<Mutex<ShoesProcess>>;

// ─── config YAML builder ──────────────────────────────────────────────────────
/// Combine all enabled nodes into a shoes-compatible config YAML.
/// Adds a local SOCKS5 mixed listener on port 1080.
fn build_shoes_config(nodes: &[Node], local_port: u16) -> String {
    let mut snippets: Vec<String> = vec![format!(
        "# auto-generated by KiteA\n\
         - address: 127.0.0.1:{local_port}\n\
         \x20\x20protocol:\n\
         \x20\x20\x20\x20type: mixed\n"
    )];

    for node in nodes.iter().filter(|n| n.enabled) {
        snippets.push(format!(
            "# node: {} ({})\n{}",
            node.name, node.protocol, node.config_yaml
        ));
    }

    snippets.join("\n---\n")
}

// ─── DB helpers ────────────────────────────────────────────────────────────────
async fn db_init(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS users (
               id            TEXT PRIMARY KEY,
               username      TEXT UNIQUE NOT NULL,
               password_hash TEXT NOT NULL,
               role          TEXT NOT NULL DEFAULT 'user',
               created_at    TEXT NOT NULL
           );
           CREATE TABLE IF NOT EXISTS nodes (
               id          TEXT PRIMARY KEY,
               name        TEXT NOT NULL,
               protocol    TEXT NOT NULL,
               server      TEXT NOT NULL,
               port        INTEGER NOT NULL,
               config_yaml TEXT NOT NULL DEFAULT '',
               enabled     INTEGER NOT NULL DEFAULT 1,
               created_at  TEXT NOT NULL,
               updated_at  TEXT NOT NULL
           );
           CREATE TABLE IF NOT EXISTS settings (
               key   TEXT PRIMARY KEY,
               value TEXT NOT NULL
           );
           INSERT OR IGNORE INTO settings (key, value)
               VALUES ('shoes_binary', 'shoes'),
                      ('local_port',   '1080'),
                      ('log_level',    'info');"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn db_get_setting(pool: &SqlitePool, key: &str) -> String {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}

// ─── API handlers ─────────────────────────────────────────────────────────────

// --- frontend ---------------------------------------------------------------
async fn frontend() -> impl Responder {
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(INDEX_HTML)
}

// --- auth -------------------------------------------------------------------
#[post("/api/auth/register")]
async fn register(
    pool: Data<SqlitePool>,
    body: Json<RegisterReq>,
) -> impl Responder {
    let username = body.username.trim();
    if username.is_empty() || body.password.len() < 6 {
        return ApiResp::<()>::err("Username cannot be empty, password must be ≥ 6 chars");
    }
    // Only allow registration if no users exist (first-run), or by an admin
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool.as_ref())
        .await
        .unwrap_or(0);
    if count > 0 {
        return ApiResp::<()>::err("Registration is only available on first run. Ask an admin.");
    }
    let pw_hash = match hash(&body.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => return ApiResp::<()>::err(e.to_string()),
    };
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?,?,?,?,?)",
    )
    .bind(&id)
    .bind(username)
    .bind(&pw_hash)
    .bind("admin")
    .bind(&now)
    .execute(pool.as_ref())
    .await
    {
        return ApiResp::<()>::err(e.to_string());
    }
    ApiResp::ok(serde_json::json!({ "message": "Admin account created." }))
}

#[post("/api/auth/login")]
async fn login(
    pool: Data<SqlitePool>,
    body: Json<LoginReq>,
) -> impl Responder {
    let user: Option<User> =
        sqlx::query_as("SELECT * FROM users WHERE username = ?")
            .bind(&body.username)
            .fetch_optional(pool.as_ref())
            .await
            .unwrap_or(None);
    let user = match user {
        Some(u) => u,
        None => return ApiResp::<()>::err("Invalid credentials"),
    };
    match verify(&body.password, &user.password_hash) {
        Ok(true) => {}
        _ => return ApiResp::<()>::err("Invalid credentials"),
    }
    match make_token(&user) {
        Ok(token) => ApiResp::ok(LoginResp {
            token,
            username: user.username.clone(),
            role: user.role.clone(),
        }),
        Err(e) => ApiResp::<()>::err(e.to_string()),
    }
}

#[post("/api/auth/change-password")]
async fn change_password(
    req: HttpRequest,
    pool: Data<SqlitePool>,
    body: Json<ChangePwdReq>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Some(c) => c,
        None => return ApiResp::<()>::unauthenticated(),
    };
    let user: Option<User> =
        sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(&claims.sub)
            .fetch_optional(pool.as_ref())
            .await
            .unwrap_or(None);
    let user = match user {
        Some(u) => u,
        None => return ApiResp::<()>::unauthenticated(),
    };
    match verify(&body.old_password, &user.password_hash) {
        Ok(true) => {}
        _ => return ApiResp::<()>::err("Old password is incorrect"),
    }
    if body.new_password.len() < 6 {
        return ApiResp::<()>::err("New password must be ≥ 6 chars");
    }
    let new_hash = match hash(&body.new_password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => return ApiResp::<()>::err(e.to_string()),
    };
    let _ = sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(&new_hash)
        .bind(&claims.sub)
        .execute(pool.as_ref())
        .await;
    ApiResp::ok(serde_json::json!({ "message": "Password changed." }))
}

// --- admin: user management -------------------------------------------------
#[get("/api/users")]
async fn list_users(req: HttpRequest, pool: Data<SqlitePool>) -> impl Responder {
    let claims = match extract_claims(&req) {
        Some(c) if c.role == "admin" => c,
        Some(_) => return ApiResp::<()>::err("Admin only"),
        None => return ApiResp::<()>::unauthenticated(),
    };
    let _ = claims;
    let users: Vec<User> = sqlx::query_as("SELECT * FROM users ORDER BY created_at ASC")
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default();
    ApiResp::ok(users)
}

#[post("/api/users")]
async fn create_user(
    req: HttpRequest,
    pool: Data<SqlitePool>,
    body: Json<RegisterReq>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Some(c) if c.role == "admin" => c,
        Some(_) => return ApiResp::<()>::err("Admin only"),
        None => return ApiResp::<()>::unauthenticated(),
    };
    let _ = claims;
    let pw_hash = match hash(&body.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => return ApiResp::<()>::err(e.to_string()),
    };
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&body.username)
    .bind(&pw_hash)
    .bind("user")
    .bind(&now)
    .execute(pool.as_ref())
    .await
    {
        return ApiResp::<()>::err(e.to_string());
    }
    ApiResp::ok(serde_json::json!({ "id": id }))
}

#[delete("/api/users/{id}")]
async fn delete_user(
    req: HttpRequest,
    pool: Data<SqlitePool>,
    path: WebPath<String>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Some(c) if c.role == "admin" => c,
        Some(_) => return ApiResp::<()>::err("Admin only"),
        None => return ApiResp::<()>::unauthenticated(),
    };
    let target_id = path.into_inner();
    if target_id == claims.sub {
        return ApiResp::<()>::err("Cannot delete yourself");
    }
    let _ = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&target_id)
        .execute(pool.as_ref())
        .await;
    ApiResp::ok(serde_json::json!({ "message": "Deleted" }))
}

// --- nodes ------------------------------------------------------------------
#[get("/api/nodes")]
async fn list_nodes(req: HttpRequest, pool: Data<SqlitePool>) -> impl Responder {
    if extract_claims(&req).is_none() {
        return ApiResp::<()>::unauthenticated();
    }
    let nodes: Vec<Node> = sqlx::query_as("SELECT * FROM nodes ORDER BY created_at ASC")
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default();
    ApiResp::ok(nodes)
}

#[post("/api/nodes")]
async fn create_node(
    req: HttpRequest,
    pool: Data<SqlitePool>,
    body: Json<CreateNodeReq>,
) -> impl Responder {
    if extract_claims(&req).is_none() {
        return ApiResp::<()>::unauthenticated();
    }
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query(
        "INSERT INTO nodes (id,name,protocol,server,port,config_yaml,enabled,created_at,updated_at)
         VALUES (?,?,?,?,?,?,1,?,?)",
    )
    .bind(&id)
    .bind(&body.name)
    .bind(&body.protocol)
    .bind(&body.server)
    .bind(body.port)
    .bind(&body.config_yaml)
    .bind(&now)
    .bind(&now)
    .execute(pool.as_ref())
    .await
    {
        return ApiResp::<()>::err(e.to_string());
    }
    ApiResp::ok(serde_json::json!({ "id": id }))
}

#[put("/api/nodes/{id}")]
async fn update_node(
    req: HttpRequest,
    pool: Data<SqlitePool>,
    path: WebPath<String>,
    body: Json<UpdateNodeReq>,
) -> impl Responder {
    if extract_claims(&req).is_none() {
        return ApiResp::<()>::unauthenticated();
    }
    let id = path.into_inner();
    let now = Utc::now().to_rfc3339();
    let node: Option<Node> = sqlx::query_as("SELECT * FROM nodes WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.as_ref())
        .await
        .unwrap_or(None);
    let mut node = match node {
        Some(n) => n,
        None => return ApiResp::<()>::err("Node not found"),
    };
    if let Some(v) = &body.name        { node.name        = v.clone(); }
    if let Some(v) = &body.protocol    { node.protocol    = v.clone(); }
    if let Some(v) = &body.server      { node.server      = v.clone(); }
    if let Some(v) = body.port         { node.port        = v; }
    if let Some(v) = &body.config_yaml { node.config_yaml = v.clone(); }
    if let Some(v) = body.enabled      { node.enabled     = v; }
    node.updated_at = now;
    if let Err(e) = sqlx::query(
        "UPDATE nodes SET name=?,protocol=?,server=?,port=?,config_yaml=?,enabled=?,updated_at=?
         WHERE id=?",
    )
    .bind(&node.name)
    .bind(&node.protocol)
    .bind(&node.server)
    .bind(node.port)
    .bind(&node.config_yaml)
    .bind(node.enabled)
    .bind(&node.updated_at)
    .bind(&id)
    .execute(pool.as_ref())
    .await
    {
        return ApiResp::<()>::err(e.to_string());
    }
    ApiResp::ok(node)
}

#[delete("/api/nodes/{id}")]
async fn delete_node(
    req: HttpRequest,
    pool: Data<SqlitePool>,
    path: WebPath<String>,
) -> impl Responder {
    if extract_claims(&req).is_none() {
        return ApiResp::<()>::unauthenticated();
    }
    let id = path.into_inner();
    let _ = sqlx::query("DELETE FROM nodes WHERE id = ?")
        .bind(&id)
        .execute(pool.as_ref())
        .await;
    ApiResp::ok(serde_json::json!({ "message": "Deleted" }))
}

// --- settings ---------------------------------------------------------------
#[get("/api/settings")]
async fn get_settings(req: HttpRequest, pool: Data<SqlitePool>) -> impl Responder {
    let claims = match extract_claims(&req) {
        Some(c) if c.role == "admin" => c,
        Some(_) => return ApiResp::<()>::err("Admin only"),
        None => return ApiResp::<()>::unauthenticated(),
    };
    let _ = claims;
    let rows: Vec<Setting> = sqlx::query_as("SELECT key, value FROM settings ORDER BY key")
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default();
    let map: serde_json::Map<String, serde_json::Value> =
        rows.into_iter().map(|r| (r.key, r.value.into())).collect();
    ApiResp::ok(serde_json::Value::Object(map))
}

#[post("/api/settings")]
async fn save_setting(
    req: HttpRequest,
    pool: Data<SqlitePool>,
    body: Json<SaveSettingReq>,
) -> impl Responder {
    let claims = match extract_claims(&req) {
        Some(c) if c.role == "admin" => c,
        Some(_) => return ApiResp::<()>::err("Admin only"),
        None => return ApiResp::<()>::unauthenticated(),
    };
    let _ = claims;
    let allowed = ["shoes_binary", "local_port", "log_level"];
    if !allowed.contains(&body.key.as_str()) {
        return ApiResp::<()>::err("Unknown setting key");
    }
    let _ = sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(&body.key)
        .bind(&body.value)
        .execute(pool.as_ref())
        .await;
    ApiResp::ok(serde_json::json!({ "message": "Saved" }))
}

// --- proxy control ----------------------------------------------------------
#[derive(Serialize)]
struct StatusResp {
    running:     bool,
    pid:         Option<u32>,
    local_port:  u16,
    node_count:  usize,
}

#[get("/api/proxy/status")]
async fn proxy_status(
    req: HttpRequest,
    pool: Data<SqlitePool>,
    process: Data<SharedProcess>,
) -> impl Responder {
    if extract_claims(&req).is_none() {
        return ApiResp::<()>::unauthenticated();
    }
    let mut p = process.lock().unwrap();
    let running = p.is_running();
    let pid = if running { p.child.as_ref().map(|c| c.id()) } else { None };
    let nodes: Vec<Node> = sqlx::query_as("SELECT * FROM nodes WHERE enabled = 1")
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default();
    let local_port: u16 = db_get_setting(pool.as_ref(), "local_port")
        .await
        .parse()
        .unwrap_or(1080);
    ApiResp::ok(StatusResp {
        running,
        pid,
        local_port,
        node_count: nodes.len(),
    })
}

#[post("/api/proxy/start")]
async fn proxy_start(
    req: HttpRequest,
    pool: Data<SqlitePool>,
    process: Data<SharedProcess>,
) -> impl Responder {
    if extract_claims(&req).is_none() {
        return ApiResp::<()>::unauthenticated();
    }
    let nodes: Vec<Node> = sqlx::query_as("SELECT * FROM nodes WHERE enabled = 1")
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default();
    if nodes.is_empty() {
        return ApiResp::<()>::err("No enabled nodes found. Please add at least one node.");
    }
    let local_port: u16 = db_get_setting(pool.as_ref(), "local_port")
        .await
        .parse()
        .unwrap_or(1080);
    let yaml = build_shoes_config(&nodes, local_port);
    // Fetch binary path from DB before locking the process mutex
    let db_binary = db_get_setting(pool.as_ref(), "shoes_binary").await;
    let mut p = process.lock().unwrap();
    if !db_binary.is_empty() {
        p.binary_path = db_binary;
    }
    let binary = p.binary_path.clone();
    match p.start(&yaml) {
        Ok(_) => ApiResp::ok(serde_json::json!({
            "message": format!("Shoes started on 127.0.0.1:{}", local_port),
            "binary": binary,
        })),
        Err(e) => ApiResp::<()>::err(format!("Failed to start shoes: {e}")),
    }
}

#[post("/api/proxy/stop")]
async fn proxy_stop(
    req: HttpRequest,
    process: Data<SharedProcess>,
) -> impl Responder {
    if extract_claims(&req).is_none() {
        return ApiResp::<()>::unauthenticated();
    }
    let mut p = process.lock().unwrap();
    match p.stop() {
        Ok(_) => ApiResp::ok(serde_json::json!({ "message": "Shoes stopped." })),
        Err(e) => ApiResp::<()>::err(e.to_string()),
    }
}

#[get("/api/proxy/config")]
async fn proxy_config(
    req: HttpRequest,
    pool: Data<SqlitePool>,
) -> impl Responder {
    if extract_claims(&req).is_none() {
        return ApiResp::<()>::unauthenticated();
    }
    let nodes: Vec<Node> = sqlx::query_as("SELECT * FROM nodes WHERE enabled = 1")
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default();
    let local_port: u16 = db_get_setting(pool.as_ref(), "local_port")
        .await
        .parse()
        .unwrap_or(1080);
    let yaml = build_shoes_config(&nodes, local_port);
    ApiResp::ok(serde_json::json!({ "yaml": yaml }))
}

// ─── first-run check ─────────────────────────────────────────────────────────
#[get("/api/setup/needed")]
async fn setup_needed(pool: Data<SqlitePool>) -> impl Responder {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool.as_ref())
        .await
        .unwrap_or(0);
    ApiResp::ok(serde_json::json!({ "needed": count == 0 }))
}

// ─── main ────────────────────────────────────────────────────────────────────
#[tokio::main]
async fn main() -> Result<()> {
    // Init logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // JWT secret: env var or random
    let secret = env::var("KITEA_JWT_SECRET").unwrap_or_else(|_| {
        let s = Uuid::new_v4().to_string();
        warn!("KITEA_JWT_SECRET not set, using random secret (sessions won't persist across restarts)");
        s
    });
    JWT_SECRET.set(secret).expect("JWT_SECRET already set");

    // Database
    let db_url = env::var("KITEA_DB_URL").unwrap_or_else(|_| "sqlite://kitea.db".to_string());
    let pool = SqlitePool::connect(&db_url).await?;
    db_init(&pool).await?;
    info!("Database ready at {db_url}");

    // Shoes process manager
    let shoes_binary = if let Ok(v) = env::var("KITEA_SHOES_BINARY") {
        v
    } else {
        let v = db_get_setting(&pool, "shoes_binary").await;
        if v.is_empty() { "shoes".to_string() } else { v }
    };
    let config_path = env::var("KITEA_SHOES_CONFIG")
        .unwrap_or_else(|_| "shoes_runtime.yaml".to_string());
    let process: SharedProcess = Arc::new(Mutex::new(ShoesProcess::new(
        shoes_binary.clone(),
        config_path,
    )));
    info!("Shoes binary: {shoes_binary}");

    // Listen address
    let listen = env::var("KITEA_LISTEN").unwrap_or_else(|_| "0.0.0.0:7890".to_string());
    info!("KiteA listening on http://{listen}");

    let pool_data = Data::new(pool);
    let process_data = Data::new(process);

    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .app_data(process_data.clone())
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            // Auth
            .service(register)
            .service(login)
            .service(change_password)
            // Users
            .service(list_users)
            .service(create_user)
            .service(delete_user)
            // Nodes
            .service(list_nodes)
            .service(create_node)
            .service(update_node)
            .service(delete_node)
            // Settings
            .service(get_settings)
            .service(save_setting)
            // Proxy
            .service(proxy_status)
            .service(proxy_start)
            .service(proxy_stop)
            .service(proxy_config)
            // Setup
            .service(setup_needed)
            // Frontend – catch-all
            .route("/", web::get().to(frontend))
            .route("/{_:.*}", web::get().to(frontend))
    })
    .bind(&listen)?
    .run()
    .await?;

    Ok(())
}
