import { app, BrowserWindow, ipcMain, Menu, Tray } from 'electron';
import * as fs from 'fs';
import * as path from 'path';
import { spawn, type ChildProcess } from 'child_process';
import { fileURLToPath } from 'url';
import {
  connectToRust,
  disconnectRust,
  setMainWindow,
  setOverlayWindow,
  setupIpcHandlers,
} from './ipc.js';
import { createAppIcon } from './tray-icon.js';
import {
  clearPathsManifestCache,
  debugNativeBinaryCandidates,
  nativeTarget,
} from './openwhisper-paths.js';
import { loadJson, saveJson } from './store.js';
import { SUPPORTED_ASR_LANGUAGES, type AppSettings, type AsrLanguageCode } from '../shared/types.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SETTINGS_FILE = 'settings.json';
const DEFAULT_SETTINGS: AppSettings = {
  model: 'medium_en_q8',
  device: 'auto',
  spokenLanguages: ['en'],
  autoDetectLanguage: false,
  launchAtLogin: false,
  showWindowOnLaunch: true,
};
const SUPPORTED_SPOKEN_LANGUAGES = new Set<AsrLanguageCode>(
  SUPPORTED_ASR_LANGUAGES.map((language) => language.id)
);
const WINDOWS_APP_USER_MODEL_ID = 'com.openwhisper.desktop';
const LOCALHOST_HOSTNAMES = new Set(['localhost', '127.0.0.1', '::1']);
type StoredSettings = Partial<Omit<AppSettings, 'model' | 'spokenLanguages' | 'autoDetectLanguage'>> & {
  model?: unknown;
  spokenLanguages?: unknown;
  autoDetectLanguage?: unknown;
};

let mainWindow: BrowserWindow | null = null;
let overlayWindow: BrowserWindow | null = null;
let tray: Tray | null = null;
let rustProcess: ChildProcess | null = null;
let isQuitting = false;
let settings: AppSettings = DEFAULT_SETTINGS;

function isAsrModel(model: unknown): model is AppSettings['model'] {
  return model === 'medium_en_q8' || model === 'large_v3_turbo_q8';
}

function isAsrLanguageCode(language: string): language is AsrLanguageCode {
  return SUPPORTED_SPOKEN_LANGUAGES.has(language as AsrLanguageCode);
}

function normalizeSpokenLanguages(value: unknown, model: AppSettings['model']): AsrLanguageCode[] {
  if (model === 'medium_en_q8') return ['en'];
  if (!Array.isArray(value)) return DEFAULT_SETTINGS.spokenLanguages;

  const languages: AsrLanguageCode[] = [];
  for (const item of value) {
    if (typeof item !== 'string') continue;

    const language = item.toLowerCase();
    if (!isAsrLanguageCode(language) || languages.includes(language)) continue;

    languages.push(language);
  }

  return languages.length > 0 ? languages : DEFAULT_SETTINGS.spokenLanguages;
}

function normalizeSettings(storedSettings: StoredSettings): AppSettings {
  const nextSettings = { ...DEFAULT_SETTINGS, ...storedSettings };
  const model = isAsrModel(nextSettings.model) ? nextSettings.model : DEFAULT_SETTINGS.model;
  const autoDetectLanguage =
    model === 'large_v3_turbo_q8' && nextSettings.autoDetectLanguage === true;
  const launchAtLogin = nextSettings.launchAtLogin === true;
  const showWindowOnLaunch = nextSettings.showWindowOnLaunch !== false;

  return {
    ...nextSettings,
    model,
    device: normalizeDevice(nextSettings.device),
    spokenLanguages: normalizeSpokenLanguages(nextSettings.spokenLanguages, model),
    autoDetectLanguage,
    launchAtLogin,
    showWindowOnLaunch,
  };
}

function normalizeDevice(value: unknown): AppSettings['device'] {
  return value === 'auto' || value === 'cpu' || value === 'gpu' ? value : DEFAULT_SETTINGS.device;
}

function isLoopbackDevServer(rawUrl: string): boolean {
  try {
    const parsed = new URL(rawUrl);
    const isHttp = parsed.protocol === 'http:' || parsed.protocol === 'https:';
    return isHttp && LOCALHOST_HOSTNAMES.has(parsed.hostname);
  } catch {
    return false;
  }
}

