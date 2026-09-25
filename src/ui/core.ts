import { createRequire } from 'module';
import path from 'path';
import { fileURLToPath } from 'url';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Load native addon built by napi-rs
let nativeBinding: any = null;

try {
  // First try the root compiled index.js / index.node
  nativeBinding = require('../../index.js');
} catch (e1) {
  try {
    nativeBinding = require('../../orion-core.win32-x64-msvc.node');
  } catch (e2) {
    try {
      nativeBinding = require('../index.node');
    } catch (e3) {
      // Will be populated once napi build finishes
      // console.warn('Native binary not yet loaded:', e1);
    }
  }
}

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
