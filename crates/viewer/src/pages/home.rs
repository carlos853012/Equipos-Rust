use dioxus::prelude::*;
use common::Equipo;
use crate::{AuthState, Route, ServerConfig};

#[component]
pub fn Home() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let server_config = use_context::<Signal<ServerConfig>>();
    let mut stats = use_resource(move || {
        let token = auth.read().token.clone().unwrap_or_default();
        let config = server_config.read().clone();
        async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            // 1. Fetch equipo list
            let eq_url = config.api_url("/api/equipos");
            let eq_res = client.get(&eq_url)
                .header("Authorization", format!("Bearer {}", token))
                .send().await;

            let equipos = match eq_res {
                Ok(resp) => resp.json::<Vec<Equipo>>().await.unwrap_or_default(),
                Err(_) => Vec::new(),
            };

            let total = equipos.len();

            // 2. Perform network scan via server to check statuses
            let scan_url = config.api_url("/api/scan");
            let scan_res = client.get(&scan_url)
                .header("Authorization", format!("Bearer {}", token))
                .send().await;

            let mut activos = 0;
            let mut alertas = 0;
            let mut offline_ips = Vec::new();

            if let Ok(resp) = scan_res {
                if let Ok(results) = resp.json::<Vec<(String, bool)>>().await {
                    for (ip, status) in results {
                        if status {
                            activos += 1;
                        } else {
                            alertas += 1;
                            if let Some(eq) = equipos.iter().find(|e| e.ip_address == ip) {
                                offline_ips.push(format!("{} ({})", eq.nombre_pc.as_deref().unwrap_or("Dispositivo"), ip));
                            } else {
                                offline_ips.push(ip);
                            }
                        }
                    }
                }
            }

            (total, activos, alertas, offline_ips)
        }
    });

    rsx! {
        section { class: "space-y-10 animate-in fade-in slide-in-from-bottom-5 duration-700",
            header {
                h2 { class: "text-xl font-black text-slate-900 tracking-tighter",
                    "Dashboard"
                    span { class: "text-indigo-600", " Operacional" }
                }
                p { class: "text-slate-500 font-bold mt-2 uppercase tracking-widest text-xs", "Estado general de la infraestructura de red" }
            }

            section { class: "grid grid-cols-1 md:grid-cols-3 gap-8",
                StatCard {
                    label: "Total Equipos".to_string(),
                    value: stats.read().as_ref().map(|s| s.0.to_string()).unwrap_or_else(|| "...".into()),
                    icon: "📦",
                    color: "indigo".to_string()
                }
                StatCard {
                    label: "Nodos Activos".to_string(),
                    value: stats.read().as_ref().map(|s| s.1.to_string()).unwrap_or_else(|| "...".into()),
                    icon: "🟢",
                    color: "emerald".to_string()
                }
                StatCard {
                    label: "Alertas Críticas".to_string(),
                    value: stats.read().as_ref().map(|s| s.2.to_string()).unwrap_or_else(|| "...".into()),
                    icon: "🚨",
                    color: "rose".to_string()
                }
            }

            if let Some(offline_list) = stats.read().as_ref().map(|s| &s.3) {
                if !offline_list.is_empty() {
                    section { class: "bg-red-50 border border-red-200 rounded-[0.5rem] p-6 space-y-3 shadow-md animate-in fade-in slide-in-from-top-3 duration-300",
                        header { class: "flex items-center gap-3 text-red-700",
                            span { class: "text-2xl animate-bounce", "🚨" }
                            h3 { class: "text-base font-black tracking-tight", "Nodos Industriales Fuera de Línea (Offline)" }
                        }
                        ul { class: "list-disc pl-5 text-xs text-red-750 font-bold space-y-1.5",
                            for item in offline_list.iter() {
                                li { "El equipo " span { class: "underline font-black", "{item}" } " no responde a los comandos de diagnóstico (Ping)." }
                            }
                        }
                    }
                }
            }

            article { class: "bg-white p-12 rounded-[0.5rem] border border-slate-200 shadow-xl",
                h3 { class: "text-2xl font-black text-slate-900 mb-6 tracking-tight", "Acciones Rápidas" }
                section { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6",
                    QuickAction { to: Route::EquipoNew {}, label: "Nuevo Equipo".to_string(), icon: "➕", desc: "Registrar dispositivo".to_string() }
                    QuickAction { to: Route::EquipoList {}, label: "Ver Listado".to_string(), icon: "📋", desc: "Gestión de inventario".to_string() }
                    QuickAction { to: Route::Import {}, label: "Importar Excel".to_string(), icon: "📥", desc: "Carga masiva".to_string() }
                    button {
                        onclick: move |_| {
                            stats.restart();
                        },
                        class: "group bg-slate-50 p-6 rounded-[0.5rem] border border-slate-100 hover:bg-indigo-600 hover:border-indigo-600 text-left transition-all focus:outline-none",
                        section { class: "text-3xl mb-4 group-hover:scale-110 transition-transform", "🔄" }
                        h4 { class: "font-black text-slate-900 group-hover:text-white transition-colors", "Sincronizar" }
                        p { class: "text-xs text-slate-500 group-hover:text-indigo-100 transition-colors mt-1", "Refrescar estado de red" }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(label: String, value: String, icon: &'static str, color: String) -> Element {
    rsx! {
        article { class: "bg-white p-8 rounded-[0.5rem] border border-slate-200 shadow-sm hover:shadow-xl transition-all group overflow-hidden relative",
            section { class: format!("absolute top-0 right-0 w-32 h-32 bg-{}-50 rounded-full -mr-16 -mt-16 transition-transform group-hover:scale-110", color) }
            section { class: "relative z-10",
                section { class: "text-4xl mb-4", "{icon}" }
                h4 { class: "text-[10px] font-black text-slate-400 uppercase tracking-[0.3em] mb-1", "{label}" }
                p { class: "text-4xl font-black text-slate-900 tracking-tighter", "{value}" }
            }
        }
    }
}

#[component]
fn QuickAction(to: Route, label: String, icon: &'static str, desc: String) -> Element {
    rsx! {
        Link { to: to, class: "group bg-slate-50 p-6 rounded-[0.5rem] border border-slate-100 hover:bg-indigo-600 hover:border-indigo-600 transition-all",
            section { class: "text-3xl mb-4 group-hover:scale-110 transition-transform", "{icon}" }
            h4 { class: "font-black text-slate-900 group-hover:text-white transition-colors", "{label}" }
            p { class: "text-xs text-slate-500 group-hover:text-indigo-100 transition-colors mt-1", "{desc}" }
        }
    }
}
