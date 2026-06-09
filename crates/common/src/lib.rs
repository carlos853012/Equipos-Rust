use serde::{Deserialize, Serialize};

/// Estructura que representa un equipo o activo industrial en la red de la planta.
/// Contiene metadatos físicos, técnicos, operativos y de seguridad del nodo.
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow, PartialEq)]
pub struct Equipo {
    /// Identificador único del equipo en la base de datos.
    pub id: Option<i32>,
    /// Grupo, planta o proceso productivo al que pertenece.
    pub grupo: Option<String>,
    /// Área operacional asignada (para control de acceso multi-área).
    pub area: Option<String>,
    /// Descripción del propósito o función del dispositivo.
    pub descripcion: Option<String>,
    /// Ubicación física o lógica corta del equipo.
    pub ubicacion: Option<String>,
    /// Variante, modelo o marca del hardware.
    pub tipo: Option<String>,
    /// Sistema operativo instalado en el equipo.
    pub sistema_operativo: Option<String>,
    /// Nombre de red del equipo (Hostname).
    pub nombre_pc: Option<String>,
    /// Nombre del usuario del sistema operativo local.
    pub usuario_windows: Option<String>,
    /// Contraseña cifrada del usuario de Windows.
    pub clave_windows: Option<String>,
    /// Contraseña cifrada de acceso remoto VNC.
    pub clave_vnc: Option<String>,
    /// Dirección IP única en la red industrial.
    pub ip_address: String,
    /// Observaciones técnicas generales.
    pub observaciones: Option<String>,
    /// Tipo de dispositivo (ej. PLC, SCADA, Switch, PC).
    pub tipo_dispositivo: Option<String>,
    /// Ubicación física estructurada detallada.
    pub ubicacion_tecnica: Option<String>,
    /// Nombre del usuario que realizó la última edición.
    pub modificado_por: Option<String>,
    /// ID del usuario que realizó la última edición.
    pub modificado_por_id: Option<i32>,
    /// Username del usuario que realizó la última edición.
    pub modificado_por_username: Option<String>,
    /// Timestamp de la última modificación.
    pub fecha_modificacion: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp de creación del registro.
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp de la última actualización.
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Registro de auditoría para trazar los cambios realizados sobre los equipos.
/// Permite reconstruir eventos y modificaciones históricas de forma segura.
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct AuditLog {
    /// Identificador único del log de auditoría.
    pub id: i32,
    /// ID del equipo afectado (si aún existe).
    pub equipo_id: Option<i32>,
    /// IP del equipo afectado para trazabilidad permanente.
    pub equipo_ip: String,
    /// Acción realizada (ej. 'create', 'update', 'delete', 'import').
    pub accion: String,
    /// Estado de las propiedades del equipo antes de la acción (JSON).
    pub antes: Option<serde_json::Value>,
    /// Estado de las propiedades del equipo después de la acción (JSON).
    pub despues: Option<serde_json::Value>,
    /// Nombre del usuario u origen de la modificación.
    pub usuario: Option<String>,
    /// Fecha y hora en la que ocurrió el evento de auditoría.
    pub fecha: Option<chrono::DateTime<chrono::Utc>>,
}

/// Representa a un usuario operador o administrador registrado en el sistema.
#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct User {
    /// Identificador único del usuario.
    pub id: i32,
    /// Nombre de usuario para iniciar sesión.
    pub username: String,
    /// Hash seguro de la contraseña.
    pub password_hash: String,
    /// Rol asignado en el sistema ('admin', 'editor', 'viewer').
    pub role: String,
    /// Área asignada al usuario (None representa acceso de administrador global).
    pub area: Option<String>,
    /// Timestamp de creación de la cuenta.
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Resumen de los resultados obtenidos tras un proceso de importación masiva de Excel.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportSummary {
    /// Número total de registros analizados.
    pub total: usize,
    /// Cantidad de nuevos equipos creados.
    pub created: usize,
    /// Cantidad de equipos existentes actualizados.
    pub updated: usize,
    /// Listado de mensajes de error con ubicación de fila en caso de fallo.
    pub errors: Vec<String>,
}
