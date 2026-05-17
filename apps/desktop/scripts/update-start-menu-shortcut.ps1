$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$desktopDir = Split-Path -Parent $scriptDir
$exePath = Join-Path $desktopDir "release\win-unpacked\OpenWhisper.exe"
$appUserModelId = "com.openwhisper.desktop"

if (-not (Test-Path -LiteralPath $exePath)) {
  throw "OpenWhisper exe was not found at $exePath. Run the Windows exe build first."
}

$programsDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
if (-not (Test-Path -LiteralPath $programsDir)) {
  New-Item -ItemType Directory -Path $programsDir | Out-Null
}

$allUsersProgramsDir = Join-Path $env:ProgramData "Microsoft\Windows\Start Menu\Programs"
$appProgramsDir = Join-Path $programsDir "OpenWhisper"
if (-not (Test-Path -LiteralPath $appProgramsDir)) {
  New-Item -ItemType Directory -Path $appProgramsDir | Out-Null
}

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Runtime.InteropServices;

[ComImport]
[Guid("00021401-0000-0000-C000-000000000046")]
public class ShellLink {}

[ComImport]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
[Guid("000214F9-0000-0000-C000-000000000046")]
public interface IShellLinkW {
  void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszFile, int cchMaxPath, IntPtr pfd, uint fFlags);
  void GetIDList(out IntPtr ppidl);
  void SetIDList(IntPtr pidl);
  void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszName, int cchMaxName);
  void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string pszName);
  void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszDir, int cchMaxPath);
  void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string pszDir);
  void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszArgs, int cchMaxPath);
  void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string pszArgs);
  void GetHotkey(out short pwHotkey);
  void SetHotkey(short wHotkey);
  void GetShowCmd(out int piShowCmd);
  void SetShowCmd(int iShowCmd);
  void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszIconPath, int cchIconPath, out int piIcon);
  void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string pszIconPath, int iIcon);
  void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string pszPathRel, uint dwReserved);
  void Resolve(IntPtr hwnd, uint fFlags);
  void SetPath([MarshalAs(UnmanagedType.LPWStr)] string pszFile);
}

[ComImport]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
[Guid("0000010b-0000-0000-C000-000000000046")]
public interface IPersistFile {
  void GetClassID(out Guid pClassID);
  void IsDirty();
  void Load([MarshalAs(UnmanagedType.LPWStr)] string pszFileName, uint dwMode);
  void Save([MarshalAs(UnmanagedType.LPWStr)] string pszFileName, bool fRemember);
  void SaveCompleted([MarshalAs(UnmanagedType.LPWStr)] string pszFileName);
  void GetCurFile([MarshalAs(UnmanagedType.LPWStr)] out string ppszFileName);
}

[ComImport]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
[Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99")]
public interface IPropertyStore {
  void GetCount(out uint cProps);
  void GetAt(uint iProp, out PropertyKey pkey);
  void GetValue(ref PropertyKey key, out PropVariant pv);
  void SetValue(ref PropertyKey key, ref PropVariant pv);
  void Commit();
}

[StructLayout(LayoutKind.Sequential, Pack = 4)]
public struct PropertyKey {
  public Guid fmtid;
  public uint pid;
}

[StructLayout(LayoutKind.Sequential)]
public struct PropVariant {
  public ushort vt;
  public ushort wReserved1;
  public ushort wReserved2;
  public ushort wReserved3;
  public IntPtr p;
  public int p2;
}

public static class ShortcutProperties {
  private static readonly PropertyKey AppUserModelIdKey = new PropertyKey {
    fmtid = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"),
    pid = 5
  };

  public static void CreateOrUpdateShortcut(
    string shortcutPath,
    string targetPath,
    string workingDirectory,
    string description,
    string appUserModelId
  ) {
    object shellLink = new ShellLink();
    IShellLinkW link = (IShellLinkW)shellLink;
    link.SetPath(targetPath);
    link.SetWorkingDirectory(workingDirectory);
    link.SetDescription(description);
    link.SetIconLocation(targetPath, 0);

    PropVariant value = new PropVariant {
      vt = 31,
      p = Marshal.StringToCoTaskMemUni(appUserModelId)
    };

    try {
      IPropertyStore propertyStore = (IPropertyStore)shellLink;
      PropertyKey key = AppUserModelIdKey;
      propertyStore.SetValue(ref key, ref value);
      propertyStore.Commit();
      ((IPersistFile)shellLink).Save(shortcutPath, true);
    } finally {
      if (value.p != IntPtr.Zero) {
        Marshal.FreeCoTaskMem(value.p);
      }
      Marshal.ReleaseComObject(shellLink);
    }
  }
}
"@

$shortcutPath = Join-Path $programsDir "OpenWhisper.lnk"
$appShortcutPath = Join-Path $appProgramsDir "OpenWhisper.lnk"
$allUsersShortcutPath = Join-Path $allUsersProgramsDir "OpenWhisper.lnk"
$workingDirectory = Split-Path -Parent $exePath
[ShortcutProperties]::CreateOrUpdateShortcut(
  $shortcutPath,
  $exePath,
  $workingDirectory,
  "OpenWhisper",
  $appUserModelId
)
try {
  [ShortcutProperties]::CreateOrUpdateShortcut(
    $allUsersShortcutPath,
    $exePath,
    $workingDirectory,
    "OpenWhisper",
    $appUserModelId
  )
} catch {
  Write-Warning "Could not update all-users Start Menu shortcut at ${allUsersShortcutPath}: $($_.Exception.Message)"
}
[ShortcutProperties]::CreateOrUpdateShortcut(
  $appShortcutPath,
  $exePath,
  $workingDirectory,
  "OpenWhisper",
  $appUserModelId
)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class ShellNotify {
  [DllImport("shell32.dll")]
  public static extern void SHChangeNotify(int eventId, uint flags, IntPtr item1, IntPtr item2);
}
"@

[ShellNotify]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)

Write-Host "Updated Start Menu shortcuts:"
Write-Host "  $shortcutPath"
Write-Host "  $appShortcutPath"
if (Test-Path -LiteralPath $allUsersShortcutPath) {
  Write-Host "  $allUsersShortcutPath"
}
