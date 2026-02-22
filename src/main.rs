// ============================================================
//  KiteA – v2rayA-like Web frontend for Shoes proxy
//  Single-file Rust backend  v0.2.0
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
use mime_guess::from_path;
use once_cell::sync::OnceCell;
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::{
    borrow::Cow,
    env,
    path::PathBuf,
    process::{Child, Command},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

// ─── embedded frontend (built from frontend/) ─────────────────────────────────
#[derive(Embed)]
#[folder = "frontend/dist/"]
struct FrontendAssets;

fn serve_embedded(path: &str) -> HttpResponse {
    let asset_path = if path.is_empty() || path == "/" { "index.html" } else { path };
    match FrontendAssets::get(asset_path) {
        Some(content) => {
            let mime = from_path(asset_path).first_or_octet_stream();
            HttpResponse::Ok()
                .content_type(mime.as_ref())
                .body(match content.data {
                    Cow::Borrowed(b) => actix_web::web::Bytes::from_static(b),
                    Cow::Owned(b) => actix_web::web::Bytes::from(b),
                })
        }
        None => {
            // SPA fallback – serve index.html for unknown paths
            match FrontendAssets::get("index.html") {
                Some(content) => HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body(match content.data {
                        Cow::Borrowed(b) => actix_web::web::Bytes::from_static(b),
                        Cow::Owned(b) => actix_web::web::Bytes::from(b),
                    }),
                None => HttpResponse::NotFound().body("Frontend not built. Run: cd frontend && npm install && npm run build"),
            }
        }
    }
}

async fn frontend_index() -> impl Responder { serve_embedded("index.html") }
async fn frontend_assets(path: WebPath<String>) -> impl Responder { serve_embedded(&path.into_inner()) }

// ─── global JWT secret ────────────────────────────────────────────────────────
static JWT_SECRET: OnceCell<String> = OnceCell::new();
fn jwt_secret() -> &'static str { JWT_SECRET.get().expect("JWT_SECRET not initialised") }

// ─── database path ────────────────────────────────────────────────────────────
fn default_db_path() -> PathBuf {
    let base = dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("KiteA")
}

fn ensure_db_url() -> String {
    if let Ok(v) = env::var("KITEA_DB_URL") {
        return v;
    }
    let dir = default_db_path();
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| warn!("Cannot create config dir {:?}: {e}", dir));
    let db_file = dir.join("kitea.db");
    format!("sqlite://{}?mode=rwc", db_file.to_string_lossy().replace('\\', "/"))
}

fn shoes_config_path() -> PathBuf {
    env::var("KITEA_SHOES_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_db_path().join("shoes_runtime.yaml"))
}

// ─── database models ──────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct User {
    id:            String,
    username:      String,
    #[serde(skip_serializing)]
    password_hash: String,
    role:          String,
    created_at:    String,
}

