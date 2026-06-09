#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
#[cfg(not(target_os = "windows"))]
use tokio::signal;
use tokio::sync::oneshot;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use crate::auth::{validate_jwt, create_jwt, hash_password, verify_password, Claims};
use tower_http::cors::{Any, CorsLayer};

mod db_manager;
mod network_scanner;
mod audit;
mod auth;
mod crypto;
mod excel_import;

#[cfg(target_os = "windows")]
mod splash;
#[cfg(target_os = "windows")]
mod tray;

use db_manager::DbManager;
use network_scanner::{NetworkScanner, run_scan};
use excel_import::import_excel;
use tower_http::limit::RequestBodyLimitLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    #[cfg(target_os = "windows")]
    let (ready_tx, ready_rx) = oneshot::channel();

    #[cfg(target_os = "windows")]
    let splash_handle = std::thread::spawn(move || splash::show(ready_rx));

    let app_root = dirs::data_local_dir()
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
        .join("EquiposIndustriales")
        .join("data");
    let data_dir = app_root.join("pgdata");
    std::fs::create_dir_all(&data_dir).ok();
    crypto::init(&app_root);

    println!("[INFO] Usando base de datos en: {:?}", data_dir);

    let mut db_manager = DbManager::new(data_dir).await?;
    db_manager.start().await?;

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/equipos_redes".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("No se pudo conectar al Pool de SQLx");

    ensure_schema(&pool).await?;

    let scanner = Arc::new(NetworkScanner::new().expect("Error al iniciar escáner"));
    let state = AppState { pool, scanner };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(|| async { "Servidor Rust Activo 🦀" }))
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
                .nest(
                    "/users",
                    Router::new()
                        .route("/", get(get_users))
                        .route("/:id", put(update_user_role).delete(delete_user))
                        .route_layer(axum::middleware::from_fn(require_admin))
                )
                .route_layer(axum::middleware::from_fn_with_state(state.clone(), authenticate_middleware))
        )
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Servidor listo en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("No se pudo bindear el puerto");

    #[cfg(target_os = "windows")]
    let shutdown_signal = {
        let (tx, rx) = oneshot::channel::<()>();
        let _ = ready_tx.send(Ok(format!("http://{}", addr)));
        std::thread::spawn(move || tray::run(tx));
        async move { rx.await.ok(); }
    };
    #[cfg(not(target_os = "windows"))]
    let shutdown_signal = async {
        signal::ctrl_c().await.expect("Ctrl+C failed");
    };

    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal.await;
            println!("\n Apagando...");
        });

    server.await?;

    #[cfg(target_os = "windows")]
    {
        splash_handle.join().ok();
    }

    db_manager.stop().await?;
    Ok(())
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    scanner: Arc<NetworkScanner>,
}

#[derive(Deserialize)]
struct EquipoFilter {
    grupo: Option<String>,
    area: Option<String>,
    search: Option<String>,
}