function resolveDevServerUrl(): string | undefined {
  const devServerUrl = process.env.VITE_DEV_SERVER_URL;
  if (!devServerUrl) return undefined;
  if (isLoopbackDevServer(devServerUrl)) return devServerUrl;
  console.warn(
    `[Security] Ignoring non-loopback VITE_DEV_SERVER_URL: ${devServerUrl}. Falling back to bundled renderer.`
  );
  return undefined;
}

function loadSettings(): AppSettings {
  return normalizeSettings(loadJson<StoredSettings>(SETTINGS_FILE, {}));
}

function createMainWindow(): void {
  Menu.setApplicationMenu(null);

  mainWindow = new BrowserWindow({
    width: 880,
    height: 720,
    minWidth: 720,
    minHeight: 560,
    show: false,
    backgroundColor: '#09090b',
    autoHideMenuBar: true,
    icon: createAppIcon(256),
    titleBarStyle: 'hidden',
    titleBarOverlay: {
      color: '#09090b',
      symbolColor: '#d4d4d8',
      height: 44,
    },
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  mainWindow.on('close', (event) => {
    if (isQuitting) return;
    event.preventDefault();
    mainWindow?.hide();
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  setMainWindow(mainWindow);
}

function createOverlayWindow(): void {
  overlayWindow = new BrowserWindow({
    width: 420,
    height: 160,
    show: false,
    frame: false,
    transparent: true,
    resizable: false,
    movable: false,
    minimizable: false,
    maximizable: false,
    fullscreenable: false,
    skipTaskbar: true,
    focusable: false,
    hasShadow: false,
    alwaysOnTop: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  overlayWindow.setAlwaysOnTop(true, 'screen-saver');
  overlayWindow.setIgnoreMouseEvents(true);

  overlayWindow.on('closed', () => {
    overlayWindow = null;
  });

  setOverlayWindow(overlayWindow);
}

function createTray(): void {
  tray = new Tray(createAppIcon(32));
  tray.setToolTip('OpenWhisper — hold Ctrl + Super to dictate');

  const menu = Menu.buildFromTemplate([
    { label: 'Open OpenWhisper', click: () => showMainWindow() },
    { type: 'separator' },
    { label: 'Hold Ctrl + Super to dictate', enabled: false },
    { type: 'separator' },
    {
      label: 'Quit OpenWhisper',
      click: () => {
        isQuitting = true;
        app.quit();
      },
    },
  ]);
  tray.setContextMenu(menu);
  tray.on('click', () => showMainWindow());
}

function showMainWindow(): void {
  if (!mainWindow || mainWindow.isDestroyed()) {
    createMainWindow();
    void loadMainWindow();
  }
  mainWindow?.show();
  mainWindow?.focus();
}

async function loadMainWindow(): Promise<void> {
  if (!mainWindow) throw new Error('Main window has not been created');
  const devServerUrl = resolveDevServerUrl();
  if (devServerUrl) {
    await mainWindow.loadURL(devServerUrl);
    mainWindow.webContents.openDevTools({ mode: 'detach' });
    return;
  }
  await mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
}

async function loadOverlayWindow(): Promise<void> {
  if (!overlayWindow) throw new Error('Overlay window has not been created');
  const devServerUrl = resolveDevServerUrl();
  if (devServerUrl) {
    await overlayWindow.loadURL(`${devServerUrl}/overlay.html`);
    return;
  }
  await overlayWindow.loadFile(path.join(__dirname, '../renderer/overlay.html'));
}

function findProjectRoot(): string {
  const starts = [
    process.env.OPENWHISPER_PROJECT_ROOT,
    process.cwd(),
    app.isPackaged ? process.resourcesPath : undefined,
    path.dirname(app.getPath('exe')),
    __dirname,
  ].filter((candidate): candidate is string => Boolean(candidate));
  const seen = new Set<string>();

  for (const start of starts) {
    let current = path.resolve(start);
    while (!seen.has(current)) {
      seen.add(current);
      const hasNativeCrate = fs.existsSync(
        path.join(current, 'crates', 'openwhisper-native', 'Cargo.toml')
      );
      const hasAsrConfig = fs.existsSync(
        path.join(current, 'config', 'whispercpp-profiles.json')
      );
      if (hasNativeCrate && hasAsrConfig) return current;

      const parent = path.dirname(current);
      if (parent === current) break;
      current = parent;
    }
  }

  throw new Error('Could not find OpenWhisper project root from packaged app location');
}

function packagedAssetPath(...parts: string[]): string {
  return path.join(process.resourcesPath, ...parts);
}

type RustHelperLaunch =
  | {
      kind: 'binary';
      workspaceRoot: string;
      binaryPath: string;
      cwd: string;
      usePackagedAssets: boolean;
    }
  | {
      kind: 'cargo';
      workspaceRoot: string;
      cwd: string;
      usePackagedAssets: false;
    };

let cachedRustHelperLaunch: RustHelperLaunch | null = null;

function platformExe(base: string): string {
  return process.platform === 'win32' ? `${base}.exe` : base;
}

function whisperCppAssetEnv(assetsRoot: string): NodeJS.ProcessEnv {
  const binary = platformExe('whisper-cli');
  return {
    OPENWHISPER_WHISPERCPP_CONFIG: path.join(assetsRoot, 'config', 'whispercpp-profiles.json'),
    OPENWHISPER_MODEL_DIR: path.join(assetsRoot, 'models'),
    OPENWHISPER_WHISPERCPP_CPU_EXE: path.join(assetsRoot, 'whispercpp', 'cpu', binary),
    OPENWHISPER_WHISPERCPP_GPU_EXE: path.join(assetsRoot, 'whispercpp', 'gpu', binary),
  };
}

function resolveRustHelperLaunch(): RustHelperLaunch {
  const workspaceRoot = findProjectRoot();
  const packagedBinaryPath = packagedAssetPath('native', platformExe('openwhisper-native'));
  const packagedConfigPath = packagedAssetPath('config', 'whispercpp-profiles.json');

  if (app.isPackaged) {
    if (!fs.existsSync(packagedBinaryPath)) {
      throw new Error(`Bundled Rust helper is missing: ${packagedBinaryPath}`);
    }
    if (!fs.existsSync(packagedConfigPath)) {
      throw new Error(`Bundled whisper.cpp config is missing: ${packagedConfigPath}`);
    }
    return {
      kind: 'binary',
      workspaceRoot,
      binaryPath: packagedBinaryPath,
      cwd: process.resourcesPath,
      usePackagedAssets: true,
    };
  }

  const devBinaryPath = debugNativeBinaryCandidates(
    workspaceRoot,
    'openwhisper-native',
    'openwhisper-native'
  ).find((candidate) => fs.existsSync(candidate));

  if (devBinaryPath) {
    return {
      kind: 'binary',
      workspaceRoot,
      binaryPath: devBinaryPath,
      cwd: workspaceRoot,
      usePackagedAssets: false,
    };
  }

  console.warn(
    '[Main] Native helper binary not found; falling back to `cargo run` (slower startup).'
  );
  return {
    kind: 'cargo',
    workspaceRoot,
    cwd: workspaceRoot,
    usePackagedAssets: false,
  };
}

function getRustHelperLaunch(): RustHelperLaunch {
  if (!cachedRustHelperLaunch) {
    cachedRustHelperLaunch = resolveRustHelperLaunch();
  }
  return cachedRustHelperLaunch;
}

function rustEnv(launch: RustHelperLaunch): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    OPENWHISPER_ASR_MODEL: settings.model,
    OPENWHISPER_ASR_DEVICE: normalizeDevice(settings.device),
    OPENWHISPER_ASR_LANGUAGES: settings.spokenLanguages.join(','),
    OPENWHISPER_ASR_AUTO_DETECT_LANGUAGE: settings.autoDetectLanguage ? '1' : '0',
  };

  if (launch.usePackagedAssets) {
    return { ...env, ...whisperCppAssetEnv(process.resourcesPath) };
  }

  const workspaceRoot = launch.workspaceRoot;
  return {
    ...env,
    OPENWHISPER_PROJECT_ROOT: workspaceRoot,
    OPENWHISPER_WHISPERCPP_CONFIG: path.join(workspaceRoot, 'config', 'whispercpp-profiles.json'),
    OPENWHISPER_MODEL_DIR: path.join(workspaceRoot, 'models'),
  };
}

function ipcAddress(pid: number): string {
  if (process.platform === 'win32') {
    return `\\\\.\\pipe\\OpenWhisper-${pid}`;
  }
  const runtimeDir = process.env.XDG_RUNTIME_DIR || '/tmp';
  return path.join(runtimeDir, `openwhisper-${pid}.sock`);
}

function startRustHelper(): Promise<string> {
  return new Promise((resolve, reject) => {
    const electronPid = process.pid;
    const pipeName = ipcAddress(electronPid);
    const launch = getRustHelperLaunch();
    const rustCwd = path.join(launch.workspaceRoot, 'crates', 'openwhisper-native');
    let settled = false;

    const settleReady = (): void => {
      if (settled) return;
      settled = true;
      resolve(pipeName);
    };

    const rustEnvVars = rustEnv(launch);
    const rustSpawnOptions = {
      stdio: ['ignore', 'pipe', 'pipe'] as ['ignore', 'pipe', 'pipe'],
      env: rustEnvVars,
    };

    if (launch.kind === 'cargo') {
      console.log('[Main] Starting native helper via cargo run');
      rustProcess = spawn(
        'cargo',
        [
          'run',
          '--target',
          nativeTarget(launch.workspaceRoot),
          '--',
          '--pipe-pid',
          String(electronPid),
        ],
        { ...rustSpawnOptions, cwd: rustCwd }
      );
    } else {
      console.log(`[Main] Starting native helper: ${launch.binaryPath}`);
      rustProcess = spawn(launch.binaryPath, ['--pipe-pid', String(electronPid)], {
        ...rustSpawnOptions,
        cwd: launch.cwd,
      });
    }

    const rustChild = rustProcess;

    const handleOutput = (prefix: string, data: Buffer): void => {
      const text = data.toString('utf-8').trim();
      if (text) console.log(`${prefix} ${text}`);
      if (text.includes('Starting IPC server')) settleReady();
    };

    rustChild.stdout?.on('data', (data: Buffer) => handleOutput('[Rust]', data));
    rustChild.stderr?.on('data', (data: Buffer) => handleOutput('[Rust stderr]', data));

    rustChild.on('error', (err) => {
      if (settled) {
        console.error('[Rust] Process error:', err);
        return;
      }
      settled = true;
      reject(err);
    });

    rustChild.on('exit', (code) => {
      console.log(`[Rust] Process exited with code ${code}`);
    });

    setTimeout(settleReady, 5_000);
  });
}

async function restartEngine(): Promise<{ ok: boolean; error?: string }> {
  disconnectRust();
  rustProcess?.kill();
  rustProcess = null;
  cachedRustHelperLaunch = null;
  clearPathsManifestCache();
  await new Promise((resolve) => setTimeout(resolve, 500));
  try {
    const pipeName = await startRustHelper();
    await connectToRust(pipeName);
    return { ok: true };
  } catch (err) {
    console.error('[Main] Failed to restart engine:', err);
    return { ok: false, error: (err as Error).message };
  }
}

function registerAppIpc(): void {
  ipcMain.handle('settings:read', () => settings);

  ipcMain.handle('settings:write', (_event, partial: Partial<AppSettings>) => {
    settings = normalizeSettings({ ...settings, ...partial });
    saveJson(SETTINGS_FILE, settings);
    if ('launchAtLogin' in partial) {
      app.setLoginItemSettings({ openAtLogin: settings.launchAtLogin });
    }
    return settings;
  });

  ipcMain.handle('engine:restart', () => restartEngine());
}

function bootstrap(): void {
  app.setAppUserModelId(WINDOWS_APP_USER_MODEL_ID);
  app.on('second-instance', () => showMainWindow());

  app.whenReady().then(async () => {
    settings = loadSettings();
    app.setLoginItemSettings({ openAtLogin: settings.launchAtLogin });

    createMainWindow();
    createOverlayWindow();
    createTray();
    setupIpcHandlers();
    registerAppIpc();

    try {
      const pipeName = await startRustHelper();
      await connectToRust(pipeName);
      console.log('[Main] Connected to Rust helper');
    } catch (err) {
      console.error('[Main] Failed to start/connect to Rust helper:', err);
    }

    await Promise.all([loadMainWindow(), loadOverlayWindow()]);
    if (settings.showWindowOnLaunch) {
      mainWindow?.show();
    }
  });

  app.on('window-all-closed', () => {});

  app.on('before-quit', () => {
    isQuitting = true;
    rustProcess?.kill();
    tray?.destroy();
  });
}

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  bootstrap();
}
