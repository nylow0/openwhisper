import * as fs from 'fs';
import * as path from 'path';

type PathsManifest = {
  windowsMsvcTarget: string;
  debugTargetSubdirs: string[][];
  executables: Record<string, string>;
};

let cachedManifest: PathsManifest | null = null;

export function loadOpenwhisperPathsManifest(workspaceRoot: string): PathsManifest {
  if (cachedManifest) return cachedManifest;
  const manifestPath = path.join(workspaceRoot, 'config', 'openwhisper-paths.json');
  cachedManifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as PathsManifest;
  return cachedManifest;
}

export function debugNativeBinaryCandidates(
  workspaceRoot: string,
  crateFolder: string,
  executableKey: keyof PathsManifest['executables']
): string[] {
  const manifest = loadOpenwhisperPathsManifest(workspaceRoot);
  const exeName = manifest.executables[executableKey];
  const crateRoot = path.join(workspaceRoot, 'crates', crateFolder);
  return manifest.debugTargetSubdirs.map((segments) => path.join(crateRoot, ...segments, exeName));
}

export function windowsMsvcTarget(workspaceRoot: string): string {
  return loadOpenwhisperPathsManifest(workspaceRoot).windowsMsvcTarget;
}

export function clearPathsManifestCache(): void {
  cachedManifest = null;
}