async fn authenticate_middleware(
    State(_state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, (StatusCode, &'static str)> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..];
            match validate_jwt(token) {
                Ok(claims) => {
                    req.extensions_mut().insert(claims);
                    return Ok(next.run(req).await);
                }
                Err(_) => {
                    return Err((StatusCode::UNAUTHORIZED, "Invalid token"));
                }
            }
        }
    }

    Err((StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header"))
}

async fn require_admin(
    req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, (StatusCode, &'static str)> {
    let role = req
        .extensions()
        .get::<Claims>()
        .map(|c| c.role.as_str());

    match role {
        Some("admin") => Ok(next.run(req).await),
        _ => Err((StatusCode::FORBIDDEN, "Se requiere rol admin")),
    }
}

/// Helper: determina si el usuario es admin global (admin sin área)
fn is_global_admin(claims: &Claims) -> bool {
    claims.role == "admin" && claims.area.is_none()
}

/// Helper: determina si el usuario puede acceder a un equipo de cierta área
fn can_access_area(claims: &Claims, equipo_area: &Option<String>) -> bool {
    if is_global_admin(claims) {
        return true;
    }
    match (&claims.area, equipo_area) {
        (Some(user_area), Some(eq_area)) => user_area == eq_area,
        (Some(_), None) => false, // equipo sin área → solo admin global
        (None, _) => false,       // usuario sin área → sin acceso (viewer none)
    }
}

async fn get_equipos(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    _filter: Option<Query<EquipoFilter>>,
) -> Json<Vec<Equipo>> {
    // Usuario sin área → no ve nada
    if claims.area.is_none() && !is_global_admin(&claims) {
        return Json(vec![]);
    }

    let filter = _filter.map(|Query(filter)| filter).unwrap_or(EquipoFilter {
        grupo: None,
        area: None,
        search: None,
    });

    let mut query = QueryBuilder::new("SELECT * FROM equipos");
    let mut has_where = false;

    // Filtro de área forzado por el servidor según el token
    if is_global_admin(&claims) {
        // Admin global: respeta filtro de área del cliente si lo envía
        if let Some(area) = filter.area.filter(|v| !v.trim().is_empty()) {
            query.push(" WHERE area = ");
            query.push_bind(area);
            has_where = true;
        }
    } else {
        // Todos los demás: forzar su área del token, ignorar lo que mande el cliente
        query.push(" WHERE area = ");
        query.push_bind(claims.area.clone().unwrap());
        has_where = true;
    }

    if let Some(grupo) = filter.grupo.filter(|value| !value.trim().is_empty()) {
        query.push(if has_where { " AND " } else { " WHERE " });
        query.push("grupo = ");
        query.push_bind(grupo);
        has_where = true;
    }

    if let Some(search) = filter.search.filter(|value| !value.trim().is_empty()) {
        let pattern = format!("%{}%", search);
        query.push(if has_where { " AND " } else { " WHERE " });
        query.push("(ip_address ILIKE ");
        query.push_bind(pattern.clone());
        query.push(" OR nombre_pc ILIKE ");
        query.push_bind(pattern.clone());
        query.push(" OR descripcion ILIKE ");
        query.push_bind(pattern);
        query.push(")");
    }

    query.push(" ORDER BY id ASC");

    let mut equipos = query
        .build_query_as::<Equipo>()
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();
    for e in &mut equipos {
        e.clave_windows = crypto::decrypt_opt(&e.clave_windows);
        e.clave_vnc = crypto::decrypt_opt(&e.clave_vnc);
    }
    Json(equipos)
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
    )
    .execute(pool)
    .await?;

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
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            username VARCHAR(255) UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            role VARCHAR(50) DEFAULT 'viewer',
            area VARCHAR(255),
            created_at TIMESTAMPTZ DEFAULT NOW()
        )"
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_equipo(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<Json<Option<Equipo>>, (StatusCode, &'static str)> {
    let mut equipo = sqlx::query_as::<_, Equipo>("SELECT * FROM equipos WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or_default();

    if let Some(ref mut e) = equipo {
        if !can_access_area(&claims, &e.area) {
            return Err((StatusCode::FORBIDDEN, "No tienes acceso a este equipo"));
        }
        e.clave_windows = crypto::decrypt_opt(&e.clave_windows);
        e.clave_vnc = crypto::decrypt_opt(&e.clave_vnc);
    }
    Ok(Json(equipo))
}

async fn create_equipo(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Json(mut payload): Json<Equipo>,
) -> Result<Json<Option<Equipo>>, (StatusCode, &'static str)> {
    // Viewer nunca puede crear
    if claims.role == "viewer" {
        return Err((StatusCode::FORBIDDEN, "Los viewers no pueden crear equipos"));
    }
    // Sin área no puede crear
    if claims.area.is_none() && !is_global_admin(&claims) {
        return Err((StatusCode::FORBIDDEN, "Debes tener un área asignada para crear equipos"));
    }
    // No admin global: forzar área del token, bloquear cambio de área
    if !is_global_admin(&claims) {
        payload.area = claims.area.clone();
    }

    println!("[DEBUG] Recibida solicitud CREATE para IP: {}", payload.ip_address);
    let clave_windows = crypto::encrypt_opt(&payload.clave_windows);
    let clave_vnc = crypto::encrypt_opt(&payload.clave_vnc);
    let res = sqlx::query_as::<_, Equipo>(
        "INSERT INTO equipos (
            ip_address, nombre_pc, grupo, area, descripcion, ubicacion, tipo,
            sistema_operativo, usuario_windows, clave_windows, clave_vnc,
            observaciones, tipo_dispositivo, ubicacion_tecnica,
            modificado_por, modificado_por_id, modificado_por_username
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7,
            $8, $9, $10, $11,
            $12, $13, $14,
            $15, $16, $17
        )
        RETURNING *"
    )
    .bind(&payload.ip_address)
    .bind(&payload.nombre_pc)
    .bind(&payload.grupo)
    .bind(&payload.area)
    .bind(&payload.descripcion)
    .bind(&payload.ubicacion)
    .bind(&payload.tipo)
    .bind(&payload.sistema_operativo)
    .bind(&payload.usuario_windows)
    .bind(&clave_windows)
    .bind(&clave_vnc)
    .bind(&payload.observaciones)
    .bind(&payload.tipo_dispositivo)
    .bind(&payload.ubicacion_tecnica)
    .bind(&payload.modificado_por)
    .bind(&payload.modificado_por_id)
    .bind(&payload.modificado_por_username)
    .fetch_one(&state.pool).await;

    match res {
        Ok(mut e) => {
            println!("[DEBUG] Equipo creado ID: {:?}", e.id);
            let despues_val = serde_json::to_value(&e).ok();
            e.clave_windows = crypto::decrypt_opt(&e.clave_windows);
            e.clave_vnc = crypto::decrypt_opt(&e.clave_vnc);
            let _ = audit::log_change(
                &state.pool, e.id, &e.ip_address, "create", None, despues_val,
                payload.modificado_por.as_deref().or(payload.modificado_por_username.as_deref()).or(Some("system")),
            ).await;
            Ok(Json(Some(e)))
        },
        Err(err) => {
            println!("[ERROR] Fallo al insertar: {:?}", err);
            Ok(Json(None))
        }
    }
}

async fn update_equipo(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Path(id): Path<i32>,
    Json(mut payload): Json<Equipo>,
) -> Result<Json<Option<Equipo>>, (StatusCode, &'static str)> {
    if claims.role == "viewer" {
        return Err((StatusCode::FORBIDDEN, "Los viewers no pueden editar equipos"));
    }

    println!("[DEBUG] Recibida solicitud UPDATE para ID: {}", id);
    let before = match sqlx::query_as::<_, Equipo>("SELECT * FROM equipos WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
    {
        Ok(Some(equipo)) => equipo,
        Ok(None) | Err(_) => return Ok(Json(None)),
    };

    // Verificar que el usuario tenga acceso al área del equipo original
    if !can_access_area(&claims, &before.area) {
        return Err((StatusCode::FORBIDDEN, "No tienes acceso a este equipo"));
    }

    // Admin de área no puede reasignar equipos a otra área
    if !is_global_admin(&claims) {
        payload.area = claims.area.clone();
    }

    let clave_windows = crypto::encrypt_opt(&payload.clave_windows);
    let clave_vnc = crypto::encrypt_opt(&payload.clave_vnc);
    let res = sqlx::query_as::<_, Equipo>(
        "UPDATE equipos SET
            ip_address = $1, nombre_pc = $2, grupo = $3, area = $4,
            descripcion = $5, ubicacion = $6, tipo = $7,
            sistema_operativo = $8, usuario_windows = $9,
            clave_windows = $10, clave_vnc = $11, observaciones = $12,
            tipo_dispositivo = $13, ubicacion_tecnica = $14,
            modificado_por = $15, modificado_por_id = $16,
            modificado_por_username = $17,
            fecha_modificacion = NOW(), updated_at = NOW()
        WHERE id = $18
        RETURNING *"
    )
    .bind(&payload.ip_address)
    .bind(&payload.nombre_pc)
    .bind(&payload.grupo)
    .bind(&payload.area)
    .bind(&payload.descripcion)
    .bind(&payload.ubicacion)
    .bind(&payload.tipo)
    .bind(&payload.sistema_operativo)
    .bind(&payload.usuario_windows)
    .bind(&clave_windows)
    .bind(&clave_vnc)
    .bind(&payload.observaciones)
    .bind(&payload.tipo_dispositivo)
    .bind(&payload.ubicacion_tecnica)
    .bind(&payload.modificado_por)
    .bind(&payload.modificado_por_id)
    .bind(&payload.modificado_por_username)
    .bind(id)
    .fetch_one(&state.pool)
    .await;

    match res {
        Ok(mut updated) => {
            println!("[DEBUG] Equipo ID {} actualizado", id);
            let despues_val = serde_json::to_value(&updated).ok();
            updated.clave_windows = crypto::decrypt_opt(&updated.clave_windows);
            updated.clave_vnc = crypto::decrypt_opt(&updated.clave_vnc);
            let _ = audit::log_change(
                &state.pool, updated.id, &updated.ip_address, "update",
                serde_json::to_value(&before).ok(), despues_val,
                payload.modificado_por.as_deref().or(payload.modificado_por_username.as_deref()).or(Some("system")),
            ).await;
            Ok(Json(Some(updated)))
        }
        Err(err) => {
            println!("[ERROR] Fallo al actualizar ID {}: {:?}", id, err);
            Ok(Json(None))
        },
    }
}

async fn delete_equipo(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<&'static str, (StatusCode, &'static str)> {
    if claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Solo los administradores pueden eliminar equipos"));
    }

    let before = sqlx::query_as::<_, Equipo>("SELECT * FROM equipos WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

    if let Some(ref equipo) = before {
        if !can_access_area(&claims, &equipo.area) {
            return Err((StatusCode::FORBIDDEN, "No tienes acceso a este equipo"));
        }
    }

    let _ = sqlx::query("DELETE FROM equipos WHERE id = $1").bind(id).execute(&state.pool).await;

    if let Some(equipo) = before {
        let _ = audit::log_change(
            &state.pool, equipo.id, &equipo.ip_address, "delete",
            serde_json::to_value(&equipo).ok(), None,
            equipo.modificado_por.as_deref().or(equipo.modificado_por_username.as_deref()).or(Some("system")),
        ).await;
    }

    Ok("OK")
}

async fn get_audit_log(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Path(ip): Path<String>,
) -> Json<Vec<AuditLog>> {
    // Verificar que la IP pertenezca al área del usuario
    if !is_global_admin(&claims) {
        if let Some(ref user_area) = claims.area {
            let equipo = sqlx::query_as::<_, Equipo>("SELECT * FROM equipos WHERE ip_address = $1")
                .bind(&ip)
                .fetch_optional(&state.pool)
                .await
                .unwrap_or_default();
            match equipo {
                Some(e) if e.area.as_deref() == Some(user_area.as_str()) => {},
                _ => return Json(vec![]),
            }
        } else {
            return Json(vec![]);
        }
    }

    let logs = sqlx::query_as::<_, AuditLog>(
        "SELECT * FROM equipo_audit_log WHERE equipo_ip = $1 ORDER BY fecha DESC"
    )
    .bind(ip)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();
    Json(logs)
}

async fn get_all_audit_logs(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
) -> Json<Vec<AuditLog>> {
    if is_global_admin(&claims) {
        // Admin global: ve todos los logs
        let logs = sqlx::query_as::<_, AuditLog>(
            "SELECT * FROM equipo_audit_log ORDER BY fecha DESC LIMIT 200"
        )
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();
        return Json(logs);
    }

    // Sin área: no ve nada
    let Some(ref user_area) = claims.area else {
        return Json(vec![]);
    };

    // Con área: solo logs de equipos de su área via JOIN
    let logs = sqlx::query_as::<_, AuditLog>(
        "SELECT al.* FROM equipo_audit_log al
         INNER JOIN equipos e ON e.ip_address = al.equipo_ip
         WHERE e.area = $1
         ORDER BY al.fecha DESC
         LIMIT 200"
    )
    .bind(user_area)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();
    Json(logs)
}

#[derive(Deserialize)]
struct ScanQuery {
    ip: Option<String>,
}

async fn scan_network(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Query(query): Query<ScanQuery>,
) -> Json<Vec<(String, bool)>> {
    let ips = if let Some(ip) = query.ip {
        // Ping a IP específica: verificar que pertenezca al área del usuario
        if !is_global_admin(&claims) {
            if let Some(ref user_area) = claims.area {
                let equipo = sqlx::query_as::<_, Equipo>("SELECT * FROM equipos WHERE ip_address = $1")
                    .bind(&ip)
                    .fetch_optional(&state.pool)
                    .await
                    .unwrap_or_default();
                match equipo {
                    Some(e) if e.area.as_deref() == Some(user_area.as_str()) => vec![ip],
                    _ => return Json(vec![]),
                }
            } else {
                return Json(vec![]);
            }
        } else {
            vec![ip]
        }
    } else {
        // Scan masivo: filtrar por área
        let query_str = if is_global_admin(&claims) {
            "SELECT ip_address FROM equipos".to_string()
        } else if let Some(ref area) = claims.area {
            format!("SELECT ip_address FROM equipos WHERE area = '{}'", area.replace('\'', "''"))
        } else {
            return Json(vec![]);
        };
        let rows = sqlx::query(&query_str).fetch_all(&state.pool).await.unwrap_or_default();
        rows.into_iter().map(|r| r.get::<String, _>("ip_address")).collect()
    };

    let results = run_scan(state.scanner.clone(), ips).await;
    Json(results)
}

#[derive(Serialize)]
struct AuthStatus {
    setup_required: bool,
}

async fn get_auth_status(State(state): State<AppState>) -> Json<AuthStatus> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    
    Json(AuthStatus {
        setup_required: count == 0,
    })
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    user: User,
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (axum::http::StatusCode, &'static str)> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        .ok_or_else(|| (axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials"))?;

    if !verify_password(&payload.password, &user.password_hash) {
        return Err((axum::http::StatusCode::UNAUTHORIZED, "Invalid credentials"));
    }

    let token = create_jwt(&user);

    Ok(Json(LoginResponse {
        token,
        user,
    }))
}

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
    role: Option<String>,
    area: Option<String>,
}

async fn register(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<User>, (axum::http::StatusCode, &'static str)> {
    let existing = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(&payload.username)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

    if existing {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Username already exists"));
    }

    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    // Primer usuario: se crea como admin global sin autenticación
    if user_count == 0 {
        let hashed = hash_password(&payload.password);
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (username, password_hash, role, area) VALUES ($1, $2, 'admin', NULL) RETURNING *"
        )
        .bind(&payload.username)
        .bind(&hashed)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user"))?;
        return Ok(Json(user));
    }

    // Para crear más usuarios se requiere ser admin autenticado
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let claims = match auth_header {
        Some(token) => validate_jwt(token).map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "Token inválido"))?,
        None => return Err((axum::http::StatusCode::FORBIDDEN, "Se requiere autenticación de admin")),
    };

    if claims.role != "admin" {
        return Err((axum::http::StatusCode::FORBIDDEN, "Solo admins pueden crear usuarios"));
    }

    // Determinar rol y área según quién crea
    let (role, area) = if is_global_admin(&claims) {
        // Admin global: puede asignar cualquier rol y área
        (payload.role.unwrap_or_else(|| "viewer".to_string()), payload.area)
    } else {
        // Admin de área: solo puede crear usuarios de su misma área, nunca admins globales
        let assigned_area = claims.area.clone();
        let role = payload.role.unwrap_or_else(|| "viewer".to_string());
        if role == "admin" && payload.area.is_none() {
            return Err((axum::http::StatusCode::FORBIDDEN, "No puedes crear admins globales"));
        }
        (role, assigned_area)
    };

    let hashed = hash_password(&payload.password);
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password_hash, role, area) VALUES ($1, $2, $3, $4) RETURNING *"
    )
    .bind(&payload.username)
    .bind(&hashed)
    .bind(&role)
    .bind(&area)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user"))?;

    Ok(Json(user))
}

