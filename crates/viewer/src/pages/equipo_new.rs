use dioxus::prelude::*;
use common::Equipo;
use crate::{AuthState, Route, ServerConfig};
use crate::components::equipo_form::EquipoForm;

#[component]
pub fn EquipoNew() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let server_config = use_context::<Signal<ServerConfig>>();
    let nav = use_navigator();

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
            let res = client.post(config.api_url("/api/equipos"))
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

    rsx! {
        section { class: "max-w-4xl mx-auto space-y-8 animate-in fade-in duration-500",
            header { class: "flex items-center gap-6",
                Link { to: Route::EquipoList {}, class: "w-12 h-12 bg-white flex items-center justify-center rounded-2xl border border-slate-200 hover:bg-slate-50 transition-all shadow-sm text-xl", "←" }
                h1 { class: "text-4xl font-black text-slate-900 tracking-tighter", "Registrar Nuevo Equipo" }
            }
            EquipoForm {
                on_submit: on_submit,
                initial_data: Equipo {
                    id: None, grupo: None, area: None, descripcion: None, ubicacion: None,
                    tipo: None, sistema_operativo: None, nombre_pc: None, usuario_windows: None,
                    clave_windows: None, clave_vnc: None, ip_address: "".to_string(),
                    observaciones: None, tipo_dispositivo: None, ubicacion_tecnica: None,
                    modificado_por: None, modificado_por_id: None, modificado_por_username: None,
                    fecha_modificacion: None, created_at: None, updated_at: None
                }
            }
        }
    }
}
