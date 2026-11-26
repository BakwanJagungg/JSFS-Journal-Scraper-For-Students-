; JSFS+ Workstation Windows x64 Installer Script (Inno Setup 6)
#define MyAppName "JSFS+ Workstation - Journal Scraper For Students"
#ifndef MyAppVersion
  #define MyAppVersion "5.0.1"
#endif
#define MyAppPublisher "BakwanJagungg"
#define MyAppURL "https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-"
#define MyAppExeName "jsfs-plus.exe"

[Setup]
AppId={{B3D7E91A-4C22-4F8B-A104-9E7C5D3B2108}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\JSFS-Plus
DefaultGroupName=JSFS+ Workstation
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
LicenseFile=..\LICENSE
OutputDir=..
OutputBaseFilename=JSFS-Plus-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\JSFS+ Workstation"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall JSFS+ Workstation"; Filename: "{uninstallexe}"
Name: "{autodesktop}\JSFS+ Workstation"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; IconFilename: "{app}\{#MyAppExeName}"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch JSFS+ Workstation"; Flags: nowait postinstall skipifsilent
