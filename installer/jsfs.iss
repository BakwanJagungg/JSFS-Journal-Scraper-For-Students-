; JSFS Windows x64 Installer Script (Inno Setup 6)
#define MyAppName "JSFS - Journal Scraper For Students"
#ifndef MyAppVersion
  #define MyAppVersion "4.13.1"
#endif
#define MyAppPublisher "BakwanJagungg"
#define MyAppURL "https://github.com/BakwanJagungg/JSFS-Journal-Scraper-For-Students-"
#define MyAppExeName "jsfs.exe"

[Setup]
AppId={{8D4F6A2E-7A42-4B9E-9A51-7E4B2F5A8C10}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\JSFS
DefaultGroupName=JSFS
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
LicenseFile=..\LICENSE
OutputDir=..
OutputBaseFilename=JSFS-windows-x64-setup
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
Name: "{group}\JSFS"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppExeName}"
Name: "{group}\Uninstall JSFS"; Filename: "{uninstallexe}"
Name: "{autodesktop}\JSFS"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; IconFilename: "{app}\{#MyAppExeName}"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch JSFS"; Flags: nowait postinstall skipifsilent
