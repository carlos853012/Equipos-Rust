use dioxus::prelude::*;

#[component]
pub fn FormInput(
    label: String,
    value: String,
    oninput: EventHandler<String>,
    #[props(default = "text".to_string())]
    type_attr: String,
) -> Element {
    rsx! {
        section { class: "space-y-2",
            label { class: "text-[10px] font-black text-slate-400 uppercase tracking-widest ml-1", "{label}" }
            input {
                r#type: "{type_attr}",
                class: "rounded-[0.5rem] w-full bg-slate-50 border border-slate-200 rounded-2xl px-5 py-4 font-bold outline-none focus:border-indigo-500 transition-all",
                value: "{value}",
                oninput: move |evt| oninput.call(evt.value())
            }
        }
    }
}
