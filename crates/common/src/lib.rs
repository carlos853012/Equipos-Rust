use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow, PartialEq)]
pub struct Equipo {
    pub id: Option<i32>,
    pub grupo: Option<String>,
    pub area: Option<String>,
    pub descripcion: Option<String>,
    pub ubicacion: Option<String>,
    pub tipo: Option<String>,
    pub sistema_operativo: Option<String>,
    pub nombre_pc: Option<String>,
    pub usuario_windows: Option<String>,
    pub clave_windows: Option<String>,
    pub clave_vnc: Option<String>,
    pub ip_address: String,
    pub observaciones: Option<String>,
    pub tipo_dispositivo: Option<String>,
    pub ubicacion_tecnica: Option<String>,
    pub modificado_por: Option<String>,
    pub modificado_por_id: Option<i32>,
    pub modificado_por_username: Option<String>,
    pub fecha_modificacion: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct AuditLog {
    pub id: i32,
    pub equipo_id: Option<i32>,
    pub equipo_ip: String,
    pub accion: String, // 'create', 'update', 'delete', 'import'
    pub antes: Option<serde_json::Value>,
    pub despues: Option<serde_json::Value>,
    pub usuario: Option<String>,
    pub fecha: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password_hash: String,
    pub role: String, // 'admin', 'editor', 'viewer'
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportSummary {
    pub total: usize,
    pub created: usize,
    pub updated: usize,
    pub errors: Vec<String>,
}
