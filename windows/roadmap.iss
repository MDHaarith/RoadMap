; Roadmap installer (Inno Setup 6). Per-user: no admin prompt, no unsigned plugin
; DLLs (NSIS loads five from %TEMP%, which Smart App Control blocks), and the
; in-app updater can rerun it silently. Build: scripts/build-windows-installer.ps1

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{3F8A2C61-7D4E-4B9A-9E15-C2B7D04A6F83}
AppName=Roadmap
AppVersion={#AppVersion}
AppPublisher=MDHaarith
AppPublisherURL=https://github.com/MDHaarith/RoadMap-releases
AppUpdatesURL=https://github.com/MDHaarith/RoadMap-releases/releases
VersionInfoVersion={#AppVersion}
VersionInfoCompany=MDHaarith
VersionInfoDescription=Roadmap installer
VersionInfoProductName=Roadmap
VersionInfoCopyright=Copyright (c) 2026 MDHaarith
; Same folder as the 1.0.0 NSIS installer, so updates replace it in place.
DefaultDirName={localappdata}\Roadmap
DisableDirPage=auto
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
SourceDir=..
OutputDir=target\release
OutputBaseFilename=Roadmap_{#AppVersion}_x64-setup
SetupIconFile=icons\icon.ico
UninstallDisplayIcon={app}\roadmap-app.exe
UninstallDisplayName=Roadmap
; Close a running Roadmap before replacing it.
CloseApplications=yes
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "target\release\roadmap-app.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "target\release\roadmap-mcp.exe"; DestDir: "{app}"; Flags: ignoreversion
; Microsoft's WebView2 bootstrapper, for the app window on systems without the runtime.
Source: "target\release\MicrosoftEdgeWebview2Setup.exe"; DestDir: "{tmp}"; Flags: deleteafterinstall; Check: NeedsWebView2

[Icons]
Name: "{userprograms}\Roadmap"; Filename: "{app}\roadmap-app.exe"
Name: "{userdesktop}\Roadmap"; Filename: "{app}\roadmap-app.exe"; Tasks: desktopicon

[Run]
Filename: "{tmp}\MicrosoftEdgeWebview2Setup.exe"; Parameters: "/silent /install"; StatusMsg: "Installing Microsoft Edge WebView2..."; Check: NeedsWebView2
Filename: "{app}\roadmap-app.exe"; Description: "Open Roadmap"; Flags: nowait postinstall skipifsilent
; The in-app updater runs this installer with /SILENT; reopen the app afterwards.
Filename: "{app}\roadmap-app.exe"; Flags: nowait; Check: WizardSilent

[Code]
const
  WebView2Key = 'Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  NsisUninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Roadmap';

function HasVersion(Root: Integer; Key: String): Boolean;
var
  Version: String;
begin
  Result := RegQueryStringValue(Root, Key, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0');
end;

function NeedsWebView2: Boolean;
begin
  Result := not (HasVersion(HKLM, 'SOFTWARE\WOW6432Node\' + WebView2Key)
    or HasVersion(HKLM, 'SOFTWARE\' + WebView2Key)
    or HasVersion(HKCU, 'Software\' + WebView2Key));
end;

// Replacing a 1.0.0 NSIS install: drop its uninstaller and Apps list entry so only this one remains.
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    DeleteFile(ExpandConstant('{app}\uninstall.exe'));
    RegDeleteKeyIncludingSubkeys(HKCU, NsisUninstallKey);
  end;
end;