/// A proxy node stored as its original URI (vmess:// vless:// ss:// trojan:// hy2:// tuic://)
/// plus human-readable metadata extracted at import time.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct Node {
    id:         String,
    name:       String,       // display name (ps / remark field)
    protocol:   String,       // vmess | vless | shadowsocks | trojan | hysteria2 | tuic | socks5 | http
    server:     String,
    port:       i64,
    uri:        String,       // original share URI (vmess://... vless://... etc.)
    group:      String,       // subscription group name, empty for manual
    enabled:    bool,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct Subscription {
    id:         String,
    name:       String,
    url:        String,
    last_update: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct Setting {
    key:   String,
    value: String,
}

// ─── request / response DTOs ──────────────────────────────────────────────────
#[derive(Deserialize)]
struct RegisterReq { username: String, password: String }

#[derive(Deserialize)]
struct LoginReq { username: String, password: String }

#[derive(Serialize)]
struct LoginResp { token: String, username: String, role: String }

/// Node created from a URI string (v2rayN / v2rayA share format)
#[derive(Deserialize)]
struct ImportNodeReq {
    uri:   String,       // vmess://... or vless://... etc.
    group: Option<String>,
}

/// Batch import from a subscription text (one URI per line, possibly base64)
#[derive(Deserialize)]
struct ImportSubscriptionReq {
    name: String,
    urls: Vec<String>,   // list of share URIs (already decoded by frontend)
    group: String,
}

#[derive(Deserialize)]
struct UpdateNodeReq {
    name:    Option<String>,
    enabled: Option<bool>,
    uri:     Option<String>,
}

#[derive(Deserialize)]
struct ChangePwdReq { old_password: String, new_password: String }

#[derive(Deserialize)]
struct SaveSettingReq { key: String, value: String }

/// The frontend generates the Shoes YAML and sends it here.
#[derive(Deserialize)]
struct StartProxyReq {
    yaml: String,
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
        HttpResponse::BadRequest().json(ApiResp::<()> { ok: false, data: None, msg: Some(msg.into()) })
    }
    fn unauth() -> HttpResponse {
        HttpResponse::Unauthorized().json(ApiResp::<()> { ok: false, data: None, msg: Some("Unauthorised".into()) })
    }
}

// ─── JWT ──────────────────────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims { sub: String, name: String, role: String, exp: i64 }

fn make_token(user: &User) -> Result<String> {
    let claims = Claims {
        sub: user.id.clone(), name: user.username.clone(),
        role: user.role.clone(),
        exp: (Utc::now() + Duration::hours(24)).timestamp(),
    };
    Ok(encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret().as_bytes()))?)
}

fn claims_from_req(req: &HttpRequest) -> Option<Claims> {
    let bearer = req.headers().get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))?;
    decode::<Claims>(bearer.trim(), &DecodingKey::from_secret(jwt_secret().as_bytes()), &Validation::default())
        .map(|t: TokenData<Claims>| t.claims).ok()
}

macro_rules! auth {
    ($req:expr) => {
        match claims_from_req(&$req) { Some(c) => c, None => return ApiResp::<()>::unauth() }
    };
    ($req:expr, admin) => {
        match claims_from_req(&$req) {
            Some(c) if c.role == "admin" => c,
            Some(_) => return ApiResp::<()>::err("Admin only"),
            None => return ApiResp::<()>::unauth(),
        }
    };
}

// ─── shoes process manager ────────────────────────────────────────────────────
struct ShoesProcess { child: Option<Child>, config_path: PathBuf, binary_path: String }

impl ShoesProcess {
    fn new(binary_path: String, config_path: PathBuf) -> Self {
        Self { child: None, config_path, binary_path }
    }
    fn is_running(&mut self) -> bool {
        self.child.as_mut().map(|c| c.try_wait().map(|s| s.is_none()).unwrap_or(false)).unwrap_or(false)
    }
    fn start(&mut self, yaml: &str) -> Result<()> {
        if self.is_running() { return Err(anyhow::anyhow!("Shoes is already running")); }
        if let Some(parent) = self.config_path.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(&self.config_path, yaml)?;
        let child = Command::new(&self.binary_path).arg(&self.config_path).spawn()?;
        info!("Shoes started (pid={})", child.id());
        self.child = Some(child);
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        if let Some(mut c) = self.child.take() { c.kill().ok(); c.wait().ok(); info!("Shoes stopped"); }
        Ok(())
    }
}

type SharedProcess = Arc<Mutex<ShoesProcess>>;

// ─── DB init ────────────────────────────────────────────────────────────────────
async fn db_init(pool: &SqlitePool) -> Result<()> {
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY, username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'user',
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY, name TEXT NOT NULL,
            protocol TEXT NOT NULL, server TEXT NOT NULL, port INTEGER NOT NULL,
            uri TEXT NOT NULL DEFAULT '', group_name TEXT NOT NULL DEFAULT '',
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS subscriptions (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, url TEXT NOT NULL,
            last_update TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY, value TEXT NOT NULL
        );
        INSERT OR IGNORE INTO settings (key, value) VALUES
            ('shoes_binary', 'shoes'),
            ('local_port',   '1080'),
            ('log_level',    'info');
    "#).execute(pool).await?;
    Ok(())
}

