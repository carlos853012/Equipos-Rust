# Equipos-Rust — Gestión de Equipos Industriales

Sistema cliente-servidor para la gestión, monitoreo e inventario de equipos
industriales en redes de minería subterránea. Desarrollado completamente en Rust.

## Arquitectura

```
Equipos-Rust/
├── crates/
│   ├── common/         -- Tipos compartidos (Equipo, AuditLog, User)
│   ├── server/         -- Backend HTTP (Axum) + PostgreSQL embebido
│   └── viewer/         -- Cliente desktop (Dioxus Desktop / WebView2)
```

### common (`crates/common`)
Define los modelos de datos que comparten server y viewer:
- `Equipo` — Equipo industrial con IP, nombre, SO, área, credenciales, etc.
- `AuditLog` — Registro de cambios con diff JSON
- `User` — Usuario del sistema con rol (viewer/editor/admin)
- `ImportSummary` — Resumen de importación Excel

### server (`crates/server`)
Backend HTTP en puerto `3000` con:
- PostgreSQL embebido (autogestionado, sin instalación externa)
- API RESTful con Axum
- Autenticación JWT + contraseñas con Argon2id
- Escaneo de red por ICMP (ping concurrente)
- Importación de Excel (.xlsx) con upsert
- Logging de auditoría con diff JSON
- *Windows:* Splash screen de inicio + ícono en bandeja del sistema con
  menú contextual (autostart en registro de Windows, salir)

### viewer (`crates/viewer`)
Cliente desktop con WebView2 (Dioxus Desktop):
- 10 pantallas: Dashboard, Login, Setup, CRUD de equipos, Importación,
  Auditoría global, Usuarios (solo admin)
- Roles de acceso: viewer (solo lectura), editor (CRUD), admin (todo)
- Almacenamiento local del host del servidor
- CSS utilitario inline (sin dependencia de internet, sin CDN)

## Requisitos

- **Rust** 1.75+ (MSVC toolchain recomendado: `stable-msvc`)
- **WebView2** (incluido en Windows 10/11, descargable para Windows 7/8)
- **Windows 10/11** (64-bit) — el server usa `native-windows-gui` y
  `winreg`; no soporta Linux/macOS

## Instalación

### Desde MSI (producción)

1. Descargar los MSIs de la última release
2. Instalar primero **server** (`EquiposIndustrialesServer.msi`)
3. Instalar **viewer** (`EquiposIndustrialesViewer.msi`)
4. Ejecutar el server (inicia minimizado en la bandeja del sistema)
5. Abrir el viewer — configura `localhost` como host del servidor
6. Crear el primer usuario administrador en la pantalla de Setup

### Desde código fuente (desarrollo)

```powershell
git clone <repo>
cd Equipos-Rust

# Compilar todo
cargo build

# Ejecutar server (una terminal aparte)
cargo run -p server

# Ejecutar viewer (otra terminal)
cargo run -p viewer
```

> El viewer asume `localhost:3000` por defecto.

## Configuración

### Server

| Variable          | Default                                   | Descripción                     |
|-------------------|-------------------------------------------|---------------------------------|
| `DATABASE_URL`    | `postgres://postgres:postgres@localhost:5432/equipos_redes` | Conexión PostgreSQL |
| `JWT_SECRET`      | `your-secret-key-change-in-production`    | Secreto para firmar JWTs        |

