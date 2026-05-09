import { BrowserWindow, ipcMain, type IpcMainInvokeEvent } from 'electron';
import * as net from 'net';
import { EventEmitter } from 'events';
import type { Command, Event, StatusEvent, UserSettings } from '../shared/types';

type PendingRequest = {
  resolve: (value: Event) => void;
  reject: (reason: Error) => void;
  timer: ReturnType<typeof setTimeout>;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isIpcEvent(value: unknown): value is Event {
  return isRecord(value) && typeof value.type === 'string';
}

class RustClient extends EventEmitter {
  private socket: net.Socket | null = null;
  private buffer = '';
  private pendingRequests = new Map<string, PendingRequest>();

  connect(pipeName: string): Promise<void> {
    return new Promise((resolve, reject) => {
      let settled = false;
      const timeout = setTimeout(() => {
        settled = true;
        reject(new Error(`Timeout connecting to Rust on ${pipeName}`));
      }, 10_000);

      this.socket = net.createConnection(pipeName, () => {
        if (settled) return;
        settled = true;
        clearTimeout(timeout);
        resolve();
      });

      this.socket.on('error', (err) => {
        if (settled) {
          console.error('[RustClient] Socket error:', err);
          return;
        }
        settled = true;
        clearTimeout(timeout);
        reject(err);
      });

      this.socket.on('data', (data) => {
        this.buffer += data.toString('utf-8');
        let idx: number;
        while ((idx = this.buffer.indexOf('\n')) !== -1) {
          const line = this.buffer.slice(0, idx).trim();
          this.buffer = this.buffer.slice(idx + 1);
          if (line) this.handleLine(line);
        }
      });

      this.socket.on('close', () => {
        this.rejectPending(new Error('Rust connection closed'));
        this.emit('disconnected');
      });
    });
  }

  private handleLine(line: string): void {
    let parsed: unknown;
    try {
      parsed = JSON.parse(line) as unknown;
    } catch (err) {
      console.error('[RustClient] Failed to parse event:', err, line);
      return;
    }

    if (!isIpcEvent(parsed)) {
      console.error('[RustClient] Ignoring malformed event:', line);
      return;
    }

    this.emit('event', parsed);

    const pending = this.pendingRequests.get(parsed.type);
    if (!pending) return;

    clearTimeout(pending.timer);
    this.pendingRequests.delete(parsed.type);
    pending.resolve(parsed);
  }

  send(command: Command): void {
    if (!this.socket) throw new Error('Not connected to Rust');
    this.socket.write(`${JSON.stringify(command)}\n`);
  }

  request(command: Command, expectedEventType: Event['type'], timeoutMs = 5_000): Promise<Event> {
    return new Promise((resolve, reject) => {
      if (!this.socket) {
        reject(new Error('Not connected to Rust'));
        return;
      }

      const timer = setTimeout(() => {
        this.pendingRequests.delete(expectedEventType);
        reject(new Error(`Timeout waiting for ${expectedEventType}`));
      }, timeoutMs);

      this.pendingRequests.set(expectedEventType, { resolve, reject, timer });
      this.send(command);
    });
  }

  disconnect(): void {
    this.rejectPending(new Error('Rust client disconnected'));
    this.socket?.end();
    this.socket = null;
  }

  private rejectPending(reason: Error): void {
    for (const pending of this.pendingRequests.values()) {
      clearTimeout(pending.timer);
      pending.reject(reason);
    }
    this.pendingRequests.clear();
  }
}

let rustClient: RustClient | null = null;
let mainWindow: BrowserWindow | null = null;

export function setMainWindow(win: BrowserWindow): void {
  mainWindow = win;
}

export async function connectToRust(pipeName: string): Promise<void> {
  rustClient = new RustClient();

  rustClient.on('event', (event: Event) => {
    mainWindow?.webContents.send('asr:event', event);

    switch (event.type) {
      case 'dictation.started':
        mainWindow?.webContents.send('dictation:started', event);
        break;
      case 'dictation.stopped':
        mainWindow?.webContents.send('dictation:stopped', event);
        break;
      case 'transcript.partial':
        mainWindow?.webContents.send('transcript:partial', event);
        break;
      case 'transcript.final':
        mainWindow?.webContents.send('transcript:final', event);
        break;
      case 'status':
        mainWindow?.webContents.send('status:update', event);
        break;
      case 'error':
        mainWindow?.webContents.send('error:received', event);
        break;
    }
  });

  rustClient.on('disconnected', () => {
    mainWindow?.webContents.send('rust:disconnected');
  });

  await rustClient.connect(pipeName);
}

export function setupIpcHandlers(): void {
  ipcMain.handle('dictation:start', async () => {
    if (!rustClient) throw new Error('Rust not connected');
    rustClient.send({ type: 'dictation.start' });
    return { success: true };
  });

  ipcMain.handle('dictation:stop', async () => {
    if (!rustClient) throw new Error('Rust not connected');
    rustClient.send({ type: 'dictation.stop' });
    return { success: true };
  });

  ipcMain.handle('status:get', async () => getStatus());

  ipcMain.handle(
    'settings:update',
    async (_event: IpcMainInvokeEvent, settings: UserSettings) => {
      if (!rustClient) throw new Error('Rust not connected');
      rustClient.send({ type: 'settings.update', settings });
      return { success: true };
    }
  );

  ipcMain.handle('health-check', async () => {
    const status = await getStatus();
    return {
      status: status.workerHealthy ? 'ok' : 'degraded',
      timestamp: Date.now(),
      isDictating: status.isDictating,
      isModelLoaded: status.isModelLoaded,
      workerHealthy: status.workerHealthy,
    };
  });
}

export function getRustClient(): RustClient | null {
  return rustClient;
}

async function getStatus(): Promise<StatusEvent> {
  if (!rustClient) throw new Error('Rust not connected');
  const response = await rustClient.request({ type: 'status.get' }, 'status', 5_000);
  if (response.type !== 'status') throw new Error(`Expected status, received ${response.type}`);
  return response;
}
