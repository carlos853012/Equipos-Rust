use dioxus::prelude::*;
use common::Equipo;
use crate::{AuthState, Route, ServerConfig};

#[component]
pub fn EquipoList() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let server_config = use_context::<Signal<ServerConfig>>();
    let mut search = use_signal(|| "".to_string());
    let mut scan_results = use_signal(|| std::collections::HashMap::<String, bool>::new());
    let mut is_scanning_all = use_signal(|| false);

    let role = auth.read().user.as_ref().map(|u| u.role.clone()).unwrap_or_default();
    let is_viewer = role == "viewer";
    let is_editor_or_admin = role == "editor" || role == "admin";

    let equipos = use_resource(move || {
        let token = auth.read().token.clone().unwrap_or_default();
        let search_val = search.read().clone();
        let config = server_config.read().clone();
        async move {
            let client = reqwest::Client::new();
            let mut url = config.api_url("/api/equipos");
            if !search_val.is_empty() {
                url = format!("{}?search={}", url, search_val);
            }
            let res = client.get(url)
                .header("Authorization", format!("Bearer {}", token))
                .send().await;
            match res {
                Ok(resp) => resp.json::<Vec<Equipo>>().await.unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        }
    });

    let run_mass_scan = move |_| {
        let config = server_config.read().clone();
        spawn(async move {
            is_scanning_all.set(true);
            let token = auth.read().token.clone().unwrap_or_default();
            let client = reqwest::Client::new();
            let res = client.get(config.api_url("/api/scan"))
                .header("Authorization", format!("Bearer {}", token))
                .send().await;

            if let Ok(resp) = res {
                if let Ok(results) = resp.json::<Vec<(String, bool)>>().await {
                    let mut current = scan_results.write();
                    for (ip, status) in results {
                        current.insert(ip, status);
                    }
                }
            }
            is_scanning_all.set(false);
        });
    };

    rsx! {
        section { class: "space-y-8 animate-in fade-in duration-500",
            header { class: "flex flex-col md:flex-row md:items-center justify-between gap-6",
                section {
                    h2 { class: "text-4xl font-black text-slate-900 tracking-tighter", "Inventario" }
                    p { class: "text-sm text-slate-500 font-bold uppercase tracking-widest", "Control de Activos Industriales" }
                }
                section { class: "flex gap-4",
                    button {
                        onclick: run_mass_scan,
                        disabled: *is_scanning_all.read(),
                        class: "bg-slate-900 text-white px-8 py-4 rounded-2xl font-black uppercase tracking-widest text-[10px] shadow-lg hover:bg-indigo-600 transition-all flex items-center gap-2",
                        if *is_scanning_all.read() { "⌛ Escaneando..." } else { span { "📡" } }
                        if !*is_scanning_all.read() { "Scan All" }
                    }
                    section { class: "relative",
                        span { class: "absolute left-4 top-1/2 -translate-y-1/2 text-slate-400", "🔍" }
                        input {
                            class: "pl-12 pr-6 py-4 bg-white border border-slate-200 rounded-2xl w-full md:w-80 font-bold outline-none focus:ring-2 ring-indigo-500/20 shadow-sm",
                            placeholder: "Buscar por IP o Nombre...",
                            value: "{search}",
                            oninput: move |evt| search.set(evt.value())
                        }
                    }
                    if is_editor_or_admin {
                        Link { to: Route::EquipoNew {}, class: "bg-indigo-600 text-white px-8 py-4 rounded-2xl font-black uppercase tracking-widest text-[10px] shadow-lg hover:bg-indigo-700 transition-all flex items-center gap-2",
                            span { "➕" }
                            "Nuevo"
                        }
                    }
                }
            }

            article { class: "bg-white rounded-[0.5rem] border border-slate-200 shadow-xl overflow-hidden",
                section { class: "overflow-x-auto",
                    table { class: "w-full text-left border-collapse",
                        thead { class: "bg-slate-900 text-white",
                            tr {
                                th { class: "px-8 py-6 text-[10px] font-black uppercase tracking-[0.2em]", "Estado" }
                                th { class: "px-8 py-6 text-[10px] font-black uppercase tracking-[0.2em]", "Dirección IP" }
                                th { class: "px-8 py-6 text-[10px] font-black uppercase tracking-[0.2em]", "Nombre Equipo" }
                                th { class: "px-8 py-6 text-[10px] font-black uppercase tracking-[0.2em]", "S.O." }
                                th { class: "px-8 py-6 text-[10px] font-black uppercase tracking-[0.2em] text-right", "Gestión" }
                            }
                        }
                        tbody { class: "divide-y divide-slate-100",
                            match equipos.read().as_ref() {
                                Some(list) => rsx! {
                                    if list.is_empty() {
                                        tr { td { colspan: "5", class: "px-8 py-20 text-center text-slate-400 font-bold italic", "No se encontraron equipos registrados." } }
                                    }
                                    for e in list.iter() {
                                        tr { class: "hover:bg-slate-200/75 transition-colors group",
                                            td { class: "px-8 py-6 whitespace-nowrap",
                                                {
                                                    let ip = e.ip_address.clone();
                                                    let status = scan_results.read().get(&ip).cloned();
                                                    match status {
                                                        Some(true) => rsx! {
                                                            section { class: "flex items-center gap-2",
                                                                span { class: "w-2.5 h-2.5 bg-green-500 rounded-full shadow-[0_0_8px_rgba(34,197,94,0.6)]" }
                                                                span { class: "text-[10px] font-black text-green-600 uppercase tracking-widest", "Online" }
                                                            }
                                                        },
                                                        Some(false) => rsx! {
                                                            section { class: "flex items-center gap-2",
                                                                span { class: "w-2.5 h-2.5 bg-rose-500 rounded-full shadow-[0_0_8px_rgba(244,63,94,0.6)]" }
                                                                span { class: "text-[10px] font-black text-rose-600 uppercase tracking-widest", "Offline" }
                                                            }
                                                        },
                                                        None => rsx! {
                                                            section { class: "flex items-center gap-2",
                                                                span { class: "w-2.5 h-2.5 bg-slate-300 rounded-full animate-pulse" }
                                                                span { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest", "Unknown" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            td { class: "px-8 py-6 whitespace-nowrap",
                                                span { class: "font-mono text-sm font-black text-indigo-600 bg-indigo-50 px-3 py-1.5 rounded-lg border border-indigo-100", "{e.ip_address}" }
                                            }
                                            td { class: "px-8 py-6 whitespace-nowrap text-sm font-bold text-slate-900 uppercase tracking-tight",
                                                "{e.nombre_pc.as_deref().unwrap_or(\"SYSTEM-NULL\")}"
                                            }
                                            td { class: "px-8 py-6 whitespace-nowrap",
                                                span { class: "inline-flex items-center px-3 py-1 rounded-full text-[10px] font-black uppercase tracking-widest bg-slate-100 text-slate-600 border border-slate-200",
                                                    "{e.sistema_operativo.as_deref().unwrap_or(\"UNKNOWN\")}"
                                                }
                                            }
                                            td { class: "px-8 py-6 whitespace-nowrap text-right flex justify-end gap-2",
                                                {
                                                    let ip = e.ip_address.clone();
                                                    let config = server_config.read().clone();
                                                    rsx! {
                                                        button {
                                                            onclick: move |_| {
                                                                let ip_clone = ip.clone();
                                                                let config_clone = config.clone();
                                                                spawn(async move {
                                                                    let token = auth.read().token.clone().unwrap_or_default();
                                                                    let client = reqwest::Client::new();
                                                                    let res = client.get(config_clone.api_url(&format!("/api/scan?ip={}", ip_clone)))
                                                                        .header("Authorization", format!("Bearer {}", token))
                                                                        .send().await;
                                                                    if let Ok(resp) = res {
                                                                        if let Ok(results) = resp.json::<Vec<(String, bool)>>().await {
                                                                            if let Some((_, status)) = results.first() {
                                                                                scan_results.write().insert(ip_clone, *status);
                                                                            }
                                                                        }
                                                                    }
                                                                });
                                                            },
                                                            class: "p-2 text-slate-600 rounded-full hover:bg-green-100 hover:text-green-600 transition-all",
                                                            title: "Realizar Ping Técnico",
                                                            "📡"
                                                        }
                                                    }
                                                }
                                                {
                                                    let ip = e.ip_address.clone();
                                                    rsx! {
                                                        button {
                                                            onclick: move |_| {
                                                                let url = format!("http://{}", ip);
                                                                let _ = webbrowser::open(&url);
                                                            },
                                                            class: "p-2 text-slate-600 rounded-full hover:bg-green-100 hover:text-green-600 transition-all",
                                                            title: "Abrir Interfaz Web",
                                                            "🌐"
                                                        }
                                                    }
                                                }
                                                Link { to: Route::EquipoDetail { id: e.id.unwrap_or(0) }, class: "inline-flex items-center gap-2 text-slate-600 px-4 py-2 rounded-full font-black uppercase tracking-[0.15em] text-[9px] hover:bg-green-100 hover:text-green-600 transition-all", "Ver" }
                                                if !is_viewer {
                                                    Link { to: Route::EquipoEdit { id: e.id.unwrap_or(0) }, class: "inline-flex items-center gap-2 text-slate-600 px-4 py-2 rounded-full font-black uppercase tracking-[0.15em] text-[9px] hover:bg-indigo-100 hover:text-indigo-600 transition-all", "Editar" }
                                                }
                                            }
                                        }
                                    }
                                },
                                None => rsx! {
                                    tr {
                                        td { colspan: "5", class: "px-8 py-20 text-center text-slate-400 font-bold italic", "Sincronizando..." }
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