Archivo `.env` en `crates/server/`:

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/equipos_redes
JWT_SECRET=mi-secreto-personalizado
```

### Viewer

- El host del servidor se configura desde la misma interfaz (engranaje en
  la pantalla de Login)
- Se persiste en `%LOCALAPPDATA%\EquiposIndustriales\viewer_storage`

## Uso

### Iniciar server

```powershell
cargo run -p server
```

En producción se instala como aplicación de escritorio (sin terminal).
Aparece un ícono en la bandeja del sistema:
- **Clic derecho** → menú contextual en la posición del cursor
- **Autostart** → toggle visual (✓) que agrega/remueve `HKCU\...\Run`
- **Salir del Servidor** → envía señal de apagado graceful a Axum

### Iniciar viewer

```powershell
cargo run -p viewer
```

Ventana de 1280×800. Sin menú contextual (deshabilitado).

## API REST (server)

| Método | Ruta                    | Auth     | Descripción                                    |
|--------|-------------------------|----------|-------------------------------------------------|
| GET    | `/`                     | No       | Health check                                   |
| GET    | `/auth/status`          | No       | ¿Hay usuarios registrados?                     |
| POST   | `/login`                | No       | Iniciar sesión → JWT                           |
| POST   | `/register`             | No       | Solo primer admin (403 si ya hay usuarios) ⚠️  |
| GET    | `/api/equipos`          | JWT      | Listar equipos (filtro por ?q=)                |
| POST   | `/api/equipos`          | JWT      | Crear equipo                                   |
| GET    | `/api/equipos/:id`      | JWT      | Detalle equipo                                 |
| PUT    | `/api/equipos/:id`      | JWT      | Actualizar equipo                              |
| DELETE | `/api/equipos/:id`      | JWT      | Eliminar equipo                                |
| POST   | `/api/import`           | JWT      | Importar Excel (.xlsx)                         |
| GET    | `/api/scan`             | JWT      | Escanear red (ping)                            |
| GET    | `/api/audit`            | JWT      | Auditoría global                               |
| GET    | `/api/audit/:ip`        | JWT      | Auditoría por equipo                           |
| GET    | `/api/users`            | JWT+Admin| Listar usuarios                                |
| POST   | `/api/users`            | JWT+Admin| Crear usuario (viewer/editor/admin)            |
| PUT    | `/api/users/:id`        | JWT+Admin| Cambiar rol                                    |
| DELETE | `/api/users/:id`        | JWT+Admin| Eliminar usuario                               |

## Construir MSIs

Requiere **WiX Toolset v3** (cargo-wix lo usa internamente):

```powershell
# Server MSI
cargo wix -p server --nocapture

# Viewer MSI
cargo wix -p viewer --nocapture
```

Los MSIs se generan en `target/wix/`.

## Datos en disco

```
%LOCALAPPDATA%\EquiposIndustriales\
├── data\
│   ├── pgdata\           -- Datos de PostgreSQL
│   └── .crypto_key       -- Llave AES-256-GCM (auto-generada)
└── viewer_storage\       -- Configuración del viewer
```

## Seguridad

### Controles implementados

| Control | Estado | Detalle |
|---------|--------|---------|
| Contraseñas con Argon2id | ✅ | Hash de contraseñas de usuarios |
| Cifrado AES-256-GCM | ✅ | `clave_windows` y `clave_vnc` cifrados en DB, descifrados en API |
| Redacción en auditoría | ✅ | Credenciales se muestran como `[CIFRADO]` en logs de auditoría |
| Bloqueo de `POST /register` | ✅ | Solo permite crear el primer admin (403 si ya hay usuarios) |
| Roles y middleware | ✅ | Admin requerido para `GET/POST/PUT/DELETE /api/users/*` |
| Autenticación JWT | ✅ | Expira a las 24h |
| Llave de cifrado auto-generada | ✅ | Se crea en `%LOCALAPPDATA%\EquiposIndustriales\data\.crypto_key` |

### Riesgos conocidos

- **JWT_SECRET hardcodeado**: si no se define `JWT_SECRET` en entorno, usa
  `your-secret-key-change-in-production`
- **CORS permisivo**: actualmente `AllowOrigin::Any`
- **Bind a `0.0.0.0:3000`**: accesible desde toda la LAN, no hay firewall
  interno configurado
- **Sin TLS**: el tráfico viaja en texto plano; usar proxy reverso (nginx,
  Caddy) en producción
- **Sin logging a archivo**: toda salida va a stdout (solo visible en
  terminal de debug)

## Licencia

MIT