async fn db_get_setting(pool: &SqlitePool, key: &str) -> String {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key).fetch_optional(pool).await.ok().flatten().unwrap_or_default()
}

// ─── API: auth ────────────────────────────────────────────────────────────────
#[post("/api/auth/register")]
async fn register(pool: Data<SqlitePool>, body: Json<RegisterReq>) -> impl Responder {
    let username = body.username.trim();
    if username.is_empty() || body.password.len() < 6 {
        return ApiResp::<()>::err("Username empty or password < 6 chars");
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool.as_ref()).await.unwrap_or(0);
    if count > 0 { return ApiResp::<()>::err("Registration disabled. Ask an admin."); }
    let hash = match hash(&body.password, DEFAULT_COST) { Ok(h) => h, Err(e) => return ApiResp::<()>::err(e.to_string()) };
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query("INSERT INTO users (id,username,password_hash,role,created_at) VALUES (?,?,?,?,?)")
        .bind(&id).bind(username).bind(&hash).bind("admin").bind(&now).execute(pool.as_ref()).await
    { return ApiResp::<()>::err(e.to_string()); }
    ApiResp::ok(serde_json::json!({ "message": "Admin account created." }))
}

#[post("/api/auth/login")]
async fn login(pool: Data<SqlitePool>, body: Json<LoginReq>) -> impl Responder {
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE username = ?")
        .bind(&body.username).fetch_optional(pool.as_ref()).await.unwrap_or(None);
    let user = match user { Some(u) => u, None => return ApiResp::<()>::err("Invalid credentials") };
    if verify(&body.password, &user.password_hash).unwrap_or(false) {
        match make_token(&user) {
            Ok(t) => ApiResp::ok(LoginResp { token: t, username: user.username, role: user.role }),
            Err(e) => ApiResp::<()>::err(e.to_string()),
        }
    } else {
        ApiResp::<()>::err("Invalid credentials")
    }
}

#[post("/api/auth/change-password")]
async fn change_password(req: HttpRequest, pool: Data<SqlitePool>, body: Json<ChangePwdReq>) -> impl Responder {
    let claims = auth!(req);
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&claims.sub).fetch_optional(pool.as_ref()).await.unwrap_or(None);
    let user = match user { Some(u) => u, None => return ApiResp::<()>::unauth() };
    if !verify(&body.old_password, &user.password_hash).unwrap_or(false) {
        return ApiResp::<()>::err("Old password incorrect");
    }
    if body.new_password.len() < 6 { return ApiResp::<()>::err("New password < 6 chars"); }
    let h = match hash(&body.new_password, DEFAULT_COST) { Ok(h) => h, Err(e) => return ApiResp::<()>::err(e.to_string()) };
    let _ = sqlx::query("UPDATE users SET password_hash=? WHERE id=?")
        .bind(&h).bind(&claims.sub).execute(pool.as_ref()).await;
    ApiResp::ok(serde_json::json!({ "message": "Password changed." }))
}

// ─── API: users ───────────────────────────────────────────────────────────────
#[get("/api/users")]
async fn list_users(req: HttpRequest, pool: Data<SqlitePool>) -> impl Responder {
    let _c = auth!(req, admin);
    let users: Vec<User> = sqlx::query_as("SELECT * FROM users ORDER BY created_at").fetch_all(pool.as_ref()).await.unwrap_or_default();
    ApiResp::ok(users)
}

