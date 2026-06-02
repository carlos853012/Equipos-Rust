use axum::{routing::{get, post, put}, Json, Router, extract::{State, Path, Query}};
use axum::middleware::Next;
use axum::http::{Request, Response, StatusCode, header};
use axum::body::Body;
use std::net::SocketAddr;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, QueryBuilder};
use common::{Equipo, AuditLog, User};
use dotenvy::dotenv;
use std::env;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use crate::auth::{validate_jwt, create_jwt, hash_password, verify_password};
use tower_http::cors::{Any, CorsLayer};
use crate::db_manager::DbManager;
use crate::network_scanner::{NetworkScanner, run_scan};
use crate::excel_import::import_excel;
use tower_http::limit::RequestBodyLimitLayer;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub scanner: Arc<NetworkScanner>,
}

/// Prepara toda la infraestructura y retorna sin bloquear.
/// El caller es responsable de llamar axum::serve(listener, app).
pub async fn start() -> anyhow::Result<(String, Router, tokio::net::TcpListener)> {
    dotenv().ok();

    let data_dir = {
    #[cfg(target_os = "windows")]
    {
        if cfg!(debug_assertions) {
            std::path::PathBuf::from("data/pgdata")
        } else {
            std::path::PathBuf::from("C:\\ProgramData\\EquiposIndustriales\\pgdata")
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::path::PathBuf::from("/var/lib/equipos-industriales/pgdata")
    }
};

    std::fs::create_dir_all(&data_dir).ok();
    println!("[INFO] Usando base de datos en: {:?}", data_dir);

    let mut db_manager = DbManager::new(data_dir).await?;
    db_manager.start().await?;

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/equipos_redes".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    ensure_schema(&pool).await?;

    let scanner = Arc::new(NetworkScanner::new()?);
    let state = AppState { pool, scanner };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(|| async { "Servidor Rust Activo" }))
        .route("/auth/status", get(get_auth_status))
        .route("/login", post(login))
        .route("/register", post(register))
        .nest(
            "/api",
            Router::new()
                .route("/equipos", get(get_equipos).post(create_equipo))
                .route("/equipos/:id", get(get_equipo).put(update_equipo).delete(delete_equipo))
                .route("/import", post(import_excel).layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)))
                .route("/scan", get(scan_network))
                .route("/audit", get(get_all_audit_logs))
                .route("/audit/:ip", get(get_audit_log))
                .route("/users", get(get_users))
                .route("/users/:id", put(update_user_role).delete(delete_user))
                .route_layer(axum::middleware::from_fn_with_state(state.clone(), authenticate_middleware))
        )
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let addr_str = format!("http://{}", addr);
    println!("Servidor listo en {}", addr_str);

    // Retorna el listener y el router sin bloquear
    Ok((addr_str, app, listener))
}

async fn ensure_schema(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS equipos (
            id SERIAL PRIMARY KEY,
            ip_address VARCHAR(45) UNIQUE NOT NULL,
            nombre_pc VARCHAR(255),
            grupo VARCHAR(255),
            area VARCHAR(255),
            descripcion TEXT,
            ubicacion VARCHAR(255),
            tipo VARCHAR(255),
            sistema_operativo VARCHAR(255),
            usuario_windows VARCHAR(255),
            clave_windows VARCHAR(255),
            clave_vnc VARCHAR(255),
            observaciones TEXT,
            tipo_dispositivo VARCHAR(255),
            ubicacion_tecnica TEXT,
            modificado_por VARCHAR(255),
            modificado_por_id INT,
            modificado_por_username VARCHAR(255),
            fecha_modificacion TIMESTAMPTZ DEFAULT NOW(),
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS equipo_audit_log (
            id SERIAL PRIMARY KEY,
            equipo_id INT,
            equipo_ip VARCHAR(45) NOT NULL,
            accion VARCHAR(50) NOT NULL,
            antes JSONB,
            despues JSONB,
            usuario VARCHAR(255),
            fecha TIMESTAMPTZ DEFAULT NOW()
        )"
    ).execute(pool).await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            username VARCHAR(255) UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            role VARCHAR(50) DEFAULT 'viewer',
            created_at TIMESTAMPTZ DEFAULT NOW()
        )"
    ).execute(pool).await?;

    Ok(())
}

