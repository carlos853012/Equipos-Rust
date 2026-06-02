use native_windows_gui as nwg;
use winreg::RegKey;
use winreg::enums::*;
use tokio::sync::oneshot;

fn is_autostart_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_READ,
    );
    match run {
        Ok(key) => key.get_value::<String, _>("EquiposIndustrialesServer").is_ok(),
        Err(_) => false,
    }
}

fn set_autostart(enabled: bool) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(run) = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_SET_VALUE,
    ) {
        if enabled {
            if let Ok(exe) = std::env::current_exe() {
                let _ = run.set_value("EquiposIndustrialesServer", &exe.to_string_lossy().to_string());
            }
        } else {
            let _ = run.delete_value("EquiposIndustrialesServer");
        }
    }
}

pub fn run(shutdown_tx: oneshot::Sender<()>) {
    nwg::init().expect("Error al inicializar NWG para el Tray");

    let mut window = Default::default();
    nwg::Window::builder()
        .flags(nwg::WindowFlags::WINDOW)
        .build(&mut window)
        .expect("Error al crear ventana contenedora del Tray");

    let mut icon = nwg::Icon::default();
    let icon_bytes = include_bytes!("../assets/icon.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.resize(32, 32, image::imageops::FilterType::Lanczos3).into_rgba8();
        if let Ok(loaded) = nwg::Icon::from_bin(&rgba.into_raw()) {
            icon = loaded;
        }
    }

    let mut menu = nwg::Menu::default();
    nwg::Menu::builder()
        .popup(true)
        .parent(&window)
        .build(&mut menu)
        .expect("Error al crear menú del Tray");

    let mut item_status = nwg::MenuItem::default();
    nwg::MenuItem::builder()
        .text("Servidor Activo")
        .disabled(true)
        .parent(&menu)
        .build(&mut item_status)
        .ok();

    let mut item_autostart = nwg::MenuItem::default();
    let autostart_checked = is_autostart_enabled();
    nwg::MenuItem::builder()
        .text("Ejecutar al inicio")
        .check(autostart_checked)
        .parent(&menu)
        .build(&mut item_autostart)
        .expect("Error al crear ítem de autostart");

    let mut item_separator = nwg::MenuSeparator::default();
    nwg::MenuSeparator::builder()
        .parent(&menu)
        .build(&mut item_separator)
        .ok();

    let mut item_exit = nwg::MenuItem::default();
    nwg::MenuItem::builder()
        .text("Salir del Servidor")
        .parent(&menu)
        .build(&mut item_exit)
        .expect("Error al crear botón Salir");

    let mut tray = nwg::TrayNotification::default();
    nwg::TrayNotification::builder()
        .parent(&window)
        .icon(Some(&icon))
        .build(&mut tray)
        .expect("Error al crear Tray Notification");

    tray.set_tip("Servidor de Equipos Industriales");

    let autostart_enabled = std::cell::Cell::new(autostart_checked);
    let shutdown_tx = std::cell::RefCell::new(Some(shutdown_tx));

    let handler = nwg::full_bind_event_handler(&window.handle, move |evt, _evt_data, handle| {
        use nwg::Event as E;
        match evt {
            E::OnWindowClose => {
                if let Some(tx) = shutdown_tx.borrow_mut().take() {
                    let _ = tx.send(());
                }
                nwg::stop_thread_dispatch();
            }
            E::OnContextMenu if handle == tray.handle => {
                let (x, y) = nwg::GlobalCursor::position();
                menu.popup(x, y);
            }
            E::OnMenuItemSelected if handle == item_autostart.handle => {
                let currently = autostart_enabled.get();
                set_autostart(!currently);
                autostart_enabled.set(!currently);
                item_autostart.set_checked(!currently);
            }
            E::OnMenuItemSelected if handle == item_exit.handle => {
                if let Some(tx) = shutdown_tx.borrow_mut().take() {
                    let _ = tx.send(());
                }
                nwg::stop_thread_dispatch();
            }
            _ => {}
        }
    });

    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);
}