#[post("/api/users")]
async fn create_user(req: HttpRequest, pool: Data<SqlitePool>, body: Json<RegisterReq>) -> impl Responder {
    let _c = auth!(req, admin);
    let h = match hash(&body.password, DEFAULT_COST) { Ok(h) => h, Err(e) => return ApiResp::<()>::err(e.to_string()) };
    let id = Uuid::new_v4().to_string(); let now = Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query("INSERT INTO users (id,username,password_hash,role,created_at) VALUES (?,?,?,?,?)")
        .bind(&id).bind(&body.username).bind(&h).bind("user").bind(&now).execute(pool.as_ref()).await
    { return ApiResp::<()>::err(e.to_string()); }
    ApiResp::ok(serde_json::json!({ "id": id }))
}

#[delete("/api/users/{id}")]
async fn delete_user(req: HttpRequest, pool: Data<SqlitePool>, path: WebPath<String>) -> impl Responder {
    let c = auth!(req, admin);
    let target = path.into_inner();
    if target == c.sub { return ApiResp::<()>::err("Cannot delete yourself"); }
    let _ = sqlx::query("DELETE FROM users WHERE id=?").bind(&target).execute(pool.as_ref()).await;
    ApiResp::ok(serde_json::json!({ "message": "Deleted" }))
}

// ─── API: nodes ───────────────────────────────────────────────────────────────
#[get("/api/nodes")]
async fn list_nodes(req: HttpRequest, pool: Data<SqlitePool>) -> impl Responder {
    auth!(req);
    let nodes: Vec<Node> = sqlx::query_as(
        "SELECT id,name,protocol,server,port,uri,group_name as group,enabled,created_at,updated_at FROM nodes ORDER BY created_at"
    ).fetch_all(pool.as_ref()).await.unwrap_or_default();
    ApiResp::ok(nodes)
}

/// Import a single node from its share URI.
#[post("/api/nodes/import")]
async fn import_node(req: HttpRequest, pool: Data<SqlitePool>, body: Json<ImportNodeReq>) -> impl Responder {
    auth!(req);
    let group = body.group.clone().unwrap_or_default();
    match upsert_node_from_uri(pool.as_ref(), &body.uri, &group).await {
        Ok(id) => ApiResp::ok(serde_json::json!({ "id": id })),
        Err(e) => ApiResp::<()>::err(e.to_string()),
    }
}

/// Batch import nodes from a subscription (frontend decodes base64 and splits lines).
#[post("/api/nodes/import-subscription")]
async fn import_subscription(req: HttpRequest, pool: Data<SqlitePool>, body: Json<ImportSubscriptionReq>) -> impl Responder {
    auth!(req);
    // Delete old nodes in this group to refresh
    let _ = sqlx::query("DELETE FROM nodes WHERE group_name=?")
        .bind(&body.group).execute(pool.as_ref()).await;
    let mut count = 0usize;
    for uri in &body.urls {
        if !uri.trim().is_empty() {
            if upsert_node_from_uri(pool.as_ref(), uri.trim(), &body.group).await.is_ok() {
                count += 1;
            }
        }
    }
    // Upsert subscription record
    let now = Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT OR REPLACE INTO subscriptions (id,name,url,last_update,created_at) VALUES (?,?,?,?,?)"
    ).bind(Uuid::new_v4().to_string()).bind(&body.name).bind("").bind(&now).bind(&now)
     .execute(pool.as_ref()).await;
    ApiResp::ok(serde_json::json!({ "imported": count }))
}

#[put("/api/nodes/{id}")]
async fn update_node(req: HttpRequest, pool: Data<SqlitePool>, path: WebPath<String>, body: Json<UpdateNodeReq>) -> impl Responder {
    auth!(req);
    let id = path.into_inner();
    let now = Utc::now().to_rfc3339();
    if let Some(true) | Some(false) = body.enabled {
        let _ = sqlx::query("UPDATE nodes SET enabled=?,updated_at=? WHERE id=?")
            .bind(body.enabled.unwrap()).bind(&now).bind(&id).execute(pool.as_ref()).await;
    }
    if let Some(name) = &body.name {
        let _ = sqlx::query("UPDATE nodes SET name=?,updated_at=? WHERE id=?")
            .bind(name).bind(&now).bind(&id).execute(pool.as_ref()).await;
    }
    ApiResp::ok(serde_json::json!({ "message": "Updated" }))
}

