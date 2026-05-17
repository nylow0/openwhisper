import { app, BrowserWindow, Menu, Tray } from 'electron';
import * as fs from 'fs';
import * as path from 'path';
import { spawn, type ChildProcess } from 'child_process';
import { fileURLToPath } from 'url';
import { connectToRust, setMainWindow, setOverlayWindow, setupIpcHandlers } from './ipc.js';
import { createAppIcon } from './tray-icon.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let mainWindow: BrowserWindow | null = null;
let overlayWindow: BrowserWindow | null = null;
let tray: Tray | null = null;
let rustProcess: ChildProcess | null = null;
let isQuitting = false;

function createMainWindow(): void {
  // The app ships its own integrated title bar — no native menu chrome.
  Menu.setApplicationMenu(null);

  mainWindow = new BrowserWindow({
    width: 760,
    height: 820,
    minWidth: 560,
    minHeight: 620,
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
        })
      : spawn('cargo', ['run', '--target', 'x86_64-pc-windows-msvc', '--', '--pipe-pid', String(electronPid)], {
          cwd: rustCwd,
          stdio: ['ignore', 'pipe', 'pipe'],
        });

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

app.whenReady().then(async () => {
  createMainWindow();
  createOverlayWindow();
  createTray();
  setupIpcHandlers();

  try {
    const pipeName = await startRustHelper();
    await connectToRust(pipeName);
    console.log('[Main] Connected to Rust helper');
  } catch (err) {
    console.error('[Main] Failed to start/connect to Rust helper:', err);
  }

  await Promise.all([loadMainWindow(), loadOverlayWindow()]);
});

// Tray application — keep running even when every window is closed.
app.on('window-all-closed', () => {});

app.on('before-quit', () => {
  isQuitting = true;
  rustProcess?.kill();
  tray?.destroy();
});
