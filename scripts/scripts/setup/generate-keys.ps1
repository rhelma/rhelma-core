$ErrorActionPreference = "Stop"

param(
  [string]$KeysDir = ".\\keys"
)

if (-not (Get-Command openssl -ErrorAction SilentlyContinue)) {
  Write-Host "❌ openssl not found. Install OpenSSL and retry." -ForegroundColor Red
  exit 127
}

New-Item -ItemType Directory -Force -Path $KeysDir | Out-Null

$priv = Join-Path $KeysDir "private.pem"
$pub  = Join-Path $KeysDir "public.pem"

if ((Test-Path $priv) -or (Test-Path $pub)) {
  Write-Host "⚠️  Keys already exist in '$KeysDir'. Refusing to overwrite." -ForegroundColor Yellow
  Write-Host "    Remove '$priv'/'$pub' and re-run if you want to regenerate." -ForegroundColor Yellow
  exit 2
}

openssl genrsa -out $priv 4096 | Out-Null
openssl rsa -in $priv -pubout -out $pub | Out-Null

Write-Host "✅ RSA keys generated:" -ForegroundColor Green
Write-Host "  - $priv"
Write-Host "  - $pub"
Write-Host "🔒 Keep keys out of git (keys/ is in .gitignore)." -ForegroundColor Cyan