#[delete("/api/nodes/{id}")]
async fn delete_node(req: HttpRequest, pool: Data<SqlitePool>, path: WebPath<String>) -> impl Responder {
    auth!(req);
    let _ = sqlx::query("DELETE FROM nodes WHERE id=?").bind(path.into_inner()).execute(pool.as_ref()).await;
    ApiResp::ok(serde_json::json!({ "message": "Deleted" }))
}

// ─── API: subscriptions ───────────────────────────────────────────────────────
#[get("/api/subscriptions")]
async fn list_subscriptions(req: HttpRequest, pool: Data<SqlitePool>) -> impl Responder {
    auth!(req);
    let subs: Vec<Subscription> = sqlx::query_as("SELECT * FROM subscriptions ORDER BY created_at").fetch_all(pool.as_ref()).await.unwrap_or_default();
    ApiResp::ok(subs)
}

#[delete("/api/subscriptions/{id}")]
async fn delete_subscription(req: HttpRequest, pool: Data<SqlitePool>, path: WebPath<String>) -> impl Responder {
    auth!(req);
    let id = path.into_inner();
    // Also remove all nodes in that subscription group (name matches)
    let sub: Option<Subscription> = sqlx::query_as("SELECT * FROM subscriptions WHERE id=?")
        .bind(&id).fetch_optional(pool.as_ref()).await.unwrap_or(None);
    if let Some(s) = sub {
        let _ = sqlx::query("DELETE FROM nodes WHERE group_name=?").bind(&s.name).execute(pool.as_ref()).await;
    }
    let _ = sqlx::query("DELETE FROM subscriptions WHERE id=?").bind(&id).execute(pool.as_ref()).await;
    ApiResp::ok(serde_json::json!({ "message": "Deleted" }))
}

// ─── API: settings ────────────────────────────────────────────────────────────
#[get("/api/settings")]
async fn get_settings(req: HttpRequest, pool: Data<SqlitePool>) -> impl Responder {
    let _c = auth!(req, admin);
    let rows: Vec<Setting> = sqlx::query_as("SELECT key,value FROM settings ORDER BY key").fetch_all(pool.as_ref()).await.unwrap_or_default();
    let map: serde_json::Map<String,serde_json::Value> = rows.into_iter().map(|r| (r.key, r.value.into())).collect();
    ApiResp::ok(serde_json::Value::Object(map))
}

#[post("/api/settings")]
async fn save_setting(req: HttpRequest, pool: Data<SqlitePool>, body: Json<SaveSettingReq>) -> impl Responder {
    let _c = auth!(req, admin);
    let allowed = ["shoes_binary","local_port","log_level"];
    if !allowed.contains(&body.key.as_str()) { return ApiResp::<()>::err("Unknown key"); }
    let _ = sqlx::query("INSERT OR REPLACE INTO settings (key,value) VALUES (?,?)")
        .bind(&body.key).bind(&body.value).execute(pool.as_ref()).await;
    ApiResp::ok(serde_json::json!({ "message": "Saved" }))
}

// ─── API: proxy control ───────────────────────────────────────────────────────
#[derive(Serialize)]
struct StatusResp { running: bool, pid: Option<u32>, local_port: u16, node_count: usize }

#[get("/api/proxy/status")]
async fn proxy_status(req: HttpRequest, pool: Data<SqlitePool>, process: Data<SharedProcess>) -> impl Responder {
    auth!(req);
    let mut p = process.lock().unwrap();
    let running = p.is_running();
    let pid = if running { p.child.as_ref().map(|c| c.id()) } else { None };
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM nodes WHERE enabled=1").fetch_one(pool.as_ref()).await.unwrap_or(0);
    let local_port: u16 = db_get_setting(pool.as_ref(), "local_port").await.parse().unwrap_or(1080);
    ApiResp::ok(StatusResp { running, pid, local_port, node_count: count as usize })
}

