use dioxus::prelude::*;
use crate::{AuthState, ServerConfig};
use crate::components::form_input::FormInput;

#[component]
pub fn UserManagement() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let server_config = use_context::<Signal<ServerConfig>>();
    let mut show_create_modal = use_signal(|| false);
    let mut new_username = use_signal(|| "".to_string());
    let mut new_password = use_signal(|| "".to_string());
    let mut new_role = use_signal(|| "viewer".to_string());
    let mut new_area = use_signal(|| "".to_string());
    let mut new_area_custom = use_signal(|| "".to_string());

    let mut editing_user = use_signal(|| None::<common::User>);
    let mut edit_role = use_signal(|| "viewer".to_string());
    let mut edit_area = use_signal(|| "".to_string());
    let mut edit_area_custom = use_signal(|| "".to_string());

    let mut deleting_user = use_signal(|| None::<common::User>);

    let mut error_msg = use_signal(|| None::<String>);

    let mut users_res = use_resource(move || {
        let token = auth.read().token.clone();
        let config = server_config.read().clone();
        async move {
            let client = reqwest::Client::new();
            let res = client.get(config.api_url("/api/users"))
                .header("Authorization", format!("Bearer {}", token.as_deref().unwrap_or("")))
                .send()
                .await;

            match res {
                Ok(resp) => resp.json::<Vec<common::User>>().await.unwrap_or_default(),
                Err(_) => Vec::new(),
            }
        }
    });

    let on_create_submit = move |_| {
        let user = new_username.read().to_string();
        let pwd = new_password.read().to_string();
        let r = new_role.read().to_string();
        let area_val = new_area.read().to_string();
        let area_custom = new_area_custom.read().to_string();
        let area_final = if area_val == "NUEVO" {
            if area_custom.trim().is_empty() { None } else { Some(area_custom.trim().to_string()) }
        } else if area_val.is_empty() {
            None
        } else {
            Some(area_val)
        };

        if user.is_empty() || pwd.is_empty() {
            error_msg.set(Some("El usuario y la contraseña no pueden estar vacíos".to_string()));
            return;
        }

        let config = server_config.read().clone();
        let token = auth.read().token.clone();
        spawn(async move {
            let client = reqwest::Client::new();
            let res = client.post(config.api_url("/register"))
                .header("Authorization", format!("Bearer {}", token.as_deref().unwrap_or("")))
                .json(&serde_json::json!({
                    "username": user,
                    "password": pwd,
                    "role": r,
                    "area": area_final
                }))
                .send()
                .await;

            match res {
                Ok(resp) if resp.status().is_success() => {
                    show_create_modal.set(false);
                    new_username.set("".to_string());
                    new_password.set("".to_string());
                    new_role.set("viewer".to_string());
                    new_area.set("".to_string());
                    new_area_custom.set("".to_string());
                    error_msg.set(None);
                    users_res.restart();
                }
                Ok(resp) => {
                    let text = resp.text().await.unwrap_or_default();
                    error_msg.set(Some(format!("Error al registrar: {}", text)));
                }
                Err(e) => {
                    error_msg.set(Some(format!("Error de red: {}", e)));
                }
            }
        });
    };

    let on_edit_submit = move |_| {
        let token = auth.read().token.clone();
        let user_to_edit = editing_user.read().clone();
        let r = edit_role.read().to_string();
        let area_val = edit_area.read().to_string();
        let area_custom = edit_area_custom.read().to_string();
        let area_final = if area_val == "NUEVO" {
            if area_custom.trim().is_empty() { None } else { Some(area_custom.trim().to_string()) }
        } else if area_val.is_empty() {
            None
        } else {
            Some(area_val)
        };

        if let Some(user) = user_to_edit {
            let config = server_config.read().clone();
            spawn(async move {
                let client = reqwest::Client::new();
                let res = client.put(config.api_url(&format!("/api/users/{}", user.id)))
                    .header("Authorization", format!("Bearer {}", token.as_deref().unwrap_or("")))
                    .json(&serde_json::json!({
                        "role": r,
                        "area": area_final
                    }))
                    .send()
                    .await;

                match res {
                    Ok(resp) if resp.status().is_success() => {
                        editing_user.set(None);
                        edit_area_custom.set("".to_string());
                        error_msg.set(None);
                        users_res.restart();
                    }
                    Ok(resp) => {
                        let text = resp.text().await.unwrap_or_default();
                        error_msg.set(Some(format!("Error al actualizar: {}", text)));
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Error de red: {}", e)));
                    }
                }
            });
        }
    };

    let on_delete_submit = move |_| {
        let token = auth.read().token.clone();
        let user_to_delete = deleting_user.read().clone();

        if let Some(user) = user_to_delete {
            if let Some(current_user) = auth.read().user.as_ref() {
                if current_user.id == user.id {
                    error_msg.set(Some("No puedes eliminar tu propio usuario administrador".to_string()));
                    return;
                }
            }

            let config = server_config.read().clone();
            spawn(async move {
                let client = reqwest::Client::new();
                let res = client.delete(config.api_url(&format!("/api/users/{}", user.id)))
                    .header("Authorization", format!("Bearer {}", token.as_deref().unwrap_or("")))
                    .send()
                    .await;

                match res {
                    Ok(resp) if resp.status().is_success() => {
                        deleting_user.set(None);
                        error_msg.set(None);
                        users_res.restart();
                    }
                    Ok(resp) => {
                        let text = resp.text().await.unwrap_or_default();
                        error_msg.set(Some(format!("Error al eliminar: {}", text)));
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("Error de red: {}", e)));
                    }
                }
            });
        }
    };

    rsx! {
        section { class: "space-y-6 relative",
            header { class: "flex justify-between items-center",
                section {
                    h2 { class: "text-2xl font-black text-slate-900", "Gestión de Usuarios" }
                    p { class: "text-sm text-slate-500 font-bold", "Control de acceso y roles del sistema" }
                }
                button {
                    class: "bg-indigo-600 hover:bg-indigo-700 text-white font-black text-xs uppercase tracking-widest px-5 py-3 rounded-[0.5rem] shadow-md transition-all flex items-center gap-2",
                    onclick: move |_| {
                        new_username.set("".to_string());
                        new_password.set("".to_string());
                        new_role.set("viewer".to_string());
                        new_area.set("".to_string());
                        new_area_custom.set("".to_string());
                        error_msg.set(None);
                        show_create_modal.set(true);
                    },
                    "➕ Nuevo Usuario"
                }
            }

            article { class: "bg-white rounded-[0.5rem] border border-slate-200 overflow-hidden shadow-sm",
                table { class: "w-full text-left border-collapse",
                    thead { class: "bg-slate-50 border-b border-slate-200",
                        tr {
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "ID" }
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "Nombre de Usuario" }
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "Rol Actual" }
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "Área" }
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest", "Fecha Registro" }
                            th { class: "px-6 py-4 text-[10px] font-black text-slate-400 uppercase tracking-widest text-right", "Acciones" }
                        }
                    }
                    tbody { class: "divide-y divide-slate-100",
                        if let Some(user_list) = users_res.read().as_ref() {
                            for user in user_list.iter() {
                                {
                                    let u = user.clone();
                                    rsx! {
                                        tr { class: "hover:bg-slate-50 transition-colors",
                                            td { class: "px-6 py-4 text-xs font-bold text-slate-400", "#{u.id}" }
                                            td { class: "px-6 py-4 text-xs font-black text-slate-900", "{u.username}" }
                                            td { class: "px-6 py-4",
                                                span { class: "bg-indigo-50 text-indigo-700 px-3 py-1 rounded-full text-[10px] font-black uppercase tracking-wider", "{u.role}" }
                                            }
                                            td { class: "px-6 py-4",
                                                if let Some(ref area) = u.area {
                                                    span { class: "bg-slate-100 text-slate-700 px-3 py-1 rounded-full text-[10px] font-black uppercase tracking-wider border border-slate-200", "{area}" }
                                                } else {
                                                    span { class: "text-slate-300 text-[10px] font-bold italic", "Sin área" }
                                                }
                                            }
                                            td { class: "px-6 py-4 text-xs font-bold text-slate-500",
                                                "{u.created_at.map(|f| f.format(\"%d/%m/%Y\").to_string()).unwrap_or_default()}"
                                            }
                                            td { class: "px-6 py-4 text-right space-x-2",
                                                {
                                                    let u_edit = u.clone();
                                                    rsx! {
                                                        button {
                                                            class: "p-2 bg-slate-100 rounded-lg text-slate-600 hover:bg-indigo-100 hover:text-indigo-600 transition-colors",
                                                            onclick: move |_| {
                                                                edit_role.set(u_edit.role.clone());
                                                                edit_area.set(u_edit.area.clone().unwrap_or_default());
                                                                edit_area_custom.set("".to_string());
                                                                editing_user.set(Some(u_edit.clone()));
                                                            },
                                                            "✏️"
                                                        }
                                                    }
                                                }
                                                {
                                                    let u_delete = u.clone();
                                                    rsx! {
                                                        button {
                                                            class: "p-2 bg-slate-100 rounded-lg text-slate-600 hover:bg-red-100 hover:text-red-600 transition-colors",
                                                            onclick: move |_| {
                                                                deleting_user.set(Some(u_delete.clone()));
                                                            },
                                                            "🗑️"
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
                }
            }

            if *show_create_modal.read() {
                section { class: "fixed inset-0 bg-slate-900/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
                    article { class: "bg-white rounded-[0.5rem] shadow-2xl border border-slate-200 max-w-md w-full overflow-hidden animate-in fade-in zoom-in duration-150",
                        header { class: "bg-slate-900 p-6 text-white flex justify-between items-center",
                            h3 { class: "text-lg font-black tracking-tight", "Registrar Nuevo Usuario" }
                            button {
                                class: "text-slate-400 hover:text-white transition-colors text-lg font-bold",
                                onclick: move |_| {
                                    show_create_modal.set(false);
                                    error_msg.set(None);
                                },
                                "✕"
                            }
                        }
                        section { class: "p-6 space-y-4",
                            if let Some(err) = error_msg.read().as_ref() {
                                section { class: "bg-red-50 text-red-600 p-3 rounded-[0.5rem] text-xs font-bold border border-red-100 text-center", "{err}" }
                            }
                            FormInput {
                                label: "Nombre de Usuario".to_string(),
                                value: new_username.read().clone(),
                                oninput: move |v| new_username.set(v)
                            }
                            FormInput {
                                label: "Contraseña".to_string(),
                                type_attr: "password".to_string(),
                                value: new_password.read().clone(),
                                oninput: move |v| new_password.set(v)
                            }
                            section { class: "space-y-2",
                                label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Rol" }
                                select {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-[0.5rem] px-5 py-4 font-bold outline-none focus:border-indigo-500 transition-all",
                                    value: "{new_role}",
                                    onchange: move |evt| new_role.set(evt.value()),
                                    option { value: "viewer", "Visualizador" }
                                    option { value: "editor", "Editor" }
                                    option { value: "admin", "Administrador" }
                                }
                            }
                            section { class: "space-y-2",
                                label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Área" }
                                select {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-[0.5rem] px-5 py-4 font-bold outline-none focus:border-indigo-500 transition-all",
                                    value: "{new_area}",
                                    onchange: move |evt| {
                                        new_area.set(evt.value());
                                        if evt.value() != "NUEVO" {
                                            new_area_custom.set("".to_string());
                                        }
                                    },
                                    option { value: "", "— Sin área —" }
                                    option { value: "SUB6", "SUB6" }
                                    option { value: "SUB5", "SUB5" }
                                    option { value: "TTE7", "TTE7" }
                                    option { value: "TTE8", "TTE8" }
                                    option { value: "TTE6", "TTE6" }
                                    option { value: "DIABLO", "DIABLO" }
                                    option { value: "NUEVO", "➕ Crear nuevo..." }
                                }
                                if *new_area.read() == "NUEVO" {
                                    input {
                                        class: "w-full bg-white border border-indigo-300 rounded-[0.5rem] px-5 py-3 font-bold outline-none focus:border-indigo-500 transition-all mt-2 text-sm",
                                        placeholder: "Nombre del nuevo área...",
                                        value: "{new_area_custom}",
                                        oninput: move |evt| new_area_custom.set(evt.value())
                                    }
                                }
                            }
                        }
                        footer { class: "bg-slate-50 px-6 py-4 flex justify-end gap-3 border-t border-slate-100",
                            button {
                                class: "px-5 py-2.5 rounded-[0.5rem] text-xs font-bold text-slate-500 hover:bg-slate-100 transition-colors",
                                onclick: move |_| {
                                    show_create_modal.set(false);
                                    error_msg.set(None);
                                },
                                "Cancelar"
                            }
                            button {
                                class: "px-5 py-2.5 rounded-[0.5rem] text-xs font-black text-white bg-indigo-600 hover:bg-indigo-700 shadow-md transition-colors",
                                onclick: on_create_submit,
                                "Registrar"
                            }
                        }
                    }
                }
            }

            if let Some(user_to_edit) = editing_user.read().clone() {
                section { class: "fixed inset-0 bg-slate-900/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
                    article { class: "bg-white rounded-[0.5rem] shadow-2xl border border-slate-200 max-w-md w-full overflow-hidden animate-in fade-in zoom-in duration-150",
                        header { class: "bg-slate-900 p-6 text-white flex justify-between items-center",
                            h3 { class: "text-lg font-black tracking-tight", "Editar Rol de Usuario" }
                            button {
                                class: "text-slate-400 hover:text-white transition-colors text-lg font-bold",
                                onclick: move |_| {
                                    editing_user.set(None);
                                    error_msg.set(None);
                                },
                                "✕"
                            }
                        }
                        section { class: "p-6 space-y-4",
                            if let Some(err) = error_msg.read().as_ref() {
                                section { class: "bg-red-50 text-red-600 p-3 rounded-[0.5rem] text-xs font-bold border border-red-100 text-center", "{err}" }
                            }
                            section { class: "space-y-1",
                                span { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Usuario" }
                                p { class: "text-sm font-black text-slate-800 px-1", "{user_to_edit.username}" }
                            }
                            section { class: "space-y-2",
                                label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Rol" }
                                select {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-[0.5rem] px-5 py-4 font-bold outline-none focus:border-indigo-500 transition-all",
                                    value: "{edit_role}",
                                    onchange: move |evt| edit_role.set(evt.value()),
                                    option { value: "viewer", "Visualizador" }
                                    option { value: "editor", "Editor" }
                                    option { value: "admin", "Administrador" }
                                }
                            }
                            section { class: "space-y-2",
                                label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Área" }
                                select {
                                    class: "w-full bg-slate-50 border border-slate-200 rounded-[0.5rem] px-5 py-4 font-bold outline-none focus:border-indigo-500 transition-all",
                                    value: "{edit_area}",
                                    onchange: move |evt| {
                                        edit_area.set(evt.value());
                                        if evt.value() != "NUEVO" {
                                            edit_area_custom.set("".to_string());
                                        }
                                    },
                                    option { value: "", "— Sin área —" }
                                    option { value: "SUB6", "SUB6" }
                                    option { value: "SUB5", "SUB5" }
                                    option { value: "TTE7", "TTE7" }
                                    option { value: "TTE8", "TTE8" }
                                    option { value: "TTE6", "TTE6" }
                                    option { value: "DIABLO", "DIABLO" }
                                    option { value: "NUEVO", "➕ Crear nuevo..." }
                                }
                                if *edit_area.read() == "NUEVO" {
                                    input {
                                        class: "w-full bg-white border border-indigo-300 rounded-[0.5rem] px-5 py-3 font-bold outline-none focus:border-indigo-500 transition-all mt-2 text-sm",
                                        placeholder: "Nombre del nuevo área...",
                                        value: "{edit_area_custom}",
                                        oninput: move |evt| edit_area_custom.set(evt.value())
                                    }
                                }
                            }
                        }
                        footer { class: "bg-slate-50 px-6 py-4 flex justify-end gap-3 border-t border-slate-100",
                            button {
                                class: "px-5 py-2.5 rounded-[0.5rem] text-xs font-bold text-slate-500 hover:bg-slate-100 transition-colors",
                                onclick: move |_| {
                                    editing_user.set(None);
                                    error_msg.set(None);
                                },
                                "Cancelar"
                            }
                            button {
                                class: "px-5 py-2.5 rounded-[0.5rem] text-xs font-black text-white bg-indigo-600 hover:bg-indigo-700 shadow-md transition-colors",
                                onclick: on_edit_submit,
                                "Guardar Cambios"
                            }
                        }
                    }
                }
            }

            if let Some(user_to_delete) = deleting_user.read().clone() {
                section { class: "fixed inset-0 bg-slate-900/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
                    article { class: "bg-white rounded-[0.5rem] shadow-2xl border border-slate-200 max-w-md w-full overflow-hidden animate-in fade-in zoom-in duration-150",
                        header { class: "bg-red-600 p-6 text-white flex justify-between items-center",
                            h3 { class: "text-lg font-black tracking-tight", "Confirmar Eliminación" }
                            button {
                                class: "text-red-200 hover:text-white transition-colors text-lg font-bold",
                                onclick: move |_| {
                                    deleting_user.set(None);
                                    error_msg.set(None);
                                },
                                "✕"
                            }
                        }
                        section { class: "p-6 space-y-4",
                            if let Some(err) = error_msg.read().as_ref() {
                                section { class: "bg-red-50 text-red-600 p-3 rounded-[0.5rem] text-xs font-bold border border-red-100 text-center", "{err}" }
                            }
                            p { class: "text-sm text-slate-600 font-bold",
                                "¿Estás seguro de que deseas eliminar permanentemente al usuario "
                                span { class: "text-slate-900 font-black", "\"{user_to_delete.username}\"" }
                                "?"
                            }
                            p { class: "text-xs text-red-500 font-medium", "Esta acción no se puede deshacer y revocará todo acceso a esta cuenta inmediatamente." }
                        }
                        footer { class: "bg-slate-50 px-6 py-4 flex justify-end gap-3 border-t border-slate-100",
                            button {
                                class: "px-5 py-2.5 rounded-[0.5rem] text-xs font-bold text-slate-500 hover:bg-slate-100 transition-colors",
                                onclick: move |_| {
                                    deleting_user.set(None);
                                    error_msg.set(None);
                                },
                                "Cancelar"
                            }
                            button {
                                class: "px-5 py-2.5 rounded-[0.5rem] text-xs font-black text-white bg-red-600 hover:bg-red-700 shadow-md transition-colors",
                                onclick: on_delete_submit,
                                "Eliminar"
                            }
                        }
                    }
                }
            }
        }
    }
}

