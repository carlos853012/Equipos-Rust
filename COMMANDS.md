# Comandos Importantes — Equipos-Rust

## Compilación y desarrollo

```powershell
# Compilar todo el workspace
cargo build

# Compilar en release
cargo build --release

# Compilar solo un crate
cargo build -p server
cargo build -p viewer

# Verificar sin generar binarios (más rápido)
cargo check -p server

# Ejecutar server (con terminal visible en debug)
cargo run -p server

# Ejecutar viewer
cargo run -p viewer
```

## MSI / Instaladores

```powershell
# Server MSI (requiere WiX Toolset v3)
cargo wix -p server --nocapture

# Viewer MSI
cargo wix -p viewer --nocapture

# Limpiar artefactos WiX anteriores
Remove-Item -Recurse -Force target\wix -ErrorAction SilentlyContinue
```

## PostgreSQL embebido (solo server)

El server gestiona PostgreSQL automáticamente. Para depuración manual:

```powershell
# Ruta de los binarios de PostgreSQL usados por el MSI
C:\Users\carlos\.theseus\postgresql\18.3.0\bin\

# Ruta de datos en desarrollo (relativa al dir de trabajo)
crates\server\data\pgdata\

# Ruta de datos en producción (MSI)
%LOCALAPPDATA%\EquiposIndustriales\data\pgdata\

# Conectarse a la base (mientras el server corre)
& "C:\Users\carlos\.theseus\postgresql\18.3.0\bin\psql.exe" -h localhost -U postgres -d equipos_redes

# Limpiar datos corruptos (detener server primero)
Remove-Item -Recurse -Force "%LOCALAPPDATA%\EquiposIndustriales\data\pgdata"
```

## Escaneo de red (ICMP)

El server requiere permisos de administrador para ICMP en algunas
configuraciones de Windows:

```powershell
# Habilitar ICMP en firewall (si hay bloqueos)
New-NetFirewallRule -DisplayName "ICMP Allow" -Protocol ICMPv4 -IcmpType 8 -Enabled True
```

## Autostart (registro de Windows)

El server gestiona esta entrada automáticamente desde el menú del tray:

```powershell
# Verificar entrada actual
Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "EquiposIndustrialesServer"

# Eliminar manualmente
Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "EquiposIndustrialesServer"

# Agregar manualmente
Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "EquiposIndustrialesServer" -Value "C:\Program Files\EquiposIndustriales\bin\server.exe"
```

## Directorios importantes

```
# Código fuente
C:\Users\carlos\Desktop\Equipos-Rust\

# Datos de PostgreSQL (desarrollo)
C:\Users\carlos\Desktop\Equipos-Rust\crates\server\data\pgdata\

# Datos de PostgreSQL (producción)
%LOCALAPPDATA%\EquiposIndustriales\data\pgdata\

# Configuración del viewer
%LOCALAPPDATA%\EquiposIndustriales\viewer_storage\

# Assets del server
C:\Users\carlos\Desktop\Equipos-Rust\crates\server\assets\icon.png
C:\Users\carlos\Desktop\Equipos-Rust\crates\server\assets\icon.ico

# CSS compilado del viewer
C:\Users\carlos\Desktop\Equipos-Rust\crates\viewer\index.css

# MSIs generados
C:\Users\carlos\Desktop\Equipos-Rust\target\wix\

# PostgreSQL embebido (binarios descargados por cargo)
C:\Users\carlos\.theseus\postgresql\18.3.0\bin\
```

## Variables de entorno

```powershell
# Configurar secreto JWT
$env:JWT_SECRET="mi-secreto-personalizado"

# URL de base de datos (opcional, el server usa la embebida por defecto)
$env:DATABASE_URL="postgres://postgres:postgres@localhost:5432/equipos_redes"
```

## Solución de problemas

```powershell
# Error "Permission denied" al iniciar PostgreSQL en MSI
# -> Migrar datos de %ProgramFiles% a %LOCALAPPDATA%:
Move-Item "$env:ProgramFiles\EquiposIndustriales\bin\data\pgdata" "$env:LOCALAPPDATA\EquiposIndustriales\data\pgdata"

# Error "postmaster.pid" archivo stale
# -> Detener server, borrar el archivo:
Remove-Item "$env:LOCALAPPDATA\EquiposIndustriales\data\pgdata\postmaster.pid" -ErrorAction SilentlyContinue

# El server no arranca por puerto ocupado
netstat -ano | findstr :3000
# PID del proceso que ocupa el puerto
taskkill /PID <PID> /F

# Viewer no se conecta al server
# -> Verificar que el server esté corriendo
# -> Revisar la IP configurada en el engranaje de Login
# -> Firewall: permitir puerto 3000
New-NetFirewallRule -DisplayName "Equipos Server" -Direction Inbound -Protocol TCP -LocalPort 3000 -Action Allow

# Reconstruir el viewer con CSS actualizado
# (el CSS está inlinado en el binario vía include_str!)
cargo build -p viewer
```

## Dependencias del sistema

- **WiX Toolset v3** — para `cargo wix`
  - Instalar desde: https://wixtoolset.org/docs/wix3/
  - Verificar: `candle.exe -?`
- **WebView2 Runtime** — para el viewer (Dioxus Desktop)
  - Incluido en Windows 11 / Windows 10 (actualizaciones recientes)
  - Descargar: https://developer.microsoft.com/en-us/microsoft-edge/webview2/
