$ErrorActionPreference = "Stop"
powershell -NoProfile -ExecutionPolicy Bypass -File "$(Split-Path -Parent $MyInvocation.MyCommand.Path)\setup\bootstrap.ps1" @args
