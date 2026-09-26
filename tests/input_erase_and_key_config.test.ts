import fs from 'fs';
import path from 'path';
import os from 'os';
import {
  SUPPORTED_PROVIDERS,
  getApiKeyForProvider,
  hasApiKeyForProvider,
  hasApiKeyForModel,
  saveApiKey,
  maskApiKey,
  detectDefaultModel,
  getGlobalEnvPath,
  getGlobalConfigPath,
} from '../src/ui/config.js';
import {
  isTerminalBackspace,
  isTerminalForwardDelete,
  stripMouseSequences,
  isMouseSequence,
  setLastRawInput,
} from '../src/ui/stdinTracker.js';

console.log('🧪 Running Orion CLI Input Erase & API Key Configuration Tests...\n');

// ==========================================
// TEST SUITE 1: Stdin Tracker & Erase Logic
// ==========================================
console.log('--- Test Suite 1: Erase and Backspace Key Detection ---');

// 1. Standard terminal Backspace (ASCII 127 \x7f)
setLastRawInput('\x7f');
const isBs1 = isTerminalBackspace({ delete: true, backspace: false });
if (!isBs1) {
  throw new Error('FAILED: ASCII 127 with key.delete must be recognized as Backspace');
}
console.log('✔ ASCII 127 (\\x7f) correctly parsed as Backspace even when ink sets key.delete=true');

// 2. Windows cmd.exe Backspace (ASCII 8 \x08 or \b)
setLastRawInput('\x08');
const isBs2 = isTerminalBackspace({ backspace: true, delete: false });
if (!isBs2) {
  throw new Error('FAILED: ASCII 8 with key.backspace must be recognized as Backspace');
}
console.log('✔ ASCII 8 (\\x08) correctly parsed as Backspace');

// 3. Ctrl+H in terminal
setLastRawInput('\x08');
const isBs3 = isTerminalBackspace({ ctrl: true }, 'h');
if (!isBs3) {
  throw new Error('FAILED: Ctrl+H must be recognized as Backspace');
}
console.log('✔ Ctrl+H correctly parsed as Backspace');

// 4. Physical Forward Delete Key (\x1b[3~)
setLastRawInput('\x1b[3~');
const isFwdDel = isTerminalForwardDelete({ delete: true });
const isNotBs = isTerminalBackspace({ delete: true });
if (!isFwdDel) {
  throw new Error('FAILED: \\x1b[3~ must be recognized as Forward Delete');
}
if (isNotBs) {
  throw new Error('FAILED: \\x1b[3~ must NOT be recognized as Backspace');
}
console.log('✔ Forward Delete (\\x1b[3~) correctly differentiated from Backspace');

// 5. Mouse tracking / click escape sequences (e.g. [<0;53;22m)
const mouseSample = '[<0;53;22m';
if (!isMouseSequence(mouseSample)) {
  throw new Error(`FAILED: isMouseSequence failed on '${mouseSample}'`);
}
const stripped = stripMouseSequences(mouseSample);
if (stripped !== '') {
  throw new Error(`FAILED: stripMouseSequences failed, expected '', got '${stripped}'`);
}
console.log('✔ Mouse click/scroll sequences ([<0;53;22m) correctly identified and completely stripped');

// ==========================================
// TEST SUITE 2: Text Editing & Erase Operations
// ==========================================
console.log('\n--- Test Suite 2: Text Editing & Erase Operations ---');

class MockPromptInput {
  value = '';
  cursorOffset = 0;
  isAllSelected = false;

  type(str: string) {
    if (this.isAllSelected) {
      this.value = str;
      this.cursorOffset = str.length;
      this.isAllSelected = false;
    } else {
      this.value =
        this.value.slice(0, this.cursorOffset) +
        str +
        this.value.slice(this.cursorOffset);
      this.cursorOffset += str.length;
    }
  }

  selectAll() {
    this.isAllSelected = true;
  }

  pressBackspace(key: { delete?: boolean; backspace?: boolean; ctrl?: boolean } = { delete: true }, input?: string) {
    if (isTerminalBackspace(key, input)) {
      if (this.isAllSelected) {
        this.value = '';
        this.cursorOffset = 0;
        this.isAllSelected = false;
        return;
      }
      if (this.cursorOffset > 0) {
        this.value =
          this.value.slice(0, this.cursorOffset - 1) +
          this.value.slice(this.cursorOffset);
        this.cursorOffset--;
      }
    }
  }

  pressForwardDelete(key: { delete?: boolean } = { delete: true }) {
    if (isTerminalForwardDelete(key)) {
      if (this.isAllSelected) {
        this.value = '';
        this.cursorOffset = 0;
        this.isAllSelected = false;
        return;
      }
      if (this.cursorOffset < this.value.length) {
        this.value =
          this.value.slice(0, this.cursorOffset) +
          this.value.slice(this.cursorOffset + 1);
      }
    }
  }

  pressCtrlU() {
    if (this.isAllSelected) {
      this.value = '';
      this.cursorOffset = 0;
      this.isAllSelected = false;
    } else {
      this.value = this.value.slice(this.cursorOffset);
      this.cursorOffset = 0;
    }
  }

