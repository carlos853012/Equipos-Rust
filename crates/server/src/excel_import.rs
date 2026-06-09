use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use calamine::{Reader, Xlsx, Data};
use std::io::Cursor;
use sqlx::PgPool;
use common::{Equipo, ImportSummary};
use serde::Serialize;
use crate::AppState;
use crate::audit;

pub async fn import_excel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<crate::auth::Claims>,
    mut multipart: Multipart,
) -> Result<Json<ImportSummary>, (StatusCode, String)> {
    if claims.role == "viewer" {
        return Err((StatusCode::FORBIDDEN, "Los viewers no pueden importar equipos".to_string()));
    }
    if claims.area.is_none() && claims.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Debes tener un área asignada para importar".to_string()));
    }
    let mut data = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("Error en multipart: {}", e))
    })? {
        if field.name() == Some("file") {
            data = Some(field.bytes().await.map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Error al leer bytes: {}", e))
            })?);
            break;
        }
    }

    let bytes = data.ok_or((StatusCode::BAD_REQUEST, "No se encontró el archivo".to_string()))?;
    let cursor = Cursor::new(bytes);
    let mut workbook: Xlsx<_> = Xlsx::new(cursor).map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("Error al abrir Excel: {}", e))
    })?;

    let mut summary = ImportSummary {
        total: 0,
        created: 0,
        updated: 0,
        errors: Vec::new(),
    };

    // Procesar todas las hojas o solo la primera? Por ahora procesamos todas las que tengan datos.
    let sheet_names = workbook.sheet_names().to_vec();
    for sheet_name in sheet_names {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            // Suponemos que la primera fila es el encabezado
            let mut rows = range.rows();
            let _headers = rows.next(); // Skip headers

            for (row_idx, row) in rows.enumerate() {
                summary.total += 1;
                
                // Mapeo manual según COLUMNS_ORDER de analizar_excel.py
                // 0: grupo, 1: area, 2: descripcion, 3: ubicacion, 4: tipo, 5: sistema_operativo, 
                // 6: nombre_pc, 7: usuario_windows, 8: clave_windows, 9: clave_vnc, 10: ip_address, 11: observaciones
                
                let ip_address = get_row_string(row, 10);
                if ip_address.is_empty() {
                    continue; // Skip rows without IP
                }

                let area_forzada = if claims.role == "admin" && claims.area.is_none() {
                    // Admin global: respeta el área del Excel
                    Some(get_row_string(row, 1)).filter(|s| !s.is_empty())
                } else {
                    // Admin/editor de área: fuerza su área
                    claims.area.clone()
                };

                let equipo = Equipo {
                    id: None,
                    ip_address: ip_address.clone(),
                    nombre_pc: Some(get_row_string(row, 6)),
                    grupo: Some(get_row_string(row, 0)),
                    area: area_forzada,
                    descripcion: Some(get_row_string(row, 2)),
                    ubicacion: Some(get_row_string(row, 3)),
                    tipo: Some(get_row_string(row, 4)),
                    sistema_operativo: Some(get_row_string(row, 5)),
                    usuario_windows: Some(get_row_string(row, 7)),
                    clave_windows: crate::crypto::encrypt_opt(&Some(get_row_string(row, 8))),
                    clave_vnc: crate::crypto::encrypt_opt(&Some(get_row_string(row, 9))),
                    observaciones: Some(get_row_string(row, 11)),
                    tipo_dispositivo: None,
                    ubicacion_tecnica: None,
                    modificado_por: Some("EXCEL_IMPORT".to_string()),
                    modificado_por_id: None,
                    modificado_por_username: Some("system".to_string()),
                    fecha_modificacion: None,
                    created_at: None,
                    updated_at: None,
                };

                match upsert_equipo(&state.pool, &equipo).await {
                    Ok(is_update) => {
                        if is_update {
                            summary.updated += 1;
                        } else {
                            summary.created += 1;
                        }
                    }
                    Err(e) => {
                        summary.errors.push(format!("Error en hoja {}, fila {}: {}", sheet_name, row_idx + 2, e));
                    }
                }
            }
        }
    }

    Ok(Json(summary))
}

fn get_row_string(row: &[Data], idx: usize) -> String {
    row.get(idx)
        .map(|d| match d {
            Data::String(s) => s.trim().to_string(),
            Data::Float(f) => f.to_string(),
            Data::Int(i) => i.to_string(),
            _ => "".to_string(),
        })
        .unwrap_or_default()
}

async fn upsert_equipo(pool: &PgPool, equipo: &Equipo) -> anyhow::Result<bool> {
    // Verificar si existe para el log de auditoría
    let existing = sqlx::query_as::<_, Equipo>("SELECT * FROM equipos WHERE ip_address = $1")
        .bind(&equipo.ip_address)
        .fetch_optional(pool)
        .await?;

    let is_update = existing.is_some();

    let res = sqlx::query_as::<_, Equipo>(
        "INSERT INTO equipos (
            ip_address, nombre_pc, grupo, area, descripcion, ubicacion, tipo,
            sistema_operativo, usuario_windows, clave_windows, clave_vnc,
            observaciones, modificado_por, modificado_por_username
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7,
            $8, $9, $10, $11,
            $12, $13, $14
        )
        ON CONFLICT (ip_address) DO UPDATE SET
            nombre_pc = EXCLUDED.nombre_pc,
            grupo = EXCLUDED.grupo,
            area = EXCLUDED.area,
            descripcion = EXCLUDED.descripcion,
            ubicacion = EXCLUDED.ubicacion,
            tipo = EXCLUDED.tipo,
            sistema_operativo = EXCLUDED.sistema_operativo,
            usuario_windows = EXCLUDED.usuario_windows,
            clave_windows = EXCLUDED.clave_windows,
            clave_vnc = EXCLUDED.clave_vnc,
            observaciones = EXCLUDED.observaciones,
            modificado_por = EXCLUDED.modificado_por,
            modificado_por_username = EXCLUDED.modificado_por_username,
            fecha_modificacion = NOW(),
            updated_at = NOW()
        RETURNING *"
    )
    .bind(&equipo.ip_address)
    .bind(&equipo.nombre_pc)
    .bind(&equipo.grupo)
    .bind(&equipo.area)
    .bind(&equipo.descripcion)
    .bind(&equipo.ubicacion)
    .bind(&equipo.tipo)
    .bind(&equipo.sistema_operativo)
    .bind(&equipo.usuario_windows)
    .bind(&equipo.clave_windows)
    .bind(&equipo.clave_vnc)
    .bind(&equipo.observaciones)
    .bind(&equipo.modificado_por)
    .bind(&equipo.modificado_por_username)
    .fetch_one(pool)
    .await?;

    // Auditoría
    let _ = audit::log_change(
        pool,
        res.id,
        &res.ip_address,
        if is_update { "update" } else { "import" },
        existing.and_then(|e| serde_json::to_value(e).ok()),
        serde_json::to_value(&res).ok(),
        Some("EXCEL_IMPORT"),
    ).await;

    Ok(is_update)
}
