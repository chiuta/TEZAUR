@echo off
rem Windows: "Uninstall-Tezaur.bat" undoes what the installer recorded; "Uninstall-Tezaur.bat /full" also sweeps leftovers and removes the program.
cd /d "%~dp0"
where py >nul 2>nul && (set PY=py -3) || (set PY=python)
if /i "%1"=="/full" (%PY% tezaur-setup.py uninstall --sweep --purge) else (%PY% tezaur-setup.py uninstall)
pause
