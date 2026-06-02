fn main() {
    // Solo en Windows
    #[cfg(target_os = "windows")]
    {
        use std::path::Path;

        let mut res = winresource::WindowsResource::new();

        if Path::new("app.manifest").exists() {
            res.set_manifest_file("app.manifest");
        } else {
            println!("cargo:warning=app.manifest no encontrado; se omite el manifiesto");
        }

        if Path::new("assets/icon.ico").exists() {
            res.set_icon("assets/icon.ico");
        } else {
            println!("cargo:warning=assets/icon.ico no encontrado; se omite el ícono");
        }

        if let Err(e) = res.compile() {
            panic!("Error al compilar recursos de Windows: {:?}", e);
        }
    }
}