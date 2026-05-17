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
import type { AppSettings } from '../shared/types';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SETTINGS_FILE = 'settings.json';
const DEFAULT_SETTINGS: AppSettings = {
  model: 'medium_en_q8',
  device: 'auto',
  launchAtLogin: false,
  showWindowOnLaunch: true,
};

let mainWindow: BrowserWindow | null = null;
let overlayWindow: BrowserWindow | null = null;
let tray: Tray | null = null;
let rustProcess: ChildProcess | null = null;
let isQuitting = false;
let settings: AppSettings = DEFAULT_SETTINGS;

function loadSettings(): AppSettings {
  return { ...DEFAULT_SETTINGS, ...loadJson<Partial<AppSettings>>(SETTINGS_FILE, {}) };
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
      const hasAsrPackage = fs.existsSync(path.join(current, 'asr', 'pyproject.toml'));
      if (hasNativeCrate && hasAsrPackage) return current;

      const parent = path.dirname(current);
      if (parent === current) break;
      current = parent;
    }
  }

  throw new Error('Could not find OpenWhisper project root from packaged app location');
}

/** Environment for the Rust helper — carries ASR model/device down to Python. */
function rustEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    OPENWHISPER_ASR_MODEL: settings.model,
    OPENWHISPER_ASR_DEVICE: settings.device,
  };
}

function startRustHelper(): Promise<string> {
  return new Promise((resolve, reject) => {
    const electronPid = process.pid;
    const pipeName = `\\\\.\\pipe\\OpenWhisper-${electronPid}`;
    const projectRoot = findProjectRoot();
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
    let settled = false;

    const settleReady = (): void => {
      if (settled) return;
      settled = true;
      resolve(pipeName);
    };

    rustProcess = binaryPath
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

/** Restarts the Rust helper + Python worker so settings changes take effect. */
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
    settings = { ...settings, ...partial };
    saveJson(SETTINGS_FILE, settings);
    if ('launchAtLogin' in partial) {
      app.setLoginItemSettings({ openAtLogin: settings.launchAtLogin });
    }
    return settings;
  });

  ipcMain.handle('engine:restart', () => restartEngine());
}

function bootstrap(): void {
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