async fn authenticate_middleware(
    State(_state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, (StatusCode, &'static str)> {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..];
            match validate_jwt(token) {
                Ok(_) => return Ok(next.run(req).await),
                Err(_) => return Err((StatusCode::UNAUTHORIZED, "Invalid token")),
            }
        }
    }
    Err((StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header"))
}

#[derive(Deserialize)]
struct EquipoFilter {
    grupo: Option<String>,
    area: Option<String>,
    search: Option<String>,
}

async fn get_equipos(State(state): State<AppState>, _filter: Option<Query<EquipoFilter>>) -> Json<Vec<Equipo>> {
    let filter = _filter.map(|Query(f)| f).unwrap_or(EquipoFilter { grupo: None, area: None, search: None });
    let mut query = QueryBuilder::new("SELECT * FROM equipos");
    let mut has_where = false;

    if let Some(grupo) = filter.grupo.filter(|v| !v.trim().is_empty()) {
        query.push(if has_where { " AND " } else { " WHERE " });
        query.push("grupo = "); query.push_bind(grupo);
        has_where = true;
    }
    if let Some(area) = filter.area.filter(|v| !v.trim().is_empty()) {
        query.push(if has_where { " AND " } else { " WHERE " });
        query.push("area = "); query.push_bind(area);
        has_where = true;
    }
    if let Some(search) = filter.search.filter(|v| !v.trim().is_empty()) {
        let pattern = format!("%{}%", search);
        query.push(if has_where { " AND " } else { " WHERE " });
        query.push("(ip_address ILIKE "); query.push_bind(pattern.clone());
        query.push(" OR nombre_pc ILIKE "); query.push_bind(pattern.clone());
        query.push(" OR descripcion ILIKE "); query.push_bind(pattern);
        query.push(")");
    }
    query.push(" ORDER BY id ASC");

    Json(query.build_query_as::<Equipo>().fetch_all(&state.pool).await.unwrap_or_default())
}

async fn get_equipo(State(state): State<AppState>, Path(id): Path<i32>) -> Json<Option<Equipo>> {
    Json(sqlx::query_as::<_, Equipo>("SELECT * FROM equipos WHERE id = $1")
        .bind(id).fetch_optional(&state.pool).await.unwrap_or_default())
}

async fn create_equipo(State(state): State<AppState>, Json(payload): Json<Equipo>) -> Json<Option<Equipo>> {
    let res = sqlx::query_as::<_, Equipo>(
        "INSERT INTO equipos (ip_address, nombre_pc, grupo, area, descripcion, ubicacion, tipo,
            sistema_operativo, usuario_windows, clave_windows, clave_vnc, observaciones,
            tipo_dispositivo, ubicacion_tecnica, modificado_por, modificado_por_id, modificado_por_username)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17) RETURNING *"
    )
    .bind(&payload.ip_address).bind(&payload.nombre_pc).bind(&payload.grupo)
    .bind(&payload.area).bind(&payload.descripcion).bind(&payload.ubicacion)
    .bind(&payload.tipo).bind(&payload.sistema_operativo).bind(&payload.usuario_windows)
    .bind(&payload.clave_windows).bind(&payload.clave_vnc).bind(&payload.observaciones)
    .bind(&payload.tipo_dispositivo).bind(&payload.ubicacion_tecnica)
    .bind(&payload.modificado_por).bind(&payload.modificado_por_id)
    .bind(&payload.modificado_por_username)
    .fetch_one(&state.pool).await;

    match res {
        Ok(e) => {
            let _ = crate::audit::log_change(&state.pool, e.id, &e.ip_address, "create",
                None, serde_json::to_value(&e).ok(),
                payload.modificado_por.as_deref().or(Some("system"))).await;
            Json(Some(e))
        }
        Err(e) => { println!("[ERROR] {e:?}"); Json(None) }
    }
}

async fn update_equipo(State(state): State<AppState>, Path(id): Path<i32>, Json(payload): Json<Equipo>) -> Json<Option<Equipo>> {
    let before = match sqlx::query_as::<_, Equipo>("SELECT * FROM equipos WHERE id = $1")
        .bind(id).fetch_optional(&state.pool).await {
        Ok(Some(e)) => e,
        _ => return Json(None),
    };

    let res = sqlx::query_as::<_, Equipo>(
        "UPDATE equipos SET ip_address=$1, nombre_pc=$2, grupo=$3, area=$4, descripcion=$5,
            ubicacion=$6, tipo=$7, sistema_operativo=$8, usuario_windows=$9, clave_windows=$10,
            clave_vnc=$11, observaciones=$12, tipo_dispositivo=$13, ubicacion_tecnica=$14,
            modificado_por=$15, modificado_por_id=$16, modificado_por_username=$17,
            fecha_modificacion=NOW(), updated_at=NOW()
         WHERE id=$18 RETURNING *"
    )
    .bind(&payload.ip_address).bind(&payload.nombre_pc).bind(&payload.grupo)
    .bind(&payload.area).bind(&payload.descripcion).bind(&payload.ubicacion)
    .bind(&payload.tipo).bind(&payload.sistema_operativo).bind(&payload.usuario_windows)
    .bind(&payload.clave_windows).bind(&payload.clave_vnc).bind(&payload.observaciones)
    .bind(&payload.tipo_dispositivo).bind(&payload.ubicacion_tecnica)
    .bind(&payload.modificado_por).bind(&payload.modificado_por_id)
    .bind(&payload.modificado_por_username).bind(id)
    .fetch_one(&state.pool).await;

    match res {
        Ok(updated) => {
            let _ = crate::audit::log_change(&state.pool, updated.id, &updated.ip_address, "update",
                serde_json::to_value(&before).ok(), serde_json::to_value(&updated).ok(),
                payload.modificado_por.as_deref().or(Some("system"))).await;
            Json(Some(updated))
        }
        Err(e) => { println!("[ERROR] {e:?}"); Json(None) }
    }
}

