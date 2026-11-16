#ifndef SourceDir
  #error SourceDir is required
#endif
[Setup]
AppId={{67CE46E2-ADE1-4C76-B545-47A678686ED8}
AppName=Worldcraft
AppVersion={#AppVersion}
VersionInfoVersion={#VersionNumbers}
AppPublisher=Worldcraft contributors
AppPublisherURL=https://github.com/InduwaraLasindu/Worldcraft
DefaultDirName={localappdata}\Programs\Worldcraft
DefaultGroupName=Worldcraft
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#OutputDir}
OutputBaseFilename=Worldcraft-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\Worldcraft.exe
LicenseFile={#ProjectRoot}\LICENSE
[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked
[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{autoprograms}\Worldcraft"; Filename: "{app}\Worldcraft.exe"
Name: "{autodesktop}\Worldcraft"; Filename: "{app}\Worldcraft.exe"; Tasks: desktopicon
[Run]
Filename: "{app}\Worldcraft.exe"; Description: "Launch Worldcraft"; Flags: nowait postinstall skipifsilent
