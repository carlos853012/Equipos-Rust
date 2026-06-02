use dioxus::prelude::*;

#[component]
pub fn InfoGroup(label: String, value: String) -> Element {
    rsx! {
        section { class: "space-y-2 group",
            p { class: "text-[9px] font-black text-slate-400 uppercase tracking-[0.3em]", "{label}" }
            p { class: "text-xl font-black text-slate-900 tracking-tight", "{value}" }
        }
    }
}