/// Frontend sends the YAML it generated; backend writes it and starts shoes.
#[post("/api/proxy/start")]
async fn proxy_start(req: HttpRequest, pool: Data<SqlitePool>, process: Data<SharedProcess>, body: Json<StartProxyReq>) -> impl Responder {
    auth!(req);
    if body.yaml.trim().is_empty() { return ApiResp::<()>::err("YAML is empty"); }
    let db_binary = db_get_setting(pool.as_ref(), "shoes_binary").await;
    let local_port: u16 = db_get_setting(pool.as_ref(), "local_port").await.parse().unwrap_or(1080);
    let mut p = process.lock().unwrap();
    if !db_binary.is_empty() { p.binary_path = db_binary; }
    match p.start(&body.yaml) {
        Ok(_) => ApiResp::ok(serde_json::json!({ "message": format!("Shoes started on 127.0.0.1:{local_port}") })),
        Err(e) => ApiResp::<()>::err(format!("Failed to start: {e}")),
    }
}

#[post("/api/proxy/stop")]
async fn proxy_stop(req: HttpRequest, process: Data<SharedProcess>) -> impl Responder {
    auth!(req);
    let mut p = process.lock().unwrap();
    match p.stop() {
        Ok(_) => ApiResp::ok(serde_json::json!({ "message": "Shoes stopped." })),
        Err(e) => ApiResp::<()>::err(e.to_string()),
    }
}

// ─── API: setup check ────────────────────────────────────────────────────────
#[get("/api/setup/needed")]
async fn setup_needed(pool: Data<SqlitePool>) -> impl Responder {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(pool.as_ref()).await.unwrap_or(0);
    ApiResp::ok(serde_json::json!({ "needed": count == 0 }))
}

// ─── node URI helper ──────────────────────────────────────────────────────────
/// Extract minimal metadata from a share URI without full parsing.
/// Full parsing and YAML generation is done in the frontend.
fn parse_uri_meta(uri: &str) -> (String, String, String, i64) {
    // Returns (name, protocol, server, port)
    let protocol = if uri.starts_with("vmess://") { "vmess" }
        else if uri.starts_with("vless://") { "vless" }
        else if uri.starts_with("ss://") { "shadowsocks" }
        else if uri.starts_with("trojan://") { "trojan" }
        else if uri.starts_with("hy2://") || uri.starts_with("hysteria2://") { "hysteria2" }
        else if uri.starts_with("tuic://") { "tuic" }
        else if uri.starts_with("socks5://") { "socks5" }
        else if uri.starts_with("http://") || uri.starts_with("https://") { "http" }
        else { "unknown" };

    // Extract name from fragment (#...)
    let name = uri.split('#').nth(1)
        .map(|s| {
            // URL decode
            s.replace("%20"," ").replace("%2F","/").replace("%40","@")
             .replace("%23","#").replace("%3A",":").replace("%3F","?")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unnamed".to_string());

    // For vmess://base64, decode JSON to get host/port
    if protocol == "vmess" {
        let b64 = &uri[8..].split('#').next().unwrap_or("");
        if let Ok(decoded) = base64_decode(b64) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&decoded) {
                let server = v.get("add").and_then(|x|x.as_str()).unwrap_or("").to_string();
                let port: i64 = v.get("port").and_then(|x| x.as_str().and_then(|s|s.parse().ok()).or_else(|| x.as_i64())).unwrap_or(0);
                let ps = v.get("ps").and_then(|x|x.as_str()).unwrap_or(&name).to_string();
                return (ps, protocol.to_string(), server, port);
            }
        }
        return (name, protocol.to_string(), String::new(), 0);
    }

    // For URI-based formats: proto://[uuid@]host:port?...#name
    let body = &uri[uri.find("://").map(|i|i+3).unwrap_or(0)..];
    let body = body.split('#').next().unwrap_or("").split('?').next().unwrap_or("");
    let hostport = if let Some(at) = body.rfind('@') { &body[at+1..] } else { body };
    let (server, port) = if let Some(colon) = hostport.rfind(':') {
        let host = hostport[..colon].to_string();
        let p: i64 = hostport[colon+1..].parse().unwrap_or(0);
        (host, p)
    } else {
        (hostport.to_string(), 0)
    };

    (name, protocol.to_string(), server, port)
}

