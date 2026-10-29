; Inno Setup script: a classic installer around the PortableApps folder (run build-portable.bat first).
; Per-user install into %LOCALAPPDATA%\Programs\TEZAUR so that Data\ (index, settings) stays writable without admin rights.
#define AppName "TEZAUR"
#define AppVer "0.1.0"
[Setup]
AppId={{B2A5E5D0-4B7C-4F1E-9C61-7E0E3B2E7A11}
AppName={#AppName}
AppVersion={#AppVer}
AppPublisher=Alexio
AppPublisherURL=https://trom.tf
DefaultDirName={localappdata}\Programs\TEZAUR
DefaultGroupName=TEZAUR
PrivilegesRequired=lowest
OutputDir=..\..\dist
OutputBaseFilename=TEZAUR-setup-{#AppVer}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
[Languages]
Name: "romanian"; MessagesFile: "Romanian.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
[Files]
Source: "..\..\dist\TezaurPortable\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion; Excludes: "Data\*"
[Dirs]
Name: "{app}\Data"; Permissions: users-modify
[Icons]
Name: "{group}\TEZAUR"; Filename: "{app}\TezaurPortable.exe"
Name: "{group}\Ajutor"; Filename: "{app}\help.html"
Name: "{autodesktop}\TEZAUR"; Filename: "{app}\TezaurPortable.exe"; Tasks: desktopicon
[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"
[Run]
Filename: "{app}\TezaurPortable.exe"; Description: "{cm:LaunchProgram,TEZAUR}"; Flags: nowait postinstall skipifsilent
[UninstallDelete]
; Data\ (your library index and settings) is deliberately left in place on uninstall.
