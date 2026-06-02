use dioxus::prelude::*;
use common::User;
use crate::{AuthState, Route, ServerConfig};
use crate::components::form_input::FormInput;

#[component]
pub fn Login() -> Element {
    let mut auth = use_context::<Signal<AuthState>>();
    let mut server_config = use_context::<Signal<ServerConfig>>();
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let nav = use_navigator();

    let mut show_ip_config = use_signal(|| false);
    let mut server_ip = use_signal(|| server_config.read().host.clone());
    let mut connection_test = use_signal(|| None::<Result<String, String>>);
    let mut testing_connection = use_signal(|| false);

    let on_login = move |_| {
        let config = server_config.read().clone();
        spawn(async move {
            loading.set(true);
            error.set(None);
            let client = reqwest::Client::new();
            let login_url = config.api_url("/login");
            let res = client.post(&login_url)
                .json(&serde_json::json!({
                    "username": username.read().to_string(),
                    "password": password.read().to_string()
                }))
                .send()
                .await;

            match res {
                Ok(response) if response.status().is_success() => {
                    if let Ok(data) = response.json::<serde_json::Value>().await {
                        let token = data["token"].as_str().unwrap_or("").to_string();
                        let user = serde_json::from_value::<User>(data["user"].clone()).unwrap();
                        *auth.write() = AuthState { token: Some(token), user: Some(user) };
                        nav.push(Route::Home {});
                    }
                }
                _ => {
                    error.set(Some("Credenciales inválidas o servidor inaccesible".to_string()));
                }
            }
            loading.set(false);
        });
    };

    let on_test_connection = move |_| {
        let ip_to_test = server_ip.read().clone();
        spawn(async move {
            testing_connection.set(true);
            connection_test.set(None);
            
            let temp_config = ServerConfig { host: ip_to_test.clone() };
            let test_url = temp_config.api_url("/auth/status");
            
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap();
                
            let res = client.get(&test_url).send().await;
            
            match res {
                Ok(response) if response.status().is_success() => {
                    connection_test.set(Some(Ok("¡Conectado!".to_string())));
                    server_config.write().host = ip_to_test;
                }
                Ok(response) => {
                    connection_test.set(Some(Err(format!("Error: {}", response.status()))));
                }
                Err(err) => {
                    connection_test.set(Some(Err(format!("Fallo: {}", err))));
                }
            }
            testing_connection.set(false);
        });
    };

    rsx! {
        section { class: "min-h-screen flex items-center justify-center p-4",
            article { class: "relative w-full max-w-md bg-white rounded-[0.5rem] shadow-2xl border border-slate-200 overflow-hidden",
                button {
                    onclick: move |_| {
                        let val = *show_ip_config.read();
                        show_ip_config.set(!val);
                    },
                    class: "absolute top-4 right-4 p-2.5 text-slate-400 hover:text-white transition-colors bg-white/10 hover:bg-white/20 rounded-full z-10",
                    title: "Configurar Dirección IP del Servidor",
                    "⚙️"
                }
                header { class: "bg-slate-900 p-12 text-white text-center",
                    
                    h2 { class: "text-3xl font-black tracking-tighter", "Acceso" }
                    p { class: "text-slate-400 text-[10px] mt-2 uppercase tracking-[0.3em] font-bold", "Terminal de Control" }
                }
                if *show_ip_config.read() {
                    section { class: "p-8 bg-slate-50 border-b border-slate-200 space-y-4 transition-all duration-300",
                        h3 { class: "text-xs font-black text-slate-500 uppercase tracking-widest", "Dirección del Servidor" }
                        section { class: "space-y-2",
                            label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "IP / Nombre de Host" }
                            input {
                                class: "w-full bg-white border border-slate-200 rounded-[0.5rem] px-4 py-3 font-mono font-bold text-sm outline-none",
                                placeholder: "ej. 192.168.1.100",
                                value: "{server_ip}",
                                oninput: move |evt| server_ip.set(evt.value())
                            }
                        }
                        section { class: "flex gap-3 items-center",
                            button {
                                onclick: on_test_connection,
                                disabled: *testing_connection.read(),
                                class: "bg-slate-900 text-white px-5 py-3 rounded-[0.5rem] font-black uppercase tracking-widest text-[9px] hover:bg-slate-800 transition-all disabled:opacity-50",
                                if *testing_connection.read() { "Probando..." } else { "Guardar y Probar" }
                            }
                            if let Some(res) = connection_test.read().as_ref() {
                                match res {
                                    Ok(msg) => rsx! { span { class: "text-[10px] font-bold text-green-600 flex items-center gap-1", "✅ {msg}" } },
                                    Err(err) => rsx! { span { class: "text-[9px] font-bold text-rose-500 flex items-center gap-1 leading-tight max-w-[180px]", "❌ {err}" } },
                                }
                            }
                        }
                    }
                }
                section { class: "p-12 space-y-8",
                    if let Some(err) = error.read().as_ref() {
                        section { class: "bg-red-50 text-red-600 p-4 rounded-[0.5rem] text-xs font-bold border border-red-100 text-center", "{err}" }
                    }
                    section { class: "space-y-4",
                        FormInput { label: "Usuario".to_string(), value: username.read().clone(), oninput: move |v| username.set(v) }
                        FormInput { label: "Contraseña".to_string(), type_attr: "password".to_string(), value: password.read().clone(), oninput: move |v| password.set(v) }
                    }
                    button {
                        onclick: on_login,
                        disabled: *loading.read(),
                        class: "w-full bg-indigo-600 text-white py-5 rounded-[0.5rem] font-black uppercase tracking-[0.2em] text-xs shadow-xl hover:bg-indigo-700 transition-all",
                        if *loading.read() { "Verificando..." } else { "Entrar al Sistema" }
                    }
                }
            }
        }
    }
}

