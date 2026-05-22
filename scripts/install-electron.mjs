import { createRequire } from 'node:module';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.join(fileURLToPath(new URL('.', import.meta.url)), '..');
const desktopPkg = path.join(repoRoot, 'apps', 'desktop', 'package.json');
const require = createRequire(desktopPkg);
const electronDir = path.dirname(require.resolve('electron/package.json'));
const pathFile = path.join(electronDir, 'path.txt');

if (fs.existsSync(pathFile)) {
  const electronPath = fs.readFileSync(pathFile, 'utf8').trim();
  if (electronPath && fs.existsSync(electronPath)) {
    process.exit(0);
  }
}

const result = spawnSync(process.execPath, ['install.js'], {
  cwd: electronDir,
  stdio: 'inherit',
});

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
