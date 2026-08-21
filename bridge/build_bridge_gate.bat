@echo off
setlocal
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build_bridge.ps1" -QualityGate
exit /b %ERRORLEVEL%
