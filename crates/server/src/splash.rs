use native_windows_gui as nwg;
use tokio::sync::oneshot;
use std::sync::{Arc, Mutex};

pub fn show(ready_rx: oneshot::Receiver<Result<String, String>>) {
    nwg::init().expect("No se pudo inicializar NWG");
    nwg::Font::set_global_family("Segoe UI").ok();

    // Ventana principal del splash
    let mut window = Default::default();
    nwg::Window::builder()
        .size((420, 280))
        .center(true)
        .title("Equipos Industriales")
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .build(&mut window)
        .expect("No se pudo crear la ventana splash");

    // Imagen del logo
    let mut logo_frame = Default::default();
    nwg::ImageFrame::builder()
        .size((100, 100))
        .position((160, 20))
        .parent(&window)
        .build(&mut logo_frame)
        .ok();

    // Cargar ícono como bitmap
    let mut bitmap = Default::default();
    let icon_bytes = include_bytes!("../assets/icon.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.resize(100, 100, image::imageops::FilterType::Lanczos3).into_rgba8();
        if let Ok(bmp) = nwg::Bitmap::from_bin(&rgba.into_raw()) {
            bitmap = bmp;
            logo_frame.set_bitmap(Some(&bitmap));
        }
    }

    // Título
    let mut title_label = Default::default();
    nwg::Label::builder()
        .text("Equipos Industriales")
        .size((380, 35))
        .position((20, 130))
        .h_align(nwg::HTextAlign::Center)
        .parent(&window)
        .build(&mut title_label)
        .ok();

    let mut title_font = Default::default();
    nwg::Font::builder()
        .size(20)
        .weight(700)
        .family("Segoe UI")
        .build(&mut title_font)
        .ok();
    title_label.set_font(Some(&title_font));

    // Mensaje de estado
    let mut status_label = Default::default();
    nwg::Label::builder()
        .text("Iniciando servidor...")
        .size((380, 25))
        .position((20, 175))
        .h_align(nwg::HTextAlign::Center)
        .parent(&window)
        .build(&mut status_label)
        .ok();

    // Barra de progreso
    let mut progress = Default::default();
    nwg::ProgressBar::builder()
        .size((360, 20))
        .position((30, 210))
        .range(0..100)
        .parent(&window)
        .build(&mut progress)
        .ok();

    // Timer — DEBE tener parent
    let mut timer = Default::default();
    nwg::AnimationTimer::builder()
        .interval(std::time::Duration::from_millis(50))
        .parent(&window)
        .build(&mut timer)
        .ok();

    // Estado compartido entre el thread receptor y el event handler
    let progress_val = Arc::new(Mutex::new(0u32));
    let server_ready = Arc::new(Mutex::new(false));
    let server_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // Thread que espera la señal del servidor sin bloquear la UI
    let server_ready_clone = server_ready.clone();
    let server_error_clone = server_error.clone();
    std::thread::spawn(move || {
        // Usamos block_on en un runtime mínimo solo para await el oneshot
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            match ready_rx.await {
                Ok(Ok(_addr)) => {
                    *server_ready_clone.lock().unwrap() = true;
                }
                Ok(Err(e)) => {
                    *server_error_clone.lock().unwrap() = Some(e);
                }
                Err(_) => {
                    *server_error_clone.lock().unwrap() =
                        Some("Canal cerrado inesperadamente".to_string());
                }
            }
        });
    });

    // Event handler del timer
    let progress_clone = progress_val.clone();
    let server_ready_clone = server_ready.clone();
    let server_error_clone = server_error.clone();

    let handler = nwg::full_bind_event_handler(&window.handle, move |evt, _evt_data, _handle| {
        use nwg::Event as E;
        match evt {
            E::OnTimerTick => {
                // Error del servidor
                if server_error_clone.lock().unwrap().is_some() {
                    nwg::stop_thread_dispatch();
                    return;
                }

                // Servidor listo: completar barra y cerrar sin sleep bloqueante
                if *server_ready_clone.lock().unwrap() {
                    progress.set_pos(100);
                    nwg::stop_thread_dispatch();
                    return;
                }

                // Animar la barra hasta 90% mientras espera
                let mut val = progress_clone.lock().unwrap();
                if *val < 99 {
                    *val += 1;
                    progress.set_pos(*val);
                }
            }
            E::OnWindowClose => {
                nwg::stop_thread_dispatch();
            }
            _ => {}
        }
    });

    timer.start();
    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);
}
