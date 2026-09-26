/**
 * Stdin raw sequence tracker for terminal keyboard events.
 * Correctly distinguishes Backspace (\x7f, \x08, \b) from Forward Delete (\x1b[3~),
 * and completely strips mouse tracking / scrolling escape sequences from typing.
 */

let lastRawInput = '';

if (typeof process !== 'undefined' && process.stdin) {
  // Listen for raw input chunks on stdin without putting stream in flowing mode
  const originalEmit = process.stdin.emit.bind(process.stdin);
  process.stdin.emit = function (event: string | symbol, ...args: any[]): boolean {
    if (event === 'data' && args[0]) {
      const chunk = args[0];
      lastRawInput = Buffer.isBuffer(chunk) ? chunk.toString('utf-8') : String(chunk);
    }
    return originalEmit(event, ...args);
  };
}

export function getLastRawInput(): string {
  return lastRawInput;
}

export function setLastRawInput(raw: string): void {
  lastRawInput = raw;
}

/**
 * Strips all SGR, X11, and terminal mouse reporting codes (e.g. \x1b[<0;53;22m or [<0;53;22m).
 * Prevents mouse clicks from leaking coordinates into input fields.
 */
export function stripMouseSequences(text: string): string {
  return text
    .replace(/\x1b?\[<[\d;]*[Mm]?/g, '')
    .replace(/\x1b?\[M[\s\S]{0,3}/g, '')
    .replace(/\[<[\d;]*[Mm]?/g, '');
}

export function isMouseSequence(text: string): boolean {
  return (
    text.includes('[<') ||
    text.includes('\x1b[<') ||
    text.includes('\x1b[M') ||
    /^\[<\d+;\d+;\d+[Mm]$/.test(text)
  );
}

/**
 * Returns true if the key pressed is the physical/forward Delete key (\x1b[3~).
 */
export function isTerminalForwardDelete(key: { delete?: boolean; backspace?: boolean }): boolean {
  if (!key.delete) return false;
  // If the raw sequence contains the standard VT100 / xterm delete key escape code [3~
  return lastRawInput.includes('[3~');
}

/**
 * Returns true if the key pressed is Backspace (ASCII 127 \x7f, ASCII 8 \x08, \b, or Ctrl+H),
 * even if Ink's parser tagged it as key.delete (which happens for \x7f).
 */
export function isTerminalBackspace(
  key: { delete?: boolean; backspace?: boolean; ctrl?: boolean },
  input?: string
): boolean {
  if (key.backspace) return true;
  if (key.ctrl && (input === 'h' || input === '\x08' || input === '\b')) return true;
  // If ink flagged it as delete, but it's NOT the forward-delete escape sequence, it is Backspace!
  if (key.delete && !lastRawInput.includes('[3~')) return true;
  return false;
}
