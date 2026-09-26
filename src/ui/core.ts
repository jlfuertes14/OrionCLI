import { createRequire } from 'module';
import path from 'path';
import { fileURLToPath } from 'url';
import fs from 'fs';
import { platform, arch } from 'process';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

function getBinaryName(): string | null {
  const p = platform;
  const a = arch;

  if (p === 'win32' && a === 'x64')   return 'orion-core.win32-x64-msvc.node';
  if (p === 'linux' && a === 'x64')   return 'orion-core.linux-x64-gnu.node';
  if (p === 'linux' && a === 'arm64') return 'orion-core.linux-arm64-gnu.node';
  if (p === 'darwin' && a === 'x64')  return 'orion-core.darwin-x64.node';
  if (p === 'darwin' && a === 'arm64')return 'orion-core.darwin-arm64.node';

  return null;
}

function loadNativeBinding(): any {
  const binaryName = getBinaryName();

  const candidates: string[] = [
    // 1. One level up (when bundled in dist/cli.js -> package root index.js)
    path.resolve(__dirname, '../index.js'),
    // 2. Two levels up (when running from src/ui/ in dev)
    path.resolve(__dirname, '../../index.js'),
  ];

  if (binaryName) {
    candidates.push(
      path.resolve(__dirname, '..', binaryName),
      path.resolve(__dirname, '../..', binaryName),
      path.resolve(__dirname, binaryName),
      path.resolve(process.cwd(), binaryName)
    );
  }

  // Walk up from __dirname to search for binaryName or index.js
  let curr = __dirname;
  for (let depth = 0; depth < 5; depth++) {
    if (binaryName) {
      candidates.push(path.join(curr, binaryName));
    }
    candidates.push(path.join(curr, 'index.js'));
    const parent = path.dirname(curr);
    if (parent === curr) break;
    curr = parent;
  }

  for (const candidate of candidates) {
    if (!fs.existsSync(candidate)) continue;
    try {
      const raw = require(candidate);
      const binding =
        raw?.napiListDirectory ? raw :
        raw?.default?.napiListDirectory ? raw.default :
        raw;

      if (binding && typeof binding.napiListDirectory === 'function') {
        return binding;
      }
    } catch {
      // Continue searching next candidate
    }
  }

  return null;
}

const nativeBinding = loadNativeBinding();

export interface DiffLine {
  tag: string;
  text: string;
}

export const core = {
  version(): string {
    return nativeBinding?.orionCoreVersion ? nativeBinding.orionCoreVersion() : '2.0.0';
  },

  async readFile(filePath: string): Promise<string> {
    if (nativeBinding?.napiReadFile) {
      return await nativeBinding.napiReadFile(filePath);
    }
    throw new Error('Native binding napiReadFile unavailable');
  },

  async writeFile(filePath: string, content: string): Promise<string> {
    if (nativeBinding?.napiWriteFile) {
      return await nativeBinding.napiWriteFile(filePath, content);
    }
    throw new Error('Native binding napiWriteFile unavailable');
  },

  async listDirectory(dirPath: string): Promise<string> {
    if (nativeBinding?.napiListDirectory) {
      return await nativeBinding.napiListDirectory(dirPath);
    }
    throw new Error('Native binding napiListDirectory unavailable');
  },

  async gitStatus(): Promise<string> {
    if (nativeBinding?.napiGitStatus) {
      return await nativeBinding.napiGitStatus();
    }
    throw new Error('Native binding napiGitStatus unavailable');
  },

  async gitDiff(): Promise<string> {
    if (nativeBinding?.napiGitDiff) {
      return await nativeBinding.napiGitDiff();
    }
    throw new Error('Native binding napiGitDiff unavailable');
  },

  async grepSearch(query: string, searchPath?: string): Promise<string> {
    if (nativeBinding?.napiGrepSearch) {
      return await nativeBinding.napiGrepSearch(query, searchPath);
    }
    throw new Error('Native binding napiGrepSearch unavailable');
  },

  async runCommand(command: string): Promise<string> {
    if (nativeBinding?.napiRunCommand) {
      return await nativeBinding.napiRunCommand(command);
    }
    throw new Error('Native binding napiRunCommand unavailable');
  },

  async executeTool(name: string, args: Record<string, any>): Promise<string> {
    if (nativeBinding?.napiExecuteTool) {
      return await nativeBinding.napiExecuteTool(name, JSON.stringify(args));
    }
    throw new Error(`Native binding napiExecuteTool unavailable for ${name}`);
  },

  computeDiff(oldText: string, newText: string): DiffLine[] {
    if (nativeBinding?.napiComputeDiff) {
      return nativeBinding.napiComputeDiff(oldText, newText);
    }
    return [];
  },

  getToolSchemas(): any[] {
    if (nativeBinding?.napiGetToolSchemas) {
      try {
        return JSON.parse(nativeBinding.napiGetToolSchemas());
      } catch {
        return [];
      }
    }
    return [];
  },

  async listSessions(limit = 25): Promise<any[]> {
    if (nativeBinding?.napiListSessions) {
      return nativeBinding.napiListSessions(limit);
    }
    return [];
  },

  async deleteSession(sessionId: string): Promise<boolean> {
    if (nativeBinding?.napiDeleteSession) {
      return nativeBinding.napiDeleteSession(sessionId);
    }
    return false;
  }
};
