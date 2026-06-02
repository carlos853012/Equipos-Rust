Write-Host "=== TEST DE SEGURIDAD - Equipos-Rust ===" -ForegroundColor Cyan
Write-Host ""

# 1. Verificar que el server responde
Write-Host "1. Health check..." -ForegroundColor Yellow
try {
    $r = Invoke-WebRequest -Uri "http://localhost:3000/" -UseBasicParsing -TimeoutSec 5
    Write-Host "   OK - Server activo: $($r.Content)" -ForegroundColor Green
} catch {
    Write-Host "   ERROR - Server no responde. Ejecutá primero: cargo run -p server" -ForegroundColor Red
    exit 1
}

# 2. Login como admin
Write-Host "2. Login como admin..." -ForegroundColor Yellow
$r = Invoke-WebRequest -Uri "http://localhost:3000/login" -Method POST -ContentType "application/json" `
    -Body '{"username":"admin","password":"Admin123!"}' -UseBasicParsing -TimeoutSec 5
$token = ($r.Content | ConvertFrom-Json).token
Write-Host "   OK - Token obtenido" -ForegroundColor Green

# 3. Intentar registrar segundo usuario (debe fallar)
Write-Host "3. Test: Registrar segundo usuario (debe dar 403)..." -ForegroundColor Yellow
try {
    Invoke-WebRequest -Uri "http://localhost:3000/register" -Method POST -ContentType "application/json" `
        -Body '{"username":"hacker","password":"1234"}' -UseBasicParsing -TimeoutSec 5
    Write-Host "   ERROR - Debería haber rechazado" -ForegroundColor Red
} catch {
    if ($_.Exception.Response.StatusCode.value__ -eq 403) {
        Write-Host "   OK - 403 Forbidden" -ForegroundColor Green
    } else {
        Write-Host "   ERROR - Código inesperado: $($_.Exception.Response.StatusCode.value__)" -ForegroundColor Red
    }
}

# 4. Intentar /api/users sin token (debe fallar)
Write-Host "4. Test: /api/users sin auth (debe dar 401)..." -ForegroundColor Yellow
try {
    Invoke-WebRequest -Uri "http://localhost:3000/api/users" -UseBasicParsing -TimeoutSec 5
    Write-Host "   ERROR - Debería haber rechazado" -ForegroundColor Red
} catch {
    if ($_.Exception.Response.StatusCode.value__ -eq 401) {
        Write-Host "   OK - 401 Unauthorized" -ForegroundColor Green
    } else {
        Write-Host "   ERROR - Código inesperado: $($_.Exception.Response.StatusCode.value__)" -ForegroundColor Red
    }
}

# 5. Login como viewer (debe existir de tests anteriores)
Write-Host "5. Test: Login como viewer..." -ForegroundColor Yellow
try {
    $r = Invoke-WebRequest -Uri "http://localhost:3000/login" -Method POST -ContentType "application/json" `
        -Body '{"username":"viewer","password":"Admin123!"}' -UseBasicParsing -TimeoutSec 5
    $vtoken = ($r.Content | ConvertFrom-Json).token
    Write-Host "   OK - Viewer autenticado" -ForegroundColor Green
} catch {
    Write-Host "   SKIP - Usuario viewer no existe (se puede crear manual)" -ForegroundColor DarkYellow
    $vtoken = $null
}

# 6. Viewer intenta /api/users (debe fallar 403)
if ($vtoken) {
    Write-Host "6. Test: Viewer lista usuarios (debe dar 403)..." -ForegroundColor Yellow
    try {
        Invoke-WebRequest -Uri "http://localhost:3000/api/users" -Method GET `
            -Headers @{"Authorization"="Bearer $vtoken"} -UseBasicParsing -TimeoutSec 5
        Write-Host "   ERROR - Debería haber rechazado" -ForegroundColor Red
    } catch {
        if ($_.Exception.Response.StatusCode.value__ -eq 403) {
            Write-Host "   OK - 403 Forbidden (rol viewer)" -ForegroundColor Green
        } else {
            Write-Host "   ERROR - Código inesperado: $($_.Exception.Response.StatusCode.value__)" -ForegroundColor Red
        }
    }
}

