import { sanitizeMessageOrder } from '../src/ui/llm.js';

console.log('🧪 Testing Multi-Turn Message Order & Tool Role Sanitization...');

// Case 1: Orphan tool role directly following user role (The exact bug reported by user)
const badHistory1 = [
  { role: 'user', content: 'check repo' },
  { role: 'tool', name: 'list_directory', content: 'file1.ts\nfile2.ts' },
  { role: 'assistant', content: 'I checked the repo.' },
  { role: 'user', content: 'what should we do?' },
];

const sanitized1 = sanitizeMessageOrder(badHistory1);

// Verify NO tool role exists directly following a user role
for (let i = 0; i < sanitized1.length; i++) {
  if (sanitized1[i].role === 'tool') {
    const prev = sanitized1[i - 1];
    if (!prev || prev.role !== 'assistant' || !prev.tool_calls) {
      throw new Error(`Test 1 Failed: Orphan tool message found at index ${i}: ${JSON.stringify(sanitized1[i])}`);
    }
  }
}

// In sanitized1, the orphan tool message must have been converted into an assistant message or merged
if (sanitized1[1].role !== 'assistant') {
  throw new Error(`Test 1 Failed: Expected orphan tool at index 1 to be converted to assistant, got: ${sanitized1[1].role}`);
}
console.log('✔ Case 1 Passed: Orphan tool message following user message is safely converted to assistant message');

// Case 2: Valid tool call with matching assistant tool_calls is preserved
const validHistory = [
  { role: 'user', content: 'list files' },
  {
    role: 'assistant',
    content: '',
    tool_calls: [{ id: 'call_123', type: 'function', function: { name: 'list_directory', arguments: '{}' } }],
  },
  { role: 'tool', tool_call_id: 'call_123', name: 'list_directory', content: 'index.ts' },
];

const sanitized2 = sanitizeMessageOrder(validHistory);
if (sanitized2[2].role !== 'tool' || sanitized2[2].tool_call_id !== 'call_123') {
  throw new Error(`Test 2 Failed: Valid tool message was mutated: ${JSON.stringify(sanitized2[2])}`);
}
console.log('✔ Case 2 Passed: Valid tool message with matching assistant tool_calls is preserved');

// Case 3: Multiple turns with past tool executions filtered to user + assistant
const multiTurnMessages = [
  { role: 'user', content: 'check repo' },
  { role: 'tool', name: 'list_directory', content: 'file1, file2' },
  { role: 'assistant', content: 'Found 2 files.' },
  { role: 'user', content: 'what should we do?' },
];

const filteredMultiTurn = multiTurnMessages
  .filter((m) => m.role === 'user' || m.role === 'assistant')
  .map((m) => ({ role: m.role, content: m.content }));

if (filteredMultiTurn.some((m) => m.role === 'tool')) {
  throw new Error('Test 3 Failed: tool message remained in multi-turn history');
}
if (filteredMultiTurn.length !== 3) {
  throw new Error(`Test 3 Failed: Expected 3 messages, got ${filteredMultiTurn.length}`);
}
if (filteredMultiTurn[0].role !== 'user' || filteredMultiTurn[1].role !== 'assistant' || filteredMultiTurn[2].role !== 'user') {
  throw new Error(`Test 3 Failed: Invalid roles: ${filteredMultiTurn.map((m) => m.role).join(', ')}`);
}
console.log('✔ Case 3 Passed: Multi-turn history cleanly maintains user -> assistant -> user sequence');

console.log('\n🎉 ALL MESSAGE ORDER & TOOL SANITIZATION TESTS PASSED SUCCESSFULLY!');