  pressCtrlW() {
    if (this.isAllSelected) {
      this.value = '';
      this.cursorOffset = 0;
      this.isAllSelected = false;
      return;
    }
    if (this.cursorOffset > 0) {
      const before = this.value.slice(0, this.cursorOffset);
      const after = this.value.slice(this.cursorOffset);
      const trimmed = before.replace(/\s+$/, '');
      const lastSpace = trimmed.lastIndexOf(' ');
      const newBefore = lastSpace >= 0 ? trimmed.slice(0, lastSpace + 1) : '';
      this.value = newBefore + after;
      this.cursorOffset = newBefore.length;
    }
  }
}

// 1. Erase at end of input
const input = new MockPromptInput();
input.type('Write unit tests for Orion');
if (input.value !== 'Write unit tests for Orion' || input.cursorOffset !== 26) {
  throw new Error(`Unexpected typed value: ${input.value}`);
}

setLastRawInput('\x7f'); // Backspace
input.pressBackspace();
if (input.value !== 'Write unit tests for Orio' || input.cursorOffset !== 25) {
  throw new Error(`Backspace failed to erase last character: ${input.value}`);
}
input.pressBackspace();
input.pressBackspace();
if (input.value !== 'Write unit tests for Or' || input.cursorOffset !== 23) {
  throw new Error(`Multiple backspaces failed: ${input.value}`);
}
console.log('✔ Backspace at end of line successfully erases characters');

// 2. Erase in middle of input
input.cursorOffset = 5; // right after "Write"
input.pressBackspace(); // deletes 'e'
if (input.value !== 'Writ unit tests for Or' || input.cursorOffset !== 4) {
  throw new Error(`Backspace in middle failed: ${input.value}, offset: ${input.cursorOffset}`);
}
console.log('✔ Backspace in middle of line successfully erases preceding character');

// 3. Forward Delete
setLastRawInput('\x1b[3~');
input.cursorOffset = 4; // on space after "Writ"
input.pressForwardDelete(); // deletes space
if (input.value !== 'Writunit tests for Or') {
  throw new Error(`Forward delete failed: ${input.value}`);
}
console.log('✔ Forward delete successfully erases character after cursor');

// 4. Ctrl+W (Erase word)
input.value = 'npm run build:ui --watch';
input.cursorOffset = input.value.length;
input.pressCtrlW();
if (input.value !== 'npm run build:ui ') {
  throw new Error(`Ctrl+W failed: '${input.value}'`);
}
console.log('✔ Ctrl+W successfully erases previous word');

// 5. Ctrl+U (Clear line before cursor)
input.value = 'Fix bug in PromptInput.tsx';
input.cursorOffset = input.value.length;
input.pressCtrlU();
if (input.value !== '') {
  throw new Error(`Ctrl+U failed: '${input.value}'`);
}
console.log('✔ Ctrl+U successfully erases whole prompt');

// 6. Select All + Backspace
input.value = 'Some typed prompt';
input.selectAll();
setLastRawInput('\x7f');
input.pressBackspace();
if (input.value !== '' || input.cursorOffset !== 0) {
  throw new Error(`Select All + Backspace failed: '${input.value}'`);
}
console.log('✔ Ctrl+A + Backspace successfully clears all selected text');


// ==========================================
// TEST SUITE 3: Global API Key Management
// ==========================================
console.log('\n--- Test Suite 3: Global API Key Management ---');

// 1. Masking
const maskedAnt = maskApiKey('sk-ant-api03-abcdef1234567890');
if (!maskedAnt.startsWith('sk-ant-') || !maskedAnt.endsWith('7890') || !maskedAnt.includes('••••')) {
  throw new Error(`maskApiKey produced invalid result: ${maskedAnt}`);
}
console.log(`✔ maskApiKey correctly masks keys: ${maskedAnt}`);

// 2. Ollama requires no key
if (!hasApiKeyForModel('ollama:llama3.3')) {
  throw new Error('Ollama models should always report having an available key');
}
console.log('✔ Ollama models correctly report key availability without remote API key');

// 3. Saving & reading API key
const testKey = 'sk-ant-api03-test-token-valid-abc123xyz';
const saveRes = saveApiKey('anthropic', testKey);
if (!saveRes.success || !fs.existsSync(saveRes.path)) {
  throw new Error(`saveApiKey failed to write to path: ${saveRes.path}`);
}

const retrieved = getApiKeyForProvider('anthropic');
if (retrieved !== testKey) {
  throw new Error(`Expected retrieved key '${testKey}', got '${retrieved}'`);
}
console.log('✔ saveApiKey successfully persisted to ~/.orion/.env and loaded into memory');

const hasAnt = hasApiKeyForModel('anthropic:claude-3-5-sonnet');
if (!hasAnt) {
  throw new Error('hasApiKeyForModel failed to detect saved anthropic key');
}
console.log('✔ hasApiKeyForModel correctly detects saved key for anthropic models');

// 4. Default Model Detection
const defModel = detectDefaultModel();
if (!defModel.includes('anthropic')) {
  throw new Error(`Expected default model to choose anthropic, got '${defModel}'`);
}
console.log(`✔ detectDefaultModel smartly selected: ${defModel}`);

console.log('\n🎉 ALL INPUT ERASE & KEY CONFIGURATION TESTS PASSED SUCCESSFULLY!\n');