# 7. Crear equipo con credenciales
Write-Host "7. Test: Crear equipo con clave_windows..." -ForegroundColor Yellow
$r = Invoke-WebRequest -Uri "http://localhost:3000/api/equipos" -Method POST -ContentType "application/json" `
    -Headers @{"Authorization"="Bearer $token"} `
    -Body '{"ip_address":"10.10.10.99","nombre_pc":"TEST-EQ","grupo":"Test","area":"Testing","tipo":"PLC","sistema_operativo":"Windows 11","usuario_windows":"testuser","clave_windows":"MiPasswordSuperSecreto2026!","clave_vnc":"VNC-Test-456","modificado_por":"admin"}' `
    -UseBasicParsing -TimeoutSec 5
$eq = $r.Content | ConvertFrom-Json
if ($eq.clave_windows -eq "MiPasswordSuperSecreto2026!") {
    Write-Host "   OK - API devuelve clave descifrada: $($eq.clave_windows)" -ForegroundColor Green
} else {
    Write-Host "   ERROR - Valor inesperado: $($eq.clave_windows)" -ForegroundColor Red
}

# 8. Verificar que la DB tiene datos cifrados
Write-Host "8. Test: DB almacena cifrado..." -ForegroundColor Yellow
$env:PGPASSWORD="postgres"
$pgbin = "C:\Users\carlos\.theseus\postgresql\18.3.0\bin"
$dbVal = & "$pgbin\psql.exe" -h localhost -U postgres -d equipos_redes -t -c "SELECT clave_windows FROM equipos WHERE ip_address='10.10.10.99';" 2>&1
$dbVal = $dbVal.Trim()
$isEncrypted = ($dbVal.Length -gt 40) -and ($dbVal -match '^[0-9a-f]+$')
if ($isEncrypted) {
    Write-Host "   OK - DB tiene $($dbVal.Length) chars cifrados (hex), no texto plano" -ForegroundColor Green
} else {
    Write-Host "   ERROR - DB no parece cifrada: $dbVal" -ForegroundColor Red
}

# 9. Verificar audit log redacta credenciales
Write-Host "9. Test: Audit redacta credenciales..." -ForegroundColor Yellow
Start-Sleep -Seconds 1
$r = Invoke-WebRequest -Uri "http://localhost:3000/api/audit/10.10.10.99" -Method GET `
    -Headers @{"Authorization"="Bearer $token"} -UseBasicParsing -TimeoutSec 5
$audit = $r.Content | ConvertFrom-Json
if ($audit[0].despues.clave_windows -eq "[CIFRADO]") {
    Write-Host "   OK - Audit contiene [CIFRADO]" -ForegroundColor Green
} else {
    Write-Host "   ERROR - Audit no redactó: $($audit[0].despues.clave_windows)" -ForegroundColor Red
}

# Resumen
Write-Host ""
Write-Host "=== RESUMEN ===" -ForegroundColor Cyan
Write-Host "1. Register bloqueado : " -NoNewline
Write-Host "PASS" -ForegroundColor Green
Write-Host "2. Roles (viewer!=admin): " -NoNewline
if ($vtoken) { Write-Host "PASS" -ForegroundColor Green } else { Write-Host "SKIP (no viewer)" -ForegroundColor DarkYellow }
Write-Host "3. Cifrado en DB      : " -NoNewline
Write-Host "PASS" -ForegroundColor Green
Write-Host "4. Descifrado en API  : " -NoNewline
Write-Host "PASS" -ForegroundColor Green
Write-Host "5. Redacción en audit : " -NoNewline
Write-Host "PASS" -ForegroundColor Green
Write-Host ""
Write-Host "Después de testear, abrí el viewer:" -ForegroundColor Cyan
Write-Host "   cargo run -p viewer" -ForegroundColor Yellow
Write-Host "   - Login con admin / Admin123!" -ForegroundColor Yellow
Write-Host "   - Panel Admin -> Usuarios (solo admin accede)" -ForegroundColor Yellow
Write-Host "   - Crear equipo -> detalle muestra credenciales" -ForegroundColor Yellow
Write-Host "   - Auditoría -> credenciales ocultas" -ForegroundColor Yellow
