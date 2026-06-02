use dioxus::prelude::*;
use common::Equipo;
use crate::components::form_input::FormInput;

#[component]
pub fn EquipoForm(on_submit: EventHandler<Equipo>, initial_data: Equipo) -> Element {
    let mut form_state = use_signal(|| initial_data.clone());
    rsx! {
        section { class: "space-y-8 pb-20",
            section { class: "bg-white p-10 rounded-[0.5rem] border border-slate-200/60 shadow-xl",
                h3 { class: "text-[10px] font-black text-indigo-500 uppercase tracking-[0.3em] mb-8", "Sección 1: Identidad del Recurso" }
                section { class: "grid grid-cols-1 md:grid-cols-2 gap-8",
                    FormInput { label: "Dirección IP *".to_string(), value: form_state.read().ip_address.clone(), oninput: move |v| form_state.write().ip_address = v }
                    FormInput { label: "Nombre PC *".to_string(), value: form_state.read().nombre_pc.clone().unwrap_or_default(), oninput: move |v| form_state.write().nombre_pc = Some(v) }
                    FormInput { label: "Grupo / Planta *".to_string(), value: form_state.read().grupo.clone().unwrap_or_default(), oninput: move |v| form_state.write().grupo = Some(v) }
                    FormInput { label: "Área Operacional".to_string(), value: form_state.read().area.clone().unwrap_or_default(), oninput: move |v| form_state.write().area = Some(v) }
                }
            }
            section { class: "bg-white p-10 rounded-[0.5rem] border border-slate-200/60 shadow-xl",
                h3 { class: "text-[10px] font-black text-indigo-500 uppercase tracking-[0.3em] mb-8", "Sección 2: Especificaciones Técnicas" }
                section { class: "grid grid-cols-1 md:grid-cols-2 gap-8",
                    FormInput { label: "Tipo de Dispositivo".to_string(), value: form_state.read().tipo_dispositivo.clone().unwrap_or_default(), oninput: move |v| form_state.write().tipo_dispositivo = Some(v) }
                    FormInput { label: "Variante / Modelo".to_string(), value: form_state.read().tipo.clone().unwrap_or_default(), oninput: move |v| form_state.write().tipo = Some(v) }
                    FormInput { label: "Sistema Operativo".to_string(), value: form_state.read().sistema_operativo.clone().unwrap_or_default(), oninput: move |v| form_state.write().sistema_operativo = Some(v) }
                    FormInput { label: "Ubicación Funcional".to_string(), value: form_state.read().ubicacion.clone().unwrap_or_default(), oninput: move |v| form_state.write().ubicacion = Some(v) }
                    section { class: "md:col-span-2",
                        FormInput { label: "Ubicación Técnica (Física)".to_string(), value: form_state.read().ubicacion_tecnica.clone().unwrap_or_default(), oninput: move |v| form_state.write().ubicacion_tecnica = Some(v) }
                    }
                }
            }
            section { class: "bg-white p-10 rounded-[0.5rem] border border-slate-200/60 shadow-xl border-l-8 border-l-amber-500",
                h3 { class: "text-[10px] font-black text-amber-600 uppercase tracking-[0.3em] mb-8", "Sección 3: Seguridad y Acceso Remoto" }
                section { class: "grid grid-cols-1 md:grid-cols-2 gap-8",
                    FormInput { label: "Usuario Windows".to_string(), value: form_state.read().usuario_windows.clone().unwrap_or_default(), oninput: move |v| form_state.write().usuario_windows = Some(v) }
                    FormInput { label: "Clave Windows".to_string(), type_attr: "password".to_string(), value: form_state.read().clave_windows.clone().unwrap_or_default(), oninput: move |v| form_state.write().clave_windows = Some(v) }
                    section { class: "md:col-span-2",
                        FormInput { label: "Clave VNC".to_string(), type_attr: "password".to_string(), value: form_state.read().clave_vnc.clone().unwrap_or_default(), oninput: move |v| form_state.write().clave_vnc = Some(v) }
                    }
                }
            }
            section { class: "bg-white p-10 rounded-[0.5rem] border border-slate-200/60 shadow-xl",
                h3 { class: "text-[10px] font-black text-indigo-500 uppercase tracking-[0.3em] mb-8", "Sección 4: Documentación Adicional" }
                section { class: "space-y-8",
                    section { class: "space-y-2",
                        label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Descripción del Recurso" }
                        textarea {
                            class: "w-full bg-slate-50  rounded-[0.5rem] border border-slate-200 px-5 py-4 font-bold outline-none focus:border-indigo-500 transition-all min-h-[120px]",
                            value: "{form_state.read().descripcion.clone().unwrap_or_default()}",
                            oninput: move |evt| form_state.write().descripcion = Some(evt.value())
                        }
                    }
                    section { class: "space-y-2",
                        label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "Observaciones Técnicas" }
                        textarea {
                            class: "w-full bg-slate-50 rounded-[0.5rem] border border-slate-200 px-5 py-4 font-bold outline-none focus:border-indigo-500 transition-all min-h-[120px]",
                            value: "{form_state.read().observaciones.clone().unwrap_or_default()}",
                            oninput: move |evt| form_state.write().observaciones = Some(evt.value())
                        }
                    }
                }
            }
            footer { class: "flex justify-end pt-4",
                button {
                    onclick: move |_| on_submit.call(form_state.read().clone()),
                    class: "bg-indigo-600 text-white px-12 py-5 rounded-[0.5rem] font-black uppercase tracking-[0.2em] text-xs shadow-2xl hover:bg-indigo-700 transition-all hover:-translate-y-1",
                    "Guardar Nodo Industrial"
                }
            }
        }
    }
}
