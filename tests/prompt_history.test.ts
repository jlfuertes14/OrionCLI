console.log('🧪 Testing Prompt History Tracking (Up/Down Navigation)...');

// Simulate the PromptInput history tracking logic
class PromptHistoryNavigator {
  history: string[] = [];
  historyIndex: number = -1;
  draftValue: string = '';
  value: string = '';

  constructor(initialHistory?: string[]) {
    if (initialHistory) {
      this.history = [...initialHistory];
    }
  }

  submit(val: string) {
    const trimmed = val.trim();
    if (!trimmed) return;
    if (this.history.length === 0 || this.history[this.history.length - 1] !== trimmed) {
      this.history.push(trimmed);
    }
    this.historyIndex = -1;
    this.draftValue = '';
    this.value = '';
  }

  pressUp() {
    if (this.history.length === 0) return;
    if (this.historyIndex === -1) {
      this.draftValue = this.value;
      this.historyIndex = this.history.length - 1;
      this.value = this.history[this.historyIndex];
    } else if (this.historyIndex > 0) {
      this.historyIndex--;
      this.value = this.history[this.historyIndex];
    }
  }

  pressDown() {
    if (this.historyIndex !== -1) {
      if (this.historyIndex < this.history.length - 1) {
        this.historyIndex++;
        this.value = this.history[this.historyIndex];
      } else {
        this.historyIndex = -1;
        this.value = this.draftValue;
      }
    }
  }
}

const nav = new PromptHistoryNavigator();
nav.submit('first prompt: what is this repo?');
nav.submit('second prompt: run build test');
nav.submit('third prompt: review changes');

if (nav.history.length !== 3) {
  throw new Error(`Expected 3 items in history, got ${nav.history.length}`);
}

// User enters draft text then presses Up
nav.value = 'partially typed question';
nav.pressUp();
if (nav.value !== 'third prompt: review changes') {
  throw new Error(`Expected 'third prompt: review changes', got '${nav.value}'`);
}
console.log('✔ Press Up (1st time): recalled most recent prompt');

nav.pressUp();
if (nav.value !== 'second prompt: run build test') {
  throw new Error(`Expected 'second prompt: run build test', got '${nav.value}'`);
}
console.log('✔ Press Up (2nd time): stepped back to 2nd prompt');

nav.pressUp();
if (nav.value !== 'first prompt: what is this repo?') {
  throw new Error(`Expected 'first prompt: what is this repo?', got '${nav.value}'`);
}
console.log('✔ Press Up (3rd time): stepped back to oldest prompt');

// Press Up when already at oldest -> stays at oldest
nav.pressUp();
if (nav.value !== 'first prompt: what is this repo?') {
  throw new Error(`Expected to stay at oldest prompt, got '${nav.value}'`);
}
console.log('✔ Press Up at top boundary: remained clamped at oldest');

// Press Down to move forward
nav.pressDown();
if (nav.value !== 'second prompt: run build test') {
  throw new Error(`Expected 'second prompt: run build test', got '${nav.value}'`);
}
console.log('✔ Press Down: moved forward to 2nd prompt');

nav.pressDown();
if (nav.value !== 'third prompt: review changes') {
  throw new Error(`Expected 'third prompt: review changes', got '${nav.value}'`);
}
console.log('✔ Press Down: moved forward to 3rd prompt');

// Press Down to return to draft
nav.pressDown();
if (nav.value !== 'partially typed question') {
  throw new Error(`Expected draft restored, got '${nav.value}'`);
}
console.log('✔ Press Down at end: restored user draft text cleanly');

console.log('\n🎉 ALL PROMPT HISTORY TESTS PASSED SUCCESSFULLY!');
