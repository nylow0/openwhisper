import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const resourcesDir = process.argv[2]
  ? path.resolve(process.argv[2])
  : path.resolve(scriptDir, '..', 'release', 'win-unpacked', 'resources');

const required = [
  ['native', 'openwhisper-native.exe'],
  ['native', 'ow-asr-rs.exe'],
  ['asr', 'config', 'whispercpp-profiles.json'],
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

const forbidden = [
  ['asr', 'pyproject.toml'],
  ['asr', '.venv'],
];
const bundledPythonPaths = forbidden
  .map((parts) => path.join(resourcesDir, ...parts))
  .filter((asset) => fs.existsSync(asset));

if (bundledPythonPaths.length > 0) {
  throw new Error(`Packaged ASR runtime still contains Python assets:\n${bundledPythonPaths.join('\n')}`);
}

console.log(`Packaged Rust ASR runtime smoke check passed: ${resourcesDir}`);
