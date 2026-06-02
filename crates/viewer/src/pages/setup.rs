use dioxus::prelude::*;
use crate::{Route, ServerConfig};

#[component]
pub fn Setup() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let nav = use_navigator();
    let server_config = use_context::<Signal<ServerConfig>>();

    let on_submit = move |_| {
        let config = server_config.read().clone();
        spawn(async move {
            loading.set(true);
            error.set(None);
            let client = reqwest::Client::new();
            let register_url = config.api_url("/register");
            let res = client.post(&register_url)
                .json(&serde_json::json!({
                    "username": username.read().to_string(),
                    "password": password.read().to_string()
                }))
                .send()
                .await;
            match res {
                Ok(response) if response.status().is_success() => {
                    nav.push(Route::Login {});
                }
                _ => {
                    error.set(Some("Error al crear usuario administrador".to_string()));
                }
            }
            loading.set(false);
        });
    };

    rsx! {
        section { class: "min-h-[80vh] flex items-center justify-center",
            article { class: "w-full max-w-md bg-white rounded-[0.5rem] shadow-2xl border border-slate-200 overflow-hidden",
                header { class: "bg-indigo-600 p-10 text-white text-center",
                    section { class: "w-16 h-16 bg-white/20 rounded-2xl flex items-center justify-center mx-auto mb-6 border border-white/30", "🛠️" }
                    h2 { class: "text-3xl font-black tracking-tighter", "Configuración" }
                    p { class: "text-indigo-100 text-sm mt-2 uppercase tracking-widest font-bold", "Crear Primer Administrador" }
                }
                section { class: "p-10 space-y-6",
                    if let Some(err) = error.read().as_ref() {
                        section { class: "bg-red-50 text-red-600 p-4 rounded-[0.5rem] text-xs font-bold border border-red-100", "{err}" }
                    }
                    section { class: "space-y-4",
                        section { class: "space-y-2",
                            label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Usuario Administrador" }
                            input {
                                class: "w-full bg-slate-50 border border-slate-200 rounded-[0.5rem] px-5 py-4 font-bold outline-none",
                                value: "{username}",
                                oninput: move |evt| username.set(evt.value())
                            }
                        }
                        section { class: "space-y-2",
                            label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Contraseña Maestra" }
                            input {
                                r#type: "password",
                                class: "w-full bg-slate-50 border border-slate-200 rounded-[0.5rem] px-5 py-4 font-bold outline-none",
                                value: "{password}",
                                oninput: move |evt| password.set(evt.value())
                            }
                        }
                    }
                    button {
                        onclick: on_submit,
                        disabled: *loading.read(),
                        class: "w-full bg-slate-900 text-white py-5 rounded-[0.5rem] font-black uppercase tracking-[0.2em] text-xs shadow-xl",
                        "Crear Admin e Iniciar"
                    }
                }
            }
        }
    }
}
