#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif

#define AppName "APL"
#define AppPublisher "Absolute Programming Language"
#define AppURL "https://github.com/artstrnv-code/Absolute-programming-launge"
#define RepositoryRoot "..\.."

[Setup]
AppId={{B770353D-C549-4DA5-9DE0-C0F8F7619BCC}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
AppUpdatesURL={#AppURL}/releases
DefaultDirName={localappdata}\Programs\APL
DefaultGroupName=APL
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesAssociations=yes
ChangesEnvironment=yes
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\apl_studio.exe
OutputDir={#RepositoryRoot}\dist\installer
OutputBaseFilename=APL-{#AppVersion}-windows-x64-setup
SourceDir={#RepositoryRoot}\dist\package
SetupLogging=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked
Name: "addtopath"; Description: "Add APL command-line tools to PATH"; GroupDescription: "Command line:"; Flags: checkedonce

[Files]
Source: "*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\APL Studio"; Filename: "{app}\apl_studio.exe"; WorkingDir: "{userdocs}"
Name: "{group}\APL Documentation"; Filename: "{app}\docs\README.md"
Name: "{userdesktop}\APL Studio"; Filename: "{app}\apl_studio.exe"; WorkingDir: "{userdocs}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Classes\.apl\OpenWithProgids"; ValueType: none; ValueName: "APL.Source"; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Classes\APL.Source"; ValueType: string; ValueData: "APL source file"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\APL.Source\DefaultIcon"; ValueType: string; ValueData: "{app}\apl_studio.exe,0"
Root: HKCU; Subkey: "Software\Classes\APL.Source\shell\open\command"; ValueType: string; ValueData: """{app}\apl_studio.exe"" ""%1"""

[Run]
Filename: "{app}\apl_studio.exe"; Description: "Launch APL Studio"; Flags: nowait postinstall skipifsilent

[Code]
function PathContains(PathValue: string; Entry: string): Boolean;
var
  Remaining: string;
  Current: string;
  Separator: Integer;
begin
  Result := False;
  Remaining := PathValue;
  while Remaining <> '' do
  begin
    Separator := Pos(';', Remaining);
    if Separator = 0 then
    begin
      Current := Remaining;
      Remaining := '';
    end
    else
    begin
      Current := Copy(Remaining, 1, Separator - 1);
      Delete(Remaining, 1, Separator);
    end;
    if CompareText(Trim(Current), Entry) = 0 then
    begin
      Result := True;
      Exit;
    end;
  end;
end;

procedure AddToUserPath(Entry: string);
var
  Current: string;
begin
  if not RegQueryStringValue(HKCU, 'Environment', 'Path', Current) then
    Current := '';
  if PathContains(Current, Entry) then
    Exit;
  if (Current <> '') and (Copy(Current, Length(Current), 1) <> ';') then
    Current := Current + ';';
  RegWriteExpandStringValue(HKCU, 'Environment', 'Path', Current + Entry);
end;

procedure RemoveFromUserPath(Entry: string);
var
  Current: string;
  Remaining: string;
  Item: string;
  Updated: string;
  Separator: Integer;
begin
  if not RegQueryStringValue(HKCU, 'Environment', 'Path', Current) then
    Exit;
  Remaining := Current;
  Updated := '';
  while Remaining <> '' do
  begin
    Separator := Pos(';', Remaining);
    if Separator = 0 then
    begin
      Item := Remaining;
      Remaining := '';
    end
    else
    begin
      Item := Copy(Remaining, 1, Separator - 1);
      Delete(Remaining, 1, Separator);
    end;
    if (Trim(Item) <> '') and (CompareText(Trim(Item), Entry) <> 0) then
    begin
      if Updated <> '' then
        Updated := Updated + ';';
      Updated := Updated + Item;
    end;
  end;
  RegWriteExpandStringValue(HKCU, 'Environment', 'Path', Updated);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
  begin
    AddToUserPath(ExpandConstant('{app}'));
    RegWriteDWordValue(HKCU, 'Software\APL', 'PathInstalled', 1);
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Installed: Cardinal;
begin
  if (CurUninstallStep = usUninstall) and
     RegQueryDWordValue(HKCU, 'Software\APL', 'PathInstalled', Installed) and
     (Installed = 1) then
  begin
    RemoveFromUserPath(ExpandConstant('{app}'));
    RegDeleteValue(HKCU, 'Software\APL', 'PathInstalled');
    RegDeleteKeyIfEmpty(HKCU, 'Software\APL');
  end;
end;
