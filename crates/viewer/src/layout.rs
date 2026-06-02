use dioxus::prelude::*;
use crate::{AuthState, AuthStatus, Route, ServerConfig};

#[component]
pub fn MainLayout() -> Element {
    let mut auth = use_context::<Signal<AuthState>>();
    let server_config = use_context::<Signal<ServerConfig>>();
    let nav = use_navigator();
    let route = use_route::<Route>();

    use_effect(move || {
        let is_setup = matches!(route, Route::Setup {});
        let is_login = matches!(route, Route::Login {});
        let config = server_config.read().clone();
        spawn(async move {
            let url = config.api_url("/auth/status");
            if let Ok(res) = reqwest::get(&url).await {
                if let Ok(status) = res.json::<AuthStatus>().await {
                    if status.setup_required && !is_setup {
                        nav.push(Route::Setup {});
                        return;
                    }
                }
            }
            if auth.read().token.is_none() && !is_login && !is_setup {
                nav.push(Route::Login {});
            }
        });
    });

    rsx! {
        section { class: "min-h-screen bg-slate-50 flex flex-col font-sans text-slate-900 antialiased",
            if let Some(user) = auth.read().user.as_ref() {
                header { class: "bg-white border-b border-slate-200 sticky top-0 z-50 shadow-sm",
                    section { class: "max-w-7xl mx-auto px-4",
                        section { class: "flex justify-between h-16",
                            section { class: "flex items-center gap-8",
                                Link { to: Route::Home {}, class: "flex items-center gap-3 group",
                                    section { class: "flex flex-col justify-center",
                                        h1 { class: "text-sm font-black text-slate-900 uppercase",
                                            "Equipos"
                                            br {}
                                            span { class: "text-indigo-600", "Industriales" }
                                        }
                                    }
                                }
                                nav { class: "hidden md:flex space-x-1",
                                    NavLink { to: Route::Home {}, icon: "📊", label: "Dashboard" }
                                    NavLink { to: Route::EquipoList {}, icon: "📋", label: "Inventario" }
                                    NavLink { to: Route::Import {}, icon: "📥", label: "Importar" }
                                    if user.role == "admin" {
                                        NavLink { to: Route::AuditLogGlobal {}, icon: "📜", label: "Historial" }
                                        NavLink { to: Route::UserManagement {}, icon: "👥", label: "Usuarios" }
                                    }
                                }
                            }
                            section { class: "flex items-center gap-4",
                                section { class: "flex flex-col text-right",
                                    span { class: "text-xs font-black text-slate-700", "{user.username}" }
                                    span { class: "text-[10px] font-bold text-indigo-500 uppercase", "{user.role}" }
                                }
                                button {
                                    onclick: move |_| {
                                        *auth.write() = AuthState { token: None, user: None };
                                        nav.push(Route::Login {});
                                    },
                                    class: "w-10 h-10 bg-slate-100 rounded-full flex items-center justify-center text-slate-400 hover:text-red-500 transition-colors",
                                    "🚪"
                                }
                            }
                        }
                    }
                }
            }
            main { class: "flex-1 max-w-7xl mx-auto w-full px-4 py-8",
                Outlet::<Route> {}
            }
        }
    }
}

#[component]
pub fn NavLink(to: Route, icon: &'static str, label: &'static str) -> Element {
    let current_route = use_route::<Route>();
    let is_active = current_route == to;
    rsx! {
        Link {
            to: to,
            class: format!(
                "flex items-center gap-3 px-6 py-2 rounded-xl text-sm font-bold transition-all {}",
                if is_active { "bg-slate-900 text-white shadow-lg" } else { "text-slate-500 hover:bg-slate-100 hover:text-slate-900" }
            ),
            span { class: "text-lg", "{icon}" }
            span { "{label}" }
        }
    }
}