fn base64_decode(s: &str) -> Result<String> {
    use base64::{Engine as _, engine::general_purpose};
    // Support both standard and URL-safe base64, with or without padding
    let s = s.trim().replace('-', "+").replace('_', "/");
    let padding = (4 - s.len() % 4) % 4;
    let s = format!("{}{}", s, "=".repeat(padding));
    let bytes = general_purpose::STANDARD.decode(s.as_bytes())
        .map_err(|e| anyhow::anyhow!("base64 decode error: {e}"))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

async fn upsert_node_from_uri(pool: &SqlitePool, uri: &str, group: &str) -> Result<String> {
    let (name, protocol, server, port) = parse_uri_meta(uri);
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO nodes (id,name,protocol,server,port,uri,group_name,enabled,created_at,updated_at) VALUES (?,?,?,?,?,?,?,1,?,?)"
    ).bind(&id).bind(&name).bind(&protocol).bind(&server).bind(port)
     .bind(uri).bind(group).bind(&now).bind(&now)
     .execute(pool).await?;
    Ok(id)
}

// ─── main ─────────────────────────────────────────────────────────────────────
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let secret = env::var("KITEA_JWT_SECRET").unwrap_or_else(|_| {
        warn!("KITEA_JWT_SECRET not set; using random secret (sessions won't survive restarts)");
        Uuid::new_v4().to_string()
    });
    JWT_SECRET.set(secret).unwrap();

    let db_url = ensure_db_url();
    info!("Database: {db_url}");
    let pool = SqlitePool::connect(&db_url).await?;
    db_init(&pool).await?;

    let shoes_binary = {
        let from_env = env::var("KITEA_SHOES_BINARY").unwrap_or_default();
        if from_env.is_empty() {
            let from_db = db_get_setting(&pool, "shoes_binary").await;
            if from_db.is_empty() { "shoes".to_string() } else { from_db }
        } else { from_env }
    };
    let config_path = shoes_config_path();
    info!("Shoes binary: {shoes_binary}");
    info!("Shoes config: {}", config_path.display());

    let process: SharedProcess = Arc::new(Mutex::new(ShoesProcess::new(shoes_binary, config_path)));
    let listen = env::var("KITEA_LISTEN").unwrap_or_else(|_| "0.0.0.0:2026".to_string());
    info!("KiteA listening on http://{listen}");

    let pool_data = Data::new(pool);
    let process_data = Data::new(process);

    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .app_data(process_data.clone())
            .wrap(middleware::Logger::default())
            .wrap(Cors::default().allow_any_origin().allow_any_method().allow_any_header())
            // Auth
            .service(register).service(login).service(change_password)
            // Users
            .service(list_users).service(create_user).service(delete_user)
            // Nodes
            .service(list_nodes).service(import_node).service(import_subscription)
            .service(update_node).service(delete_node)
            // Subscriptions
            .service(list_subscriptions).service(delete_subscription)
            // Settings
            .service(get_settings).service(save_setting)
            // Proxy
            .service(proxy_status).service(proxy_start).service(proxy_stop)
            // Setup
            .service(setup_needed)
            // Frontend static assets
            .route("/", web::get().to(frontend_index))
            .route("/{path:.*}", web::get().to(frontend_assets))
    })
    .bind(&listen)?
    .run()
    .await?;
    Ok(())
}
