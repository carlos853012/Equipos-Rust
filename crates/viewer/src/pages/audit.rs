use dioxus::prelude::*;
use crate::{AuthState, ServerConfig};

#[component]
pub fn AuditLogGlobal() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let server_config = use_context::<Signal<ServerConfig>>();
    let mut active_log = use_signal(|| None::<common::AuditLog>);

    let logs = use_resource(move || {
        let token = auth.read().token.clone();
        let config = server_config.read().clone();
        async move {
            let client = reqwest::Client::new();
            let res = client.get(config.api_url("/api/audit"))
                .header("Authorization", format!("Bearer {}", token.as_deref().unwrap_or("")))
                .send()
                .await;

            match res {
                Ok(resp) => resp.json::<Vec<common::AuditLog>>().await.unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        }
    });

    let log_list: Vec<common::AuditLog> = logs.read().as_ref().cloned().unwrap_or_default();

    let rows = log_list.iter().map(|log| {
        let l = log.clone();
        let fecha_str = l.fecha
            .map(|f| f.format("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_default();
        let usuario_str = l.usuario.as_deref().unwrap_or("System").to_string();
        let accion_class = match l.accion.as_str() {
            "create" | "import" => "bg-green-100 text-green-700 px-2 py-1 rounded-full text-[9px] font-black uppercase",
            "update"            => "bg-blue-100 text-blue-700 px-2 py-1 rounded-full text-[9px] font-black uppercase",
            "delete"            => "bg-red-100 text-red-700 px-2 py-1 rounded-full text-[9px] font-black uppercase",
            _                   => "bg-slate-100 text-slate-700 px-2 py-1 rounded-full text-[9px] font-black uppercase",
        };
        let accion_str = l.accion.clone();
        let ip_str = l.equipo_ip.clone();
        let tiene_cambios = l.antes.is_some() || l.despues.is_some();
        rsx! {
            tr { class: "hover:bg-slate-200/75 transition-colors",
                td { class: "px-6 py-4 text-xs font-bold text-slate-500", "{fecha_str}" }
                td { class: "px-6 py-4",
                    section { class: "flex flex-col",
                        span { class: "text-xs font-black text-slate-900", "{ip_str}" }
                    }
                }
                td { class: "px-6 py-4",
                    span { class: accion_class, "{accion_str}" }
                }
                td { class: "px-6 py-4 text-xs font-bold text-slate-700", "{usuario_str}" }
                td { class: "px-6 py-4",
                    if tiene_cambios {
                        {
                            let l_click = l.clone();
                            rsx! {
                                button {
                                    class: "text-xs font-bold text-indigo-600 hover:text-indigo-800 transition-colors focus:outline-none",
                                    onclick: move |_| {
                                        active_log.set(Some(l_click.clone()));
                                    },
                                    "Ver Cambios"
                                }
                            }
                        }
                    } else {
                        span { class: "text-xs font-bold text-slate-300", "Sin datos" }
                    }
                }
            }
        }
    });

    rsx! {
        section { class: "space-y-6 relative",
            header {
                h2 { class: "text-2xl font-black text-slate-900", "Historial Global de Cambios" }
                p { class: "text-sm text-slate-500 font-bold", "Auditoría en tiempo real de toda la infraestructura" }
            }
            article { class: "bg-white rounded-[0.5rem] border border-slate-200 overflow-hidden shadow-sm",
                table { class: "w-full text-left border-collapse",
                    thead { class: "bg-slate-50 border-b border-slate-200",
                        tr {
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "Fecha" }
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "Equipo / IP" }
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "Acción" }
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "Usuario" }
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "Detalles" }
                        }
                    }
                    tbody { class: "divide-y divide-slate-100",
                        {rows}
                    }
                }
            }

            if let Some(selected_log) = active_log.read().clone() {
                {
                    let log = selected_log.clone();
                    let fecha_str = log.fecha
                        .map(|f| f.format("%d/%m/%Y %H:%M").to_string())
                        .unwrap_or_default();
                    let usuario_str = log.usuario.as_deref().unwrap_or("System");
                    let (accion_class, _dot_class) = match log.accion.as_str() {
                        "create" | "import" => ("bg-green-50 text-green-700 border-green-200", "bg-green-500"),
                        "update"            => ("bg-blue-50 text-blue-700 border-blue-200", "bg-blue-500"),
                        "delete"            => ("bg-red-50 text-red-700 border-red-200", "bg-red-500"),
                        _                   => ("bg-slate-50 text-slate-700 border-slate-200", "bg-slate-400"),
                    };
                    rsx! {
                        section { class: "fixed inset-0 bg-slate-900/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
                            article { class: "bg-white rounded-[0.5rem] shadow-2xl border border-slate-200 max-w-lg w-full overflow-hidden animate-in fade-in zoom-in duration-150",
                                header { class: "bg-slate-900 p-6 text-white flex justify-between items-center",
                                    h3 { class: "text-lg font-black tracking-tight", "Detalles de Modificación" }
                                    button {
                                        class: "text-slate-400 hover:text-white transition-colors text-lg font-bold",
                                        onclick: move |_| {
                                            active_log.set(None);
                                        },
                                        "✕"
                                    }
                                }
                                section { class: "p-6 space-y-6",
                                    section { class: "grid grid-cols-2 gap-4 bg-slate-50 p-4 rounded-xl border border-slate-100 text-xs font-medium",
                                        section { class: "space-y-1",
                                            span { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Equipo / IP" }
                                            p { class: "font-mono font-black text-slate-800", "{log.equipo_ip}" }
                                        }
                                        section { class: "space-y-1",
                                            span { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Fecha y Hora" }
                                            p { class: "text-slate-800 font-bold", "{fecha_str}" }
                                        }
                                        section { class: "space-y-1",
                                            span { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Usuario" }
                                            p { class: "text-slate-800 font-bold", "{usuario_str}" }
                                        }
                                        section { class: "space-y-1",
                                            span { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Acción" }
                                            p {
                                                span { class: "border px-2 py-0.5 rounded-full text-[9px] font-black uppercase tracking-wider {accion_class}", "{log.accion}" }
                                            }
                                        }
                                    }
                                    section { class: "space-y-3",
                                        h4 { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Diferencia de Campos" }
                                        section { class: "max-h-[300px] overflow-y-auto pr-2",
                                            {format_diff(&log.antes, &log.despues)}
                                        }
                                    }
                                }
                                footer { class: "bg-slate-50 px-6 py-4 flex justify-end border-t border-slate-100",
                                    button {
                                        class: "px-5 py-2.5 rounded-[0.5rem] text-xs font-black text-white bg-indigo-600 hover:bg-indigo-700 shadow-md transition-colors",
                                        onclick: move |_| {
                                            active_log.set(None);
                                        },
                                        "Cerrar"
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
