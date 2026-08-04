@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build_dev.ps1"
exit /b %ERRORLEVEL%
