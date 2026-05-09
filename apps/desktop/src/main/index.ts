import { app, BrowserWindow } from 'electron';
import * as fs from 'fs';
import * as path from 'path';
import { spawn, type ChildProcess } from 'child_process';
import { fileURLToPath } from 'url';
import { connectToRust, setMainWindow, setupIpcHandlers } from './ipc.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let mainWindow: BrowserWindow | null = null;
let rustProcess: ChildProcess | null = null;

async function createWindow(): Promise<void> {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  setMainWindow(mainWindow);
  setupIpcHandlers();

  if (process.env.VITE_DEV_SERVER_URL) {
    await mainWindow.loadURL(process.env.VITE_DEV_SERVER_URL);
    mainWindow.webContents.openDevTools();
    return;
  }

  await mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));
}

function startRustHelper(): Promise<string> {
  return new Promise((resolve, reject) => {
    const electronPid = process.pid;
    const pipeName = `\\\\.\\pipe\\OpenWhisper-${electronPid}`;
    const projectRoot = path.resolve(__dirname, '..', '..', '..', '..');
    const binaryPath = path.join(
      projectRoot,
      'crates',
      'openwhisper-native',
      'target',
      'debug',
      'openwhisper-native.exe'
    );
    const rustCwd = path.join(projectRoot, 'crates', 'openwhisper-native');
    const useBinary = fs.existsSync(binaryPath);
    let settled = false;

    const settleReady = (): void => {
      if (settled) return;
      settled = true;
      resolve(pipeName);
    };

    rustProcess = useBinary
      ? spawn(binaryPath, ['--pipe-pid', String(electronPid)], {
          cwd: projectRoot,
          stdio: ['ignore', 'pipe', 'pipe'],
        })
      : spawn('cargo', ['run', '--', '--pipe-pid', String(electronPid)], {
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
  await createWindow();

  try {
    const pipeName = await startRustHelper();
    await connectToRust(pipeName);
    console.log('[Main] Connected to Rust helper');
  } catch (err) {
    console.error('[Main] Failed to start/connect to Rust helper:', err);
  }
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

app.on('before-quit', () => {
  rustProcess?.kill();
});
