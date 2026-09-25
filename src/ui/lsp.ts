import { spawn, ChildProcess } from 'child_process';
import path from 'path';
import fs from 'fs';
import { pathToFileURL, fileURLToPath } from 'url';

export interface LspLocation {
  uri: string;
  filePath: string;
  line: number;
  character: number;
  snippet?: string;
}

export interface LspSymbol {
  name: string;
  kind: string;
  range: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
}

class LspClient {
  private process: ChildProcess | null = null;
  private messageId = 1;
  private pendingRequests = new Map<number, { resolve: (val: any) => void; reject: (err: any) => void }>();
  private buffer = '';
  private initialized = false;
  private rootUri: string;
  private command: string;
  private args: string[];

  constructor(command: string, args: string[], rootDir: string) {
    this.command = command;
    this.args = args;
    this.rootUri = pathToFileURL(rootDir).href;
  }

  public async start(): Promise<boolean> {
    try {
      this.process = spawn(this.command, this.args, {
        stdio: ['pipe', 'pipe', 'ignore'],
        shell: process.platform === 'win32',
      });

      this.process.stdout?.on('data', (chunk: Buffer) => {
        this.buffer += chunk.toString('utf-8');
        this.processBuffer();
      });

      this.process.on('error', () => {
        this.process = null;
      });

      // Send initialize request
      const initResponse = await this.sendRequest('initialize', {
        processId: process.pid,
        rootUri: this.rootUri,
        capabilities: {
          textDocument: {
            definition: { dynamicRegistration: true },
            references: { dynamicRegistration: true },
            documentSymbol: { hierarchicalDocumentSymbolSupport: true },
          },
        },
      });

      if (initResponse) {
        this.sendNotification('initialized', {});
        this.initialized = true;
        return true;
      }
      return false;
    } catch {
      this.process = null;
      return false;
    }
  }

  public isRunning(): boolean {
    return this.process !== null && this.initialized;
  }

  public sendNotification(method: string, params: any) {
    if (!this.process?.stdin?.writable) return;
    const msg = JSON.stringify({ jsonrpc: '2.0', method, params });
    const full = `Content-Length: ${Buffer.byteLength(msg, 'utf-8')}\r\n\r\n${msg}`;
    this.process.stdin.write(full);
  }

  public sendRequest(method: string, params: any): Promise<any> {
    return new Promise((resolve, reject) => {
      if (!this.process?.stdin?.writable) {
        return reject(new Error('LSP server process not running or stdin closed'));
      }
      const id = this.messageId++;
      this.pendingRequests.set(id, { resolve, reject });

      const msg = JSON.stringify({ jsonrpc: '2.0', id, method, params });
      const full = `Content-Length: ${Buffer.byteLength(msg, 'utf-8')}\r\n\r\n${msg}`;
      this.process.stdin.write(full);

      // Timeout after 6 seconds
      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error(`LSP request ${method} timed out`));
        }
      }, 6000);
    });
  }

  private processBuffer() {
    while (true) {
      const headerEnd = this.buffer.indexOf('\r\n\r\n');
      if (headerEnd === -1) break;

      const headers = this.buffer.slice(0, headerEnd);
      const match = headers.match(/Content-Length:\s*(\d+)/i);
      if (!match) {
        this.buffer = this.buffer.slice(headerEnd + 4);
        continue;
      }

      const length = parseInt(match[1], 10);
      const contentStart = headerEnd + 4;
      if (this.buffer.length < contentStart + length) {
        // Full message not yet received
        break;
      }

      const bodyStr = this.buffer.slice(contentStart, contentStart + length);
      this.buffer = this.buffer.slice(contentStart + length);

      try {
        const parsed = JSON.parse(bodyStr);
        if (parsed.id !== undefined && this.pendingRequests.has(parsed.id)) {
          const { resolve, reject } = this.pendingRequests.get(parsed.id)!;
          this.pendingRequests.delete(parsed.id);
          if (parsed.error) {
            reject(new Error(parsed.error.message || 'LSP error'));
          } else {
            resolve(parsed.result);
          }
        }
      } catch {}
    }
  }

  public stop() {
    try {
      this.process?.kill();
    } catch {}
    this.process = null;
    this.initialized = false;
  }
}

// Global server instances per language
const activeLspClients = new Map<string, LspClient>();

function getLspServerForFile(filePath: string, workspaceDir: string): { lang: string; client: LspClient } | null {
  const ext = path.extname(filePath).toLowerCase();

  let lang = '';
  let cmd = '';
  let args: string[] = [];

  if (['.ts', '.tsx', '.js', '.jsx'].includes(ext)) {
    lang = 'typescript';
    cmd = 'npx';
    args = ['typescript-language-server', '--stdio'];
  } else if (ext === '.rs') {
    lang = 'rust';
    cmd = 'rust-analyzer';
    args = [];
  } else if (ext === '.go') {
    lang = 'go';
    cmd = 'gopls';
    args = [];
  } else if (ext === '.py') {
    lang = 'python';
    cmd = 'pyright-langserver';
    args = ['--stdio'];
  }

  if (!lang) return null;

  if (!activeLspClients.has(lang)) {
    activeLspClients.set(lang, new LspClient(cmd, args, workspaceDir));
  }

  return { lang, client: activeLspClients.get(lang)! };
}

