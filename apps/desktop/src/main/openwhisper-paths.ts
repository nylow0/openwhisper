import * as fs from 'fs';
import * as path from 'path';

type PathsManifest = {
  windowsMsvcTarget: string;
  linuxGnuTarget: string;
  debugTargetSubdirs: string[][];
  executables: Record<string, string>;
  executables_linux: Record<string, string>;
};

let cachedManifest: PathsManifest | null = null;

export function loadOpenwhisperPathsManifest(workspaceRoot: string): PathsManifest {
  if (cachedManifest) return cachedManifest;
  const manifestPath = path.join(workspaceRoot, 'config', 'openwhisper-paths.json');
  cachedManifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8')) as PathsManifest;
  return cachedManifest;
}

function platformExecutables(manifest: PathsManifest): Record<string, string> {
  return process.platform === 'win32' ? manifest.executables : manifest.executables_linux;
}

export function debugNativeBinaryCandidates(
  workspaceRoot: string,
  crateFolder: string,
  executableKey: string
): string[] {
  const manifest = loadOpenwhisperPathsManifest(workspaceRoot);
  const exeName = platformExecutables(manifest)[executableKey];
  if (!exeName) return [];
  const crateRoot = path.join(workspaceRoot, 'crates', crateFolder);
  return manifest.debugTargetSubdirs.map((segments) => path.join(crateRoot, ...segments, exeName));
}

export function nativeTarget(workspaceRoot: string): string {
  const manifest = loadOpenwhisperPathsManifest(workspaceRoot);
  return process.platform === 'win32' ? manifest.windowsMsvcTarget : manifest.linuxGnuTarget;
}

export function clearPathsManifestCache(): void {
  cachedManifest = null;
}
