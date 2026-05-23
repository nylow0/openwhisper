import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const resourcesDir = process.argv[2]
  ? path.resolve(process.argv[2])
  : path.resolve(scriptDir, '..', 'release', 'win-unpacked', 'resources');
const appDir = path.resolve(resourcesDir, '..');

const required = [
  ['native', 'openwhisper-native.exe'],
  ['native', 'ow-asr-rs.exe'],
  ['config', 'whispercpp-profiles.json'],
  ['models', 'ggml-medium.en-q8_0.bin'],
  ['models', 'ggml-large-v3-turbo-q8_0.bin'],
  ['whispercpp', 'cpu', 'whisper-cli.exe'],
  ['whispercpp', 'cpu', 'ggml.dll'],
  ['whispercpp', 'gpu', 'whisper-cli.exe'],
  ['whispercpp', 'gpu', 'cublas64_12.dll'],
  ['whispercpp', 'gpu', 'ggml-cuda.dll'],
];
const missing = required
  .map((parts) => path.join(resourcesDir, ...parts))
  .filter((asset) => !fs.existsSync(asset));

if (missing.length > 0) {
  throw new Error(`Packaged ASR runtime assets are missing:\n${missing.join('\n')}`);
}

const mainProcessPath = path.join(appDir, 'resources', 'app.asar.unpacked', 'dist', 'main', 'index.js');
const unpackedMainProcessPath = path.join(appDir, 'resources', 'app', 'dist', 'main', 'index.js');
const distMainProcessPath = path.join(scriptDir, '..', 'dist', 'main', 'index.js');
const mainProcessCandidates = [mainProcessPath, unpackedMainProcessPath, distMainProcessPath];
const mainProcess = mainProcessCandidates.find((candidate) => fs.existsSync(candidate));

if (!mainProcess) {
  throw new Error(
    `Could not find built main process for package smoke check:\n${mainProcessCandidates.join('\n')}`
  );
}

const mainSource = fs.readFileSync(mainProcess, 'utf8');
const resolverStart = mainSource.indexOf('function resolveRustHelperLaunch');
const resolverEnd = mainSource.indexOf('function getRustHelperLaunch', resolverStart);
const resolverSource = resolverStart >= 0 && resolverEnd >= 0
  ? mainSource.slice(resolverStart, resolverEnd)
  : '';
const packagedBranch = resolverSource.indexOf('app.isPackaged');
const projectRootLookup = resolverSource.indexOf('findProjectRoot()');

if (resolverSource === '' || packagedBranch < 0 || projectRootLookup < 0) {
  throw new Error('Package smoke check could not inspect resolveRustHelperLaunch.');
}

if (projectRootLookup < packagedBranch) {
  throw new Error(
    'Packaged launch resolves the development project root before checking bundled resources.'
  );
}

console.log(`Packaged Rust ASR runtime smoke check passed: ${resourcesDir}`);