async fn delete_equipo(State(state): State<AppState>, Path(id): Path<i32>) -> &'static str {
    let before = sqlx::query_as::<_, Equipo>("SELECT * FROM equipos WHERE id = $1")
        .bind(id).fetch_optional(&state.pool).await.ok().flatten();
    let _ = sqlx::query("DELETE FROM equipos WHERE id = $1").bind(id).execute(&state.pool).await;
    if let Some(e) = before {
        let _ = crate::audit::log_change(&state.pool, e.id, &e.ip_address, "delete",
            serde_json::to_value(&e).ok(), None,
            e.modificado_por.as_deref().or(Some("system"))).await;
    }
    "OK"
}

async fn get_audit_log(State(state): State<AppState>, Path(ip): Path<String>) -> Json<Vec<AuditLog>> {
    Json(sqlx::query_as::<_, AuditLog>("SELECT * FROM equipo_audit_log WHERE equipo_ip = $1 ORDER BY fecha DESC")
        .bind(ip).fetch_all(&state.pool).await.unwrap_or_default())
}

async fn get_all_audit_logs(State(state): State<AppState>) -> Json<Vec<AuditLog>> {
    Json(sqlx::query_as::<_, AuditLog>("SELECT * FROM equipo_audit_log ORDER BY fecha DESC LIMIT 200")
        .fetch_all(&state.pool).await.unwrap_or_default())
}

#[derive(Deserialize)]
struct ScanQuery { ip: Option<String> }

async fn scan_network(State(state): State<AppState>, Query(query): Query<ScanQuery>) -> Json<Vec<(String, bool)>> {
    let ips = if let Some(ip) = query.ip {
        vec![ip]
    } else {
        sqlx::query("SELECT ip_address FROM equipos").fetch_all(&state.pool).await
            .unwrap_or_default().into_iter().map(|r| r.get::<String, _>("ip_address")).collect()
    };
    Json(run_scan(state.scanner.clone(), ips).await)
}

#[derive(Serialize)]
struct AuthStatus { setup_required: bool }

async fn get_auth_status(State(state): State<AppState>) -> Json<AuthStatus> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool).await.unwrap_or(0);
    Json(AuthStatus { setup_required: count == 0 })
}

#[derive(Deserialize)]
struct LoginRequest { username: String, password: String }

#[derive(Serialize)]
struct LoginResponse { token: String, user: User }

async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>)
    -> Result<Json<LoginResponse>, (StatusCode, &'static str)>
{
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&payload.username).fetch_optional(&state.pool).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid credentials"))?;

    if !verify_password(&payload.password, &user.password_hash) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials"));
    }
    Ok(Json(LoginResponse { token: create_jwt(&user), user }))
}

#[derive(Deserialize)]
struct RegisterRequest { username: String, password: String, role: Option<String> }

async fn register(State(state): State<AppState>, Json(payload): Json<RegisterRequest>)
    -> Result<Json<User>, (StatusCode, &'static str)>
{
    let existing = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(&payload.username).fetch_one(&state.pool).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if existing { return Err((StatusCode::BAD_REQUEST, "Username already exists")); }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool).await.unwrap_or(0);
    let role = if count == 0 { "admin".to_string() } else { payload.role.unwrap_or_else(|| "viewer".to_string()) };

    let user = sqlx::query_as::<_, User>("INSERT INTO users (username, password_hash, role) VALUES ($1, $2, $3) RETURNING *")
        .bind(&payload.username).bind(&hash_password(&payload.password)).bind(&role)
        .fetch_one(&state.pool).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user"))?;

    Ok(Json(user))
}

async fn get_users(State(state): State<AppState>) -> Json<Vec<User>> {
    Json(sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY username ASC")
        .fetch_all(&state.pool).await.unwrap_or_default())
}

#[derive(Deserialize)]
struct UpdateRoleRequest { role: String }

async fn update_user_role(State(state): State<AppState>, Path(id): Path<i32>, Json(payload): Json<UpdateRoleRequest>)
    -> Result<Json<User>, (StatusCode, &'static str)>
{
    let user = sqlx::query_as::<_, User>("UPDATE users SET role = $1 WHERE id = $2 RETURNING *")
        .bind(&payload.role).bind(id).fetch_one(&state.pool).await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update user role"))?;
    Ok(Json(user))
}

async fn delete_user(State(state): State<AppState>, Path(id): Path<i32>) -> StatusCode {
    match sqlx::query("DELETE FROM users WHERE id = $1").bind(id).execute(&state.pool).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
