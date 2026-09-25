import { execSync, spawnSync } from 'child_process';

/**
 * Get clean text from system clipboard (Windows PowerShell, macOS pbpaste, Linux xclip/xsel)
 */
export function getClipboardText(): string {
  try {
    if (process.platform === 'win32') {
      const out = execSync('powershell.exe -NoProfile -Command "Get-Clipboard"', {
        encoding: 'utf-8',
        timeout: 1500,
        stdio: ['ignore', 'pipe', 'ignore'],
      });
      // Replace CRLF / newlines with spaces for single-line prompt input
      return out.replace(/\r\n/g, ' ').replace(/\n/g, ' ').replace(/\r/g, ' ').trim();
    } else if (process.platform === 'darwin') {
      const out = execSync('pbpaste', {
        encoding: 'utf-8',
        timeout: 1500,
        stdio: ['ignore', 'pipe', 'ignore'],
      });
      return out.replace(/\r\n/g, ' ').replace(/\n/g, ' ').replace(/\r/g, ' ').trim();
    } else {
      const out = execSync('xclip -selection clipboard -o 2>/dev/null || xsel --clipboard --output 2>/dev/null', {
        encoding: 'utf-8',
        timeout: 1500,
        stdio: ['ignore', 'pipe', 'ignore'],
      });
      return out.replace(/\r\n/g, ' ').replace(/\n/g, ' ').replace(/\r/g, ' ').trim();
    }
  } catch {
    return '';
  }
}

/**
 * Copy text to system clipboard (Windows PowerShell, macOS pbcopy, Linux xclip)
 */
export function setClipboardText(text: string): boolean {
  try {
    if (process.platform === 'win32') {
      const b64 = Buffer.from(text, 'utf16le').toString('base64');
      execSync(
        `powershell.exe -NoProfile -EncodedCommand ${Buffer.from(
          `$bytes = [Convert]::FromBase64String('${b64}'); $str = [System.Text.Encoding]::Unicode.GetString($bytes); Set-Clipboard -Value $str`,
          'utf16le'
        ).toString('base64')}`,
        {
          stdio: ['ignore', 'ignore', 'ignore'],
          timeout: 1500,
        }
      );
      return true;
    } else if (process.platform === 'darwin') {
      spawnSync('pbcopy', [], {
        input: text,
        encoding: 'utf-8',
        timeout: 1500,
      });
      return true;
    } else {
      spawnSync('xclip', ['-selection', 'clipboard'], {
        input: text,
        encoding: 'utf-8',
        timeout: 1500,
      });
      return true;
    }
  } catch {
    return false;
  }
}
