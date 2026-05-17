import { app } from 'electron';
import * as fs from 'fs';
import * as path from 'path';

// Tiny JSON document store under the OS per-user app-data directory.
// Used for persisting settings and transcript history across sessions.

function storePath(name: string): string {
  return path.join(app.getPath('userData'), name);
}

export function loadJson<T>(name: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(storePath(name), 'utf-8')) as T;
  } catch {
    return fallback;
  }
}

export function saveJson(name: string, data: unknown): void {
  try {
    fs.writeFileSync(storePath(name), JSON.stringify(data, null, 2), 'utf-8');
  } catch (err) {
    console.error(`[store] Failed to save ${name}:`, err);
  }
}
