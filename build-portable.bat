@echo off
rem Assembles dist\TezaurPortable (PortableApps.com format) on Windows. Requires Rust (rustup.rs).
cd /d "%~dp0"
cargo build --release || exit /b 1
if exist dist\TezaurPortable rmdir /s /q dist\TezaurPortable
mkdir dist
xcopy /E /I /Y packaging\TezaurPortable dist\TezaurPortable >nul
copy /Y target\release\tezaur.exe dist\TezaurPortable\App\Tezaur\ >nul
copy /Y target\release\TezaurPortable.exe dist\TezaurPortable\ >nul
echo ready: dist\TezaurPortable
echo Next (optional): open the folder with PortableApps.com Installer to produce the .paf.exe