/**
 * OpenCode Strategy: Query local LSP server for exact symbol definition
 */
export async function lspFindDefinition(
  filePath: string,
  line: number,
  character: number,
  workspaceDir: string = process.cwd()
): Promise<string> {
  const target = getLspServerForFile(filePath, workspaceDir);
  if (!target) {
    return `No LSP server configured for file extension ${path.extname(filePath)}. Use grep_search instead.`;
  }

  const { client } = target;
  if (!client.isRunning()) {
    const started = await client.start();
    if (!started) {
      return `LSP server for ${target.lang} could not be started. Falling back to native ripgrep search.`;
    }
  }

  const fullPath = path.isAbsolute(filePath) ? filePath : path.resolve(workspaceDir, filePath);
  const docUri = pathToFileURL(fullPath).href;

  try {
    const result = await client.sendRequest('textDocument/definition', {
      textDocument: { uri: docUri },
      position: { line: Math.max(0, line - 1), character: Math.max(0, character - 1) },
    });

    if (!result || (Array.isArray(result) && result.length === 0)) {
      return `No LSP definition found for symbol at ${filePath}:${line}:${character}.`;
    }

    const locations = Array.isArray(result) ? result : [result];
    const formatted = locations.map((loc: any) => {
      const locUri = loc.uri || loc.targetUri;
      const targetRange = loc.range || loc.targetSelectionRange || loc.targetRange;
      const targetFile = fileURLToPath(locUri);
      const targetLine = (targetRange?.start?.line ?? 0) + 1;
      const targetCol = (targetRange?.start?.character ?? 0) + 1;

      // Extract line preview if file is accessible
      let snippet = '';
      try {
        const fileContent = fs.readFileSync(targetFile, 'utf-8');
        const lines = fileContent.split('\n');
        snippet = lines[targetLine - 1]?.trim() || '';
      } catch {}

      return `→ ${path.relative(workspaceDir, targetFile)}:${targetLine}:${targetCol} ${snippet ? `[${snippet}]` : ''}`;
    });

    return `Compiler Definition (LSP):\n${formatted.join('\n')}`;
  } catch (err: any) {
    return `LSP query failed: ${err.message}. Use grep_search as fallback.`;
  }
}

/**
 * OpenCode Strategy: Query local LSP server for exact symbol references
 */
export async function lspFindReferences(
  filePath: string,
  line: number,
  character: number,
  workspaceDir: string = process.cwd()
): Promise<string> {
  const target = getLspServerForFile(filePath, workspaceDir);
  if (!target) {
    return `No LSP server configured for file extension ${path.extname(filePath)}. Use grep_search instead.`;
  }

  const { client } = target;
  if (!client.isRunning()) {
    const started = await client.start();
    if (!started) {
      return `LSP server for ${target.lang} could not be started. Falling back to native ripgrep search.`;
    }
  }

  const fullPath = path.isAbsolute(filePath) ? filePath : path.resolve(workspaceDir, filePath);
  const docUri = pathToFileURL(fullPath).href;

  try {
    const result = await client.sendRequest('textDocument/references', {
      textDocument: { uri: docUri },
      position: { line: Math.max(0, line - 1), character: Math.max(0, character - 1) },
      context: { includeDeclaration: true },
    });

    if (!result || (Array.isArray(result) && result.length === 0)) {
      return `No LSP references found for symbol at ${filePath}:${line}:${character}.`;
    }

    const locations = Array.isArray(result) ? result : [result];
    const formatted = locations.slice(0, 30).map((loc: any) => {
      const locUri = loc.uri || loc.targetUri;
      const targetRange = loc.range;
      const targetFile = fileURLToPath(locUri);
      const targetLine = (targetRange?.start?.line ?? 0) + 1;
      const targetCol = (targetRange?.start?.character ?? 0) + 1;

      let snippet = '';
      try {
        const fileContent = fs.readFileSync(targetFile, 'utf-8');
        const lines = fileContent.split('\n');
        snippet = lines[targetLine - 1]?.trim() || '';
      } catch {}

      return `• ${path.relative(workspaceDir, targetFile)}:${targetLine}:${targetCol} ${snippet ? `[${snippet}]` : ''}`;
    });

    return `Compiler References (LSP - ${locations.length} found):\n${formatted.join('\n')}`;
  } catch (err: any) {
    return `LSP references query failed: ${err.message}. Use grep_search as fallback.`;
  }
}