async fn get_users(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
) -> Json<Vec<User>> {
    if is_global_admin(&claims) {
        let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY username ASC")
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();
        return Json(users);
    }
    // Admin de área: solo ve usuarios de su área
    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE area = $1 ORDER BY username ASC"
    )
    .bind(&claims.area)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();
    Json(users)
}

#[derive(Deserialize)]
struct UpdateRoleRequest {
    role: String,
    area: Option<String>,
}

async fn update_user_role(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRoleRequest>,
) -> Result<Json<User>, (StatusCode, &'static str)> {
    // Bloquear auto-edición
    if claims.user_id == id {
        return Err((StatusCode::FORBIDDEN, "No puedes modificar tu propio usuario"));
    }

    // Obtener el usuario a modificar
    let target = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        .ok_or((StatusCode::NOT_FOUND, "Usuario no encontrado"))?;

    // Admin de área: solo puede modificar usuarios de su misma área
    if !is_global_admin(&claims) {
        if target.area != claims.area {
            return Err((StatusCode::FORBIDDEN, "Solo puedes gestionar usuarios de tu área"));
        }
        // Admin de área no puede crear otros admins globales (sin área)
        if payload.role == "admin" && payload.area.is_none() {
            return Err((StatusCode::FORBIDDEN, "No puedes crear admins globales"));
        }
    }

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET role = $1, area = $2 WHERE id = $3 RETURNING *"
    )
    .bind(&payload.role)
    .bind(&payload.area)
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to update user"))?;

    Ok(Json(user))
}

async fn delete_user(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<Claims>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    if claims.user_id == id {
        return Err((StatusCode::FORBIDDEN, "No puedes eliminarte a ti mismo"));
    }

    if !is_global_admin(&claims) {
        let target = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .unwrap_or_default();
        if let Some(t) = target {
            if t.area != claims.area {
                return Err((StatusCode::FORBIDDEN, "Solo puedes eliminar usuarios de tu área"));
            }
        }
    }

    match sqlx::query("DELETE FROM users WHERE id = $1").bind(id).execute(&state.pool).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
