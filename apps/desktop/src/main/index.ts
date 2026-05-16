import { app, BrowserWindow, globalShortcut } from 'electron';
import * as fs from 'fs';
import * as path from 'path';
import { spawn, type ChildProcess } from 'child_process';
import { fileURLToPath } from 'url';
import { connectToRust, setMainWindow, setupIpcHandlers } from './ipc.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let mainWindow: BrowserWindow | null = null;
let rustProcess: ChildProcess | null = null;
const DICTATION_HOTKEY = 'CommandOrControl+Alt+Space';

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
    },
  });

  setMainWindow(mainWindow);
  setupIpcHandlers();
}

async function loadMainWindow(): Promise<void> {
  if (!mainWindow) throw new Error('Main window has not been created');
  if (process.env.VITE_DEV_SERVER_URL) {
    await mainWindow.loadURL(process.env.VITE_DEV_SERVER_URL);
    mainWindow.webContents.openDevTools();
    return;
  }

  await mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
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

function registerGlobalShortcuts(): void {
  const registered = globalShortcut.register(DICTATION_HOTKEY, () => {
    mainWindow?.webContents.send('hotkey:toggle-dictation');
  });

  if (!registered) {
    console.warn(`[Main] Failed to register global shortcut ${DICTATION_HOTKEY}`);
  }
}

app.whenReady().then(async () => {
  createWindow();
  registerGlobalShortcuts();

  try {
    const pipeName = await startRustHelper();
    await connectToRust(pipeName);
    console.log('[Main] Connected to Rust helper');
  } catch (err) {
    console.error('[Main] Failed to start/connect to Rust helper:', err);
  }

  await loadMainWindow();
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

app.on('before-quit', () => {
  globalShortcut.unregisterAll();
  rustProcess?.kill();
});
