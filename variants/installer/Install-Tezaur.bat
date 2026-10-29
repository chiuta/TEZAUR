@echo off
rem Windows: double-click to see what would be installed; "Install-Tezaur.bat /yes" installs everything that is ready.
cd /d "%~dp0"
where py >nul 2>nul && (set PY=py -3) || (set PY=python)
if /i "%1"=="/yes" (%PY% tezaur-setup.py install --yes) else (%PY% tezaur-setup.py scan & echo. & echo Run "Install-Tezaur.bat /yes" to apply. & pause)
