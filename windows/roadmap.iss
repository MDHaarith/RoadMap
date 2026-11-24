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
; Look only: Roadmap's route and icon instead of Inno's default pictures.
WizardImageFile=windows\wizard-100.bmp,windows\wizard-150.bmp,windows\wizard-200.bmp
WizardSmallImageFile=windows\wizard-small-100.bmp,windows\wizard-small-150.bmp,windows\wizard-small-200.bmp

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; Flags: unchecked
Name: connectai; Description: "Connect Roadmap to your AI tools (Claude Code, Codex, OpenCode, Gemini CLI, Cursor, VS Code and others) so they can read your roadmaps"
Name: opencode; Description: "Install OpenCode, so Roadmap can use its free AI models"; Flags: unchecked; Check: NeedsOpenCode

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
; OpenCode first, so connecting finds it. roadmap-mcp prints what it did; a failure here never stops the install.
Filename: "{app}\roadmap-mcp.exe"; Parameters: "install-opencode"; StatusMsg: "Installing OpenCode..."; Flags: runhidden; Tasks: opencode
Filename: "{app}\roadmap-mcp.exe"; Parameters: "connect"; StatusMsg: "Connecting Roadmap to your AI tools..."; Flags: runhidden; Tasks: connectai
; Unticked: take Roadmap out of the tools and stop the app adding it back on start.
Filename: "{app}\roadmap-mcp.exe"; Parameters: "disconnect"; Flags: runhidden; Tasks: not connectai
Filename: "{app}\roadmap-app.exe"; Description: "Open Roadmap"; Flags: nowait postinstall skipifsilent
; The in-app updater runs this installer with /SILENT; reopen the app afterwards.
Filename: "{app}\roadmap-app.exe"; Flags: nowait; Check: WizardSilent

[UninstallRun]
Filename: "{app}\roadmap-mcp.exe"; Parameters: "disconnect"; Flags: runhidden; RunOnceId: "DisconnectAITools"

[Messages]
WizardSelectTasks=Set up Roadmap
SelectTasksDesc=Choose what Setup does besides installing Roadmap.
SelectTasksLabel2=Tick what you want, then click Next.
FinishedHeadingLabel=Roadmap is ready
FinishedLabel=Roadmap is installed. Open it from the Start menu, or tick the box below to open it now.

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

function Home: String;
begin
  Result := ExpandConstant('{%USERPROFILE}');
end;

// The same places src/harness.rs find_opencode looks on Windows.
function NeedsOpenCode: Boolean;
begin
  Result := (FileSearch('opencode.exe', GetEnv('PATH')) = '')
    and (FileSearch('opencode.cmd', GetEnv('PATH')) = '')
    and not FileExists(Home + '\.opencode\bin\opencode.exe')
    and not FileExists(ExpandConstant('{userappdata}\npm\opencode.cmd'));
end;

procedure AddTool(var List: String; Name: String; Found: Boolean);
begin
  if Found then
  begin
    if List <> '' then
      List := List + ', ';
    List := List + Name;
  end;
end;

// The tools `roadmap-mcp connect` adds itself to, found the same way as in src/harness.rs.
function FoundTools: String;
var
  Codex: String;
begin
  Result := '';
  Codex := GetEnv('CODEX_HOME');
  if Codex = '' then
    Codex := Home + '\.codex';
  AddTool(Result, 'Claude Code', FileExists(Home + '\.claude.json'));
  AddTool(Result, 'Claude Desktop', DirExists(ExpandConstant('{userappdata}\Claude')));
  AddTool(Result, 'Codex', DirExists(Codex));
  AddTool(Result, 'OpenCode', (not NeedsOpenCode) or DirExists(Home + '\.config\opencode'));
  AddTool(Result, 'Gemini CLI', DirExists(Home + '\.gemini'));
  AddTool(Result, 'Cursor', DirExists(Home + '\.cursor'));
  AddTool(Result, 'VS Code', DirExists(ExpandConstant('{userappdata}\Code\User')));
  AddTool(Result, 'Windsurf', DirExists(Home + '\.codeium\windsurf'));
end;

// Roadmap's blue on page titles, and a line under the tasks saying which AI tools were found.
procedure InitializeWizard;
var
  Found: TNewStaticText;
  Tools: String;
begin
  WizardForm.PageNameLabel.Font.Color := $C9661A;
  Tools := FoundTools;
  if Tools = '' then
    Tools := 'none yet. Roadmap connects to them when you install one and open Roadmap.'
  else
    Tools := Tools + '.';
  WizardForm.TasksList.Height := WizardForm.TasksList.Height - ScaleY(48);
  Found := TNewStaticText.Create(WizardForm);
  Found.Parent := WizardForm.SelectTasksPage;
  Found.AutoSize := False;
  Found.WordWrap := True;
  Found.Left := WizardForm.TasksList.Left;
  Found.Top := WizardForm.TasksList.Top + WizardForm.TasksList.Height + ScaleY(10);
  Found.Width := WizardForm.TasksList.Width;
  Found.Height := ScaleY(38);
  Found.Anchors := [akLeft, akRight, akBottom];
  Found.Font.Color := $61595D;
  Found.Caption := 'AI tools found on this computer: ' + Tools;
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
