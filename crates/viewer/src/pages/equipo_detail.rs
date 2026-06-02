use dioxus::prelude::*;
use common::Equipo;
use crate::{AuthState, Route, ServerConfig};
use crate::components::info_group::InfoGroup;

#[component]
pub fn EquipoDetail(id: i32) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let server_config = use_context::<Signal<ServerConfig>>();
    let data = use_resource(move || {
        let token = auth.read().token.clone().unwrap_or_default();
        let config = server_config.read().clone();
        async move {
            let client = reqwest::Client::new();
            
            let eq_res = client
                .get(config.api_url(&format!("/api/equipos/{}", id)))
                .header("Authorization", format!("Bearer {}", token))
                .send().await;
                
            let eq = match eq_res {
                Ok(resp) => resp.json::<Option<Equipo>>().await.unwrap_or(None),
                Err(_) => None,
            };
            
            let mut logs = Vec::new();
            if let Some(ref e) = eq {
                let log_res = client
                    .get(config.api_url(&format!("/api/audit/{}", e.ip_address)))
                    .header("Authorization", format!("Bearer {}", token))
                    .send().await;
                if let Ok(resp) = log_res {
                    logs = resp.json::<Vec<common::AuditLog>>().await.unwrap_or_default();
                }
            }
            
            (eq, logs)
        }
    });

    rsx! {
        section { class: "max-w-5xl mx-auto space-y-10 animate-in fade-in zoom-in-95 duration-500",
            header { class: "flex items-center justify-between",
                section { class: "flex items-center gap-6",
                    Link { to: Route::EquipoList {}, class: "w-12 h-12 bg-white flex items-center justify-center rounded-[0.5rem] border border-slate-200 hover:bg-slate-50 transition-all shadow-sm text-xl", "←" }
                    section {
                        h1 { class: "text-4xl font-black text-slate-900 tracking-tighter", "Ficha Técnica" }
                    }
                }
                div { class: "bg-indigo-600 text-white px-6 py-2 rounded-full font-black text-[10px] uppercase tracking-widest shadow-lg", "ID Registro: {id}" }
            }
            match &*data.read() {
                Some((Some(e), logs)) => rsx! {
                    article { class: "bg-white rounded-[0.5rem] border border-slate-200/60 shadow-2xl overflow-hidden",
                        header { class: "bg-slate-900 p-12 text-white relative",
                            section { class: "relative z-10 flex justify-between items-end",
                                section {
                                    h2 { class: "text-4xl font-black tracking-tighter mb-2", "{e.nombre_pc.as_deref().unwrap_or(\"SIN NOMBRE\")}" }
                                    p { class: "font-mono text-indigo-400 text-lg font-bold tracking-widest", "{e.ip_address}" }
                                }
                                Link {
                                    to: Route::EquipoEdit { id: e.id.unwrap_or(0) },
                                    class: "bg-white text-slate-900 px-6 py-3 rounded-xl font-black uppercase tracking-widest text-[10px] hover:bg-indigo-50 transition-all shadow-lg",
                                    "Editar Ficha"
                                }
                            }
                        }
                        section { class: "p-12",
                            section { class: "grid grid-cols-1 md:grid-cols-3 gap-12",
                                InfoGroup { label: "Sistema Operativo".to_string(), value: e.sistema_operativo.clone().unwrap_or_else(|| "-".to_string()) }
                                InfoGroup { label: "Área".to_string(), value: e.area.clone().unwrap_or_else(|| "-".to_string()) }
                                InfoGroup { label: "Ubicación Física".to_string(), value: e.ubicacion.clone().unwrap_or_else(|| "-".to_string()) }
                                InfoGroup { label: "Usuario Windows".to_string(), value: e.usuario_windows.clone().unwrap_or_else(|| "-".to_string()) }
                                InfoGroup { label: "Clave VNC".to_string(), value: e.clave_vnc.clone().unwrap_or_else(|| "PROTEGIDA".to_string()) }
                                InfoGroup { label: "Tipo Dispositivo".to_string(), value: e.tipo_dispositivo.clone().unwrap_or_else(|| "INDUSTRIAL".to_string()) }
                                section { class: "md:col-span-3 pt-10 border-t border-slate-100",
                                    h3 { class: "text-[15px] font-black text-slate-400 uppercase tracking-[0.3em] mb-4", "Observaciones Técnicas" }
                                    section { class: "bg-slate-50 p-8 rounded-[0.5rem] border border-slate-100",
                                        p { class: "text-slate-600 leading-relaxed font-medium italic", "{e.descripcion.as_deref().unwrap_or(\"No hay descripción disponible.\")}" }
                                    }
                                }
                                section { class: "md:col-span-3 pt-10 border-t border-slate-100 space-y-6",
                                    h3 { class: "text-[15px] font-black text-slate-400 uppercase tracking-[0.3em]", "Historial de Cambios" }
                                    if logs.is_empty() {
                                        section { class: "bg-slate-50 p-8 rounded-[0.5rem] border border-slate-100 text-center text-slate-400 font-bold italic",
                                            "No hay registro de cambios para este equipo."
                                        }
                                    } else {
                                        div { class: "space-y-6 relative before:absolute before:inset-y-0 before:left-4 before:w-0.5 before:bg-slate-100",
                                            for log in logs.iter() {
                                                {
                                                    let fecha_str = log.fecha
                                                        .map(|f| f.format("%d/%m/%Y %H:%M").to_string())
                                                        .unwrap_or_default();
                                                    let usuario_str = log.usuario.as_deref().unwrap_or("System");
                                                    let (accion_class, dot_class) = match log.accion.as_str() {
                                                        "create" | "import" => ("bg-green-50 text-green-700 border-green-200", "bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.6)]"),
                                                        "update"            => ("bg-blue-50 text-blue-700 border-blue-200", "bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.6)]"),
                                                        "delete"            => ("bg-red-50 text-red-700 border-red-200", "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.6)]"),
                                                        _                   => ("bg-slate-50 text-slate-700 border-slate-200", "bg-slate-400"),
                                                    };
                                                    rsx! {
                                                        div { class: "relative pl-10 flex flex-col gap-2 group",
                                                            div { class: "absolute left-2.5 top-1.5 w-3.5 h-3.5 rounded-full border-4 border-white {dot_class} transition-all duration-300 group-hover:scale-125 z-10" }
                                                            header { class: "flex items-center gap-3 flex-wrap",
                                                                span { class: "text-xs font-bold text-slate-500", "{fecha_str}" }
                                                                span { class: "border px-2 py-0.5 rounded-full text-[9px] font-black uppercase tracking-wider {accion_class}", "{log.accion}" }
                                                                span { class: "text-xs font-bold text-slate-500", "por" }
                                                                span { class: "text-xs font-black text-slate-900 bg-slate-100 px-2 py-0.5 rounded-lg border border-slate-200", "{usuario_str}" }
                                                            }
                                                            {format_diff(&log.antes, &log.despues)}
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                Some((None, _)) => rsx! {
                    section { class: "p-20 text-center text-slate-400 font-black bg-white rounded-[3rem]", "Dispositivo no encontrado." }
                },
                None => rsx! {
                    section { class: "p-20 text-center text-slate-400 animate-pulse bg-white rounded-[3rem]", "Sincronizando..." }
                }
            }
        }
    }
}

fn format_diff(antes: &Option<serde_json::Value>, despues: &Option<serde_json::Value>) -> Element {
    let mut diffs = Vec::new();
    if let (Some(a), Some(d)) = (antes, despues) {
        if let (Some(obj_a), Some(obj_d)) = (a.as_object(), d.as_object()) {
            for (key, val_d) in obj_d {
                let val_a = obj_a.get(key).unwrap_or(&serde_json::Value::Null);
                
                let val_a_str = match val_a {
                    serde_json::Value::Null => "-".to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                let val_d_str = match val_d {
                    serde_json::Value::Null => "-".to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    v => v.to_string(),
                };
                
                let label = match key.as_str() {
                    "ip_address" => "Dirección IP",
                    "nombre_pc" => "Nombre PC",
                    "grupo" => "Grupo",
                    "area" => "Área",
                    "descripcion" => "Descripción",
                    "ubicacion" => "Ubicación",
                    "tipo" => "Tipo",
                    "sistema_operativo" => "Sistema Operativo",
                    "usuario_windows" => "Usuario Windows",
                    "clave_windows" => "Clave Windows",
                    "clave_vnc" => "Clave VNC",
                    "observaciones" => "Observaciones",
                    "tipo_dispositivo" => "Tipo Dispositivo",
                    "ubicacion_tecnica" => "Ubicación Técnica",
                    k => k,
                };
                
                diffs.push((label.to_string(), val_a_str, val_d_str));
            }
        }
    }
    
    if diffs.is_empty() {
        rsx! {
            span { class: "text-slate-400 italic text-[11px] font-medium", "Sin detalles de cambios" }
        }
    } else {
        rsx! {
            div { class: "space-y-1.5 mt-1 text-[11px]",
                for (field, from, to) in diffs {
                    div { class: "flex items-center gap-2 flex-wrap font-medium",
                        span { class: "text-slate-500 font-bold capitalize", "{field}:" }
                        span { class: "text-rose-600 bg-rose-50 px-2 py-0.5 rounded border border-rose-100 line-through font-mono", "{from}" }
                        span { class: "text-slate-400", "➔" }
                        span { class: "text-emerald-700 bg-emerald-50 px-2 py-0.5 rounded border border-emerald-100 font-mono font-bold", "{to}" }
                    }
                }
            }
        }
    }
}
