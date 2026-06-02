#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dioxus::prelude::*;
use dioxus_desktop::{Config, WindowBuilder, LogicalSize};
use common::User;
use serde::{Deserialize, Serialize};
use dioxus_sdk::storage::{use_synced_storage, LocalStorage};

mod layout;
mod pages;
mod components;

use layout::MainLayout;
use pages::{
    home::Home,
    login::Login,
    setup::Setup,
    equipo_list::EquipoList,
    equipo_detail::EquipoDetail,
    equipo_new::EquipoNew,
    equipo_edit::EquipoEdit,
    import::Import,
    audit::AuditLogGlobal,
    users::UserManagement,
};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[layout(MainLayout)]
    #[route("/")]
    Home {},
    #[route("/equipos")]
    EquipoList {},
    #[route("/detail/:id")]
    EquipoDetail { id: i32 },
    #[route("/edit/:id")]
    EquipoEdit { id: i32 },
    #[route("/new")]
    EquipoNew {},
    #[route("/import")]
    Import {},
    #[route("/audit")]
    AuditLogGlobal {},
    #[route("/users")]
    UserManagement {},
    #[route("/login")]
    Login {},
    #[route("/setup")]
    Setup {},
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthState {
    pub token: Option<String>,
    pub user: Option<User>,
}

#[derive(Deserialize)]
pub struct AuthStatus {
    pub setup_required: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub host: String,
}

impl ServerConfig {
    pub fn api_url(&self, path: &str) -> String {
        let clean_path = path.trim_start_matches('/');

        let host = if self.host.starts_with("http://") || self.host.starts_with("https://") {
            if self.host.contains(':') {
                let without_scheme = self.host
                    .trim_start_matches("https://")
                    .trim_start_matches("http://");
                if without_scheme.contains(':') {
                    self.host.clone()
                } else {
                    format!("{}:3000", self.host.trim_end_matches('/'))
                }
            } else {
                format!("{}:3000", self.host.trim_end_matches('/'))
            }
        } else {
            if self.host.contains(':') {
                format!("http://{}", self.host)
            } else {
                format!("http://{}:3000", self.host)
            }
        };

        format!("{}/{}", host.trim_end_matches('/'), clean_path)
    }
}

fn load_icon() -> dioxus_desktop::tao::window::Icon {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes)
        .expect("No se pudo cargar ícono")
        .into_rgba8();
    let (w, h) = img.dimensions();
    dioxus_desktop::tao::window::Icon::from_rgba(img.into_raw(), w, h)
        .expect("No se pudo crear ícono")
}

// =========================================================================
// FUNCIÓN MAIN MODIFICADA
// =========================================================================
fn main() {
    // 1. Obtenemos la ruta base segura en AppData\Local\EquiposIndustriales
    let mut app_base_dir = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir());
    app_base_dir.push("EquiposIndustriales");

    // 2. Configurar el almacenamiento de dioxus-sdk en la ruta segura
    #[cfg(not(target_arch = "wasm32"))]
    {
        let storage_dir = app_base_dir.join("data/viewer_storage");
        std::fs::create_dir_all(&storage_dir).ok();
        dioxus_sdk::storage::set_directory(storage_dir);
    }

    // 3. Crear la configuración del entorno para WebView2 (Caché y datos locales)
    let webview_data_dir = app_base_dir.join("webview_cache");

    let config = Config::new()
        .with_disable_context_menu(true)
        .with_menu(None)
        .with_data_directory(webview_data_dir) // <-- Esto evita el crash de acceso denegado de WebView2
        .with_window(
            WindowBuilder::new()
                .with_title("Equipos Industriales")
                .with_window_icon(Some(load_icon()))
                .with_inner_size(LogicalSize::new(1280.0, 800.0))
        );

    dioxus_desktop::launch::launch(App, vec![], config);
}

#[component]
fn App() -> Element {
    let mut storage = use_synced_storage::<LocalStorage, String>(
        "viewer_server_host".to_string(),
        || "localhost".to_string()
    );

    use_context_provider(|| Signal::new(AuthState { token: None, user: None }));
    let server_config = use_context_provider(|| Signal::new(ServerConfig { host: storage.read().clone() }));

    use_effect(move || {
        let host = server_config.read().host.clone();
        if *storage.read() != host {
            *storage.write() = host;
        }
    });

    rsx! {
        style { {include_str!("../index.css")} }
        Router::<Route> {}
    }
}