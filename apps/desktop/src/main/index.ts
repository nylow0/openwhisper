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

  return {
    ...nextSettings,
    model,
    spokenLanguages: normalizeSpokenLanguages(nextSettings.spokenLanguages, model),
    autoDetectLanguage,
  };
}

function loadSettings(): AppSettings {
  return normalizeSettings(loadJson<StoredSettings>(SETTINGS_FILE, {}));
}

function createMainWindow(): void {
  // The app ships its own integrated title bar — no native menu chrome.
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
      sandbox: false,
    },
  });

  // OpenWhisper lives in the tray — closing the window just hides it.
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
      sandbox: false,
    },
  });

  // Float above everything, and never intercept clicks — it is a HUD.
  overlayWindow.setAlwaysOnTop(true, 'screen-saver');
  overlayWindow.setIgnoreMouseEvents(true);

  overlayWindow.on('closed', () => {
    overlayWindow = null;
  });

  setOverlayWindow(overlayWindow);
}

function createTray(): void {
  tray = new Tray(createAppIcon(32));
  tray.setToolTip('OpenWhisper — hold Ctrl + Win to dictate');

  const menu = Menu.buildFromTemplate([
    { label: 'Open OpenWhisper', click: () => showMainWindow() },
    { type: 'separator' },
    { label: 'Hold Ctrl + Win to dictate', enabled: false },
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
  if (process.env.VITE_DEV_SERVER_URL) {
    await mainWindow.loadURL(process.env.VITE_DEV_SERVER_URL);
    mainWindow.webContents.openDevTools({ mode: 'detach' });
    return;
  }
  await mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
}

async function loadOverlayWindow(): Promise<void> {
  if (!overlayWindow) throw new Error('Overlay window has not been created');
  if (process.env.VITE_DEV_SERVER_URL) {
    await overlayWindow.loadURL(`${process.env.VITE_DEV_SERVER_URL}/overlay.html`);
    return;
  }
  await overlayWindow.loadFile(path.join(__dirname, '../renderer/overlay.html'));
}

function findProjectRoot(): string {
  const starts = [
    process.env.OPENWHISPER_PROJECT_ROOT,
    process.cwd(),
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

/** Environment for the Rust helper and its selected ASR worker. */
function packagedAssetPath(...parts: string[]): string {
  return path.join(process.resourcesPath, ...parts);
}

function rustEnv(): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    OPENWHISPER_ASR_MODEL: settings.model,
    OPENWHISPER_ASR_DEVICE: settings.device,
    OPENWHISPER_ASR_LANGUAGES: settings.spokenLanguages.join(','),
    OPENWHISPER_ASR_AUTO_DETECT_LANGUAGE: settings.autoDetectLanguage ? '1' : '0',
  };

  if (!app.isPackaged) return env;

  return {
    ...env,
    OPENWHISPER_WHISPERCPP_CONFIG: packagedAssetPath('config', 'whispercpp-profiles.json'),
    OPENWHISPER_MODEL_DIR: packagedAssetPath('models'),
    OPENWHISPER_WHISPERCPP_CPU_EXE: packagedAssetPath(
      'whispercpp',
      'cpu',
      'whisper-cli.exe'
    ),
    OPENWHISPER_WHISPERCPP_GPU_EXE: packagedAssetPath(
      'whispercpp',
      'gpu',
      'whisper-cli.exe'
    ),
  };
}

function startRustHelper(): Promise<string> {
  return new Promise((resolve, reject) => {
    const electronPid = process.pid;
    const pipeName = `\\\\.\\pipe\\OpenWhisper-${electronPid}`;
    const projectRoot = app.isPackaged ? process.resourcesPath : findProjectRoot();
    const binaryPaths = [
      path.join(
        projectRoot,
        'crates',
        'openwhisper-native',
        'target',
        'x86_64-pc-windows-msvc',
        'debug',
        'openwhisper-native.exe'
      ),
      path.join(
        projectRoot,
        'crates',
        'openwhisper-native',
        'target',
        'debug',
        'openwhisper-native.exe'
      ),
    ];
    const rustCwd = path.join(projectRoot, 'crates', 'openwhisper-native');
    const binaryPath = binaryPaths.find((candidate) => fs.existsSync(candidate));
    const packagedBinaryPath = packagedAssetPath('native', 'openwhisper-native.exe');
    let settled = false;

    const settleReady = (): void => {
      if (settled) return;
      settled = true;
      resolve(pipeName);
    };

    if (app.isPackaged && !fs.existsSync(packagedBinaryPath)) {
      reject(new Error(`Bundled Rust helper is missing: ${packagedBinaryPath}`));
      return;
    }

    rustProcess = app.isPackaged
      ? spawn(packagedBinaryPath, ['--pipe-pid', String(electronPid)], {
          cwd: projectRoot,
          stdio: ['ignore', 'pipe', 'pipe'],
          env: rustEnv(),
        })
      : binaryPath
      ? spawn(binaryPath, ['--pipe-pid', String(electronPid)], {
          cwd: projectRoot,
          stdio: ['ignore', 'pipe', 'pipe'],
          env: rustEnv(),
        })
      : spawn(
          'cargo',
          ['run', '--target', 'x86_64-pc-windows-msvc', '--', '--pipe-pid', String(electronPid)],
          {
            cwd: rustCwd,
            stdio: ['ignore', 'pipe', 'pipe'],
            env: rustEnv(),
          }
        );

    const handleOutput = (prefix: string, data: Buffer): void => {
      const text = data.toString('utf-8').trim();
      if (text) console.log(`${prefix} ${text}`);
      if (text.includes('Starting IPC server')) settleReady();
    };

    rustProcess.stdout?.on('data', (data: Buffer) => handleOutput('[Rust]', data));
    rustProcess.stderr?.on('data', (data: Buffer) => handleOutput('[Rust stderr]', data));

    rustProcess.on('error', (err) => {
      if (settled) {
        console.error('[Rust] Process error:', err);
        return;
      }
      settled = true;
      reject(err);
    });

    rustProcess.on('exit', (code) => {
      console.log(`[Rust] Process exited with code ${code}`);
    });

    setTimeout(settleReady, 5_000);
  });
}

/** Restarts the Rust helper and selected ASR worker so settings changes take effect. */
async function restartEngine(): Promise<{ ok: boolean; error?: string }> {
  disconnectRust();
  rustProcess?.kill();
  rustProcess = null;
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

  // Tray application — keep running even when every window is closed.
  app.on('window-all-closed', () => {});

  app.on('before-quit', () => {
    isQuitting = true;
    rustProcess?.kill();
    tray?.destroy();
  });
}

// Single-instance: a second launch focuses the existing window instead of
// starting another tray icon.
if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  bootstrap();
}
