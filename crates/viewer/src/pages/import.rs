use dioxus::prelude::*;
use crate::{AuthState, ServerConfig};

#[component]
pub fn Import() -> Element {
    let mut status = use_signal(|| "".to_string());
    let mut loading = use_signal(|| false);
    let auth = use_context::<Signal<AuthState>>();
    let server_config = use_context::<Signal<ServerConfig>>();

    let upload_file = move |evt: Event<FormData>| {
        let token = auth.read().token.clone();
        let config = server_config.read().clone();
        spawn(async move {
            if let Some(file_engine) = evt.files() {
                let files = file_engine.files();
                if let Some(file_name) = files.first() {
                    loading.set(true);
                    status.set(format!("Subiendo {}...", file_name));

                    if let Some(bytes) = file_engine.read_file(file_name).await {
                        let client = reqwest::Client::new();
                        let part = reqwest::multipart::Part::bytes(bytes)
                            .file_name(file_name.clone())
                            .mime_str("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
                            .unwrap_or_else(|_| reqwest::multipart::Part::bytes(vec![]));

                        let form = reqwest::multipart::Form::new().part("file", part);

                        let res = client.post(config.api_url("/api/import"))
                            .header("Authorization", format!("Bearer {}", token.as_deref().unwrap_or("")))
                            .multipart(form)
                            .send()
                            .await;

                        match res {
                            Ok(resp) => {
                                if resp.status().is_success() {
                                    if let Ok(summary) = resp.json::<common::ImportSummary>().await {
                                        status.set(format!("✅ Éxito: {} creados, {} actualizados.", summary.created, summary.updated));
                                    } else {
                                        status.set("✅ Importación completada (error al leer resumen).".to_string());
                                    }
                                } else {
                                    let err_text = resp.text().await.unwrap_or_default();
                                    status.set(format!("❌ Error: {}", err_text));
                                }
                            }
                            Err(e) => status.set(format!("❌ Error de conexión: {}", e)),
                        }
                    } else {
                        status.set("❌ No se pudo leer el archivo.".to_string());
                    }
                    loading.set(false);
                }
            }
        });
    };

    rsx! {
        section { class: "space-y-6",
            header { class: "flex justify-between items-center",
                section {
                    h2 { class: "text-2xl font-black text-slate-900", "Importación de Datos" }
                    p { class: "text-sm text-slate-500 font-bold", "Carga masiva desde archivos Excel" }
                }
            }

            article { class: "p-12 text-center space-y-8",
                section { class: "w-24 h-24 bg-indigo-50 text-indigo-600 rounded-3xl flex items-center justify-center mx-auto text-4xl shadow-inner",
                    if *loading.read() { "⏳" } else { "📁" }
                }
                section {
                    h3 { class: "text-xl font-black text-slate-900", "Seleccionar Archivo Excel" }
                    p { class: "text-slate-500 text-sm max-w-sm mx-auto mt-2",
                        "El archivo debe seguir la estructura de la Plantilla Universal (IP en columna K)."
                    }
                }

                if *loading.read() {
                    div { class: "flex justify-center",
                        div { class: "animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600" }
                    }
                } else {
                    label {
                        class: "inline-block bg-slate-900 text-white px-10 py-5 rounded-[1.5rem] font-black uppercase tracking-widest text-xs shadow-xl hover:bg-indigo-600 transition-all cursor-pointer hover:-translate-y-1",
                        "Abrir Explorador de Archivos"
                        input {
                            r#type: "file",
                            accept: ".xlsx",
                            class: "hidden",
                            onchange: upload_file
                        }
                    }
                }

                if !status.read().is_empty() {
                    p { class: "text-sm font-bold text-indigo-600", "{status}" }
                }
            }
        }
    }
}
