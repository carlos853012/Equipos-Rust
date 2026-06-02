use sqlx::PgPool;
use serde_json::{Value, Map};
use anyhow::Result;

pub async fn log_change(
    pool: &PgPool,
    equipo_id: Option<i32>,
    equipo_ip: &str,
    accion: &str,
    antes: Option<Value>,
    despues: Option<Value>,
    usuario: Option<&str>,
) -> Result<()> {
    let (diff_antes, diff_despues) = match (antes, despues) {
        (Some(a), Some(d)) => {
            let (final_a, final_d) = calculate_diff(&a, &d);
            (Some(final_a), Some(final_d))
        },
        (None, Some(d)) => {
            let redacted = redact_sensitive(d);
            (None, Some(redacted))
        },
        (a, d) => (a, d),
    };

    sqlx::query(
        "INSERT INTO equipo_audit_log (equipo_id, equipo_ip, accion, antes, despues, usuario)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(equipo_id)
    .bind(equipo_ip)
    .bind(accion)
    .bind(diff_antes)
    .bind(diff_despues)
    .bind(usuario)
    .execute(pool)
    .await?;
    
    Ok(())
}

const SENSITIVE_FIELDS: [&str; 3] = ["clave_windows", "clave_vnc", "usuario_windows"];

fn redact_sensitive(mut value: Value) -> Value {
    if let Some(obj) = value.as_object_mut() {
        for k in SENSITIVE_FIELDS {
            if obj.contains_key(k) {
                obj.insert(k.to_string(), Value::String("[CIFRADO]".to_string()));
            }
        }
    }
    value
}

/// Compara dos objetos JSON y devuelve solo los campos que cambiaron.
fn calculate_diff(antes: &Value, despues: &Value) -> (Value, Value) {
    let mut map_a = Map::new();
    let mut map_d = Map::new();

    if let (Some(obj_a), Some(obj_d)) = (antes.as_object(), despues.as_object()) {
        for (k, v_d) in obj_d {
            let v_a = obj_a.get(k).unwrap_or(&Value::Null);

            if v_a != v_d {
                // Metadatos de sistema que no aportan al diff
                if k == "updated_at" || k == "fecha_modificacion" || k == "created_at" {
                    continue;
                }
                // Campos sensibles: guardamos marca de redacción en vez del valor real
                if SENSITIVE_FIELDS.contains(&k.as_str()) {
                    map_a.insert(k.clone(), Value::String("[CIFRADO]".to_string()));
                    map_d.insert(k.clone(), Value::String("[CIFRADO]".to_string()));
                } else {
                    map_a.insert(k.clone(), v_a.clone());
                    map_d.insert(k.clone(), v_d.clone());
                }
            }
        }
    }

    (Value::Object(map_a), Value::Object(map_d))
}
