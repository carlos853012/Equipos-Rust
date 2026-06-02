use postgresql_embedded::{PostgreSQL, Settings};
use anyhow::{Result, Context};
use std::path::PathBuf;

pub struct DbManager {
    pg: PostgreSQL,
    reused: bool,
    data_dir: PathBuf,
}

impl DbManager {
    pub async fn new(data_dir: PathBuf) -> Result<Self> {
        let mut settings = Settings::default();
        settings.port = 5432;
        settings.data_dir = data_dir.clone();
        settings.temporary = false; // Desactivar la recreación/borrado temporal
        
        settings.username = "postgres".to_string();
        settings.password = "postgres".to_string();

        let pg = PostgreSQL::new(settings);
        Ok(Self { pg, reused: false, data_dir })
    }

    pub async fn start(&mut self) -> Result<()> {
        // Verificar si ya hay una base de datos escuchando en el puerto 5432
        let is_running = std::net::TcpStream::connect("127.0.0.1:5432").is_ok();
        if is_running {
            println!("[DB] PostgreSQL ya está ejecutándose en el puerto 5432. Reutilizando instancia existente...");
            self.reused = true;
            return Ok(());
        }

        // Si no está corriendo pero existe un archivo postmaster.pid obsoleto, lo limpiamos para evitar fallos al arrancar
        let pid_file = self.data_dir.join("postmaster.pid");
        if pid_file.exists() {
            println!("[DB] Detectado archivo postmaster.pid obsoleto. Eliminándolo para permitir el arranque...");
            if let Err(e) = std::fs::remove_file(&pid_file) {
                println!("[DB] Advertencia: no se pudo eliminar postmaster.pid obsoleto: {}", e);
            }
        }

        println!("[DB] Inicializando/Iniciando PostgreSQL embebido...");
        
        self.pg.setup().await
            .context("Error al configurar PostgreSQL")?;
        
        self.pg.start().await
            .context("Error al iniciar PostgreSQL")?;

        println!("[DB] PostgreSQL listo en el puerto 5432");

        if !self.pg.database_exists("equipos_redes").await? {
            println!("[DB] Creando base de datos 'equipos_redes'...");
            self.pg.create_database("equipos_redes").await
                .context("Error al crear la base de datos")?;
        }

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if self.reused {
            println!("[DB] Instancia de PostgreSQL externa o reutilizada. No se detiene en este proceso.");
            return Ok(());
        }
        println!("[DB] Deteniendo PostgreSQL...");
        self.pg.stop().await.context("Error al detener PostgreSQL")?;
        Ok(())
    }
}

