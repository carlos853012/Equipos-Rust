use dioxus::prelude::*;
use common::Equipo;
use crate::{AuthState, Route, ServerConfig};
use crate::components::equipo_form::EquipoForm;

#[component]
pub fn EquipoEdit(id: i32) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let server_config = use_context::<Signal<ServerConfig>>();
    let nav = use_navigator();
    let equipo_res = use_resource(move || {
        let token = auth.read().token.clone().unwrap_or_default();
        let config = server_config.read().clone();
        async move {
            let client = reqwest::Client::new();
            client.get(config.api_url(&format!("/api/equipos/{}", id)))
                .header("Authorization", format!("Bearer {}", token))
                .send().await.unwrap()
                .json::<Option<Equipo>>().await.unwrap_or_default()
        }
    });

    let on_submit = move |mut data: Equipo| {
        let current_user = auth.read().user.clone();
        if let Some(user) = current_user {
            data.modificado_por = Some(user.username.clone());
            data.modificado_por_id = Some(user.id);
            data.modificado_por_username = Some(user.username.clone());
        }
        let config = server_config.read().clone();
        spawn(async move {
            let token = auth.read().token.clone().unwrap_or_default();
            let client = reqwest::Client::new();
            let res = client.put(config.api_url(&format!("/api/equipos/{}", id)))
                .header("Authorization", format!("Bearer {}", token))
                .json(&data)
                .send()
                .await;
            if let Ok(response) = res {
                if response.status().is_success() {
                    nav.push(Route::EquipoList {});
                }
            }
        });
    };

    let equipo_read = equipo_res.read();
    let x = match equipo_read.as_ref() {
        Some(Some(e)) => rsx! {
            section { class: "max-w-4xl mx-auto space-y-8 animate-in fade-in duration-500",
                header { class: "flex items-center gap-6",
                    Link { to: Route::EquipoList {}, class: "w-12 h-12 bg-white flex items-center justify-center rounded-2xl border border-slate-200 hover:bg-slate-50 transition-all shadow-sm text-xl", "←" }
                    h1 { class: "text-4xl font-black text-slate-900 tracking-tighter", "Editar Especificaciones" }
                }
                EquipoForm { on_submit: on_submit, initial_data: e.clone() }
            }
        },
        _ => rsx! {
            section { class: "p-20 text-center text-slate-400 font-black animate-pulse", "Cargando ficha para edición..." }
        }
    };
    x
}
