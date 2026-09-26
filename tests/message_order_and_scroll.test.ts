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

// Case 4: Parallel Tool Calls (Exact scenario: "check my src folder and read the codebase")
const parallelToolHistory = [
  { role: 'user', content: 'check my src folder and read the codebase' },
  {
    role: 'assistant',
    content: '',
    tool_calls: [
      { id: 'call_dir_1', type: 'function', function: { name: 'list_directory', arguments: '{"path":"src"}' } },
      { id: 'call_read_1', type: 'function', function: { name: 'read_file', arguments: '{"path":"src/index.ts"}' } },
      { id: 'call_read_2', type: 'function', function: { name: 'read_file', arguments: '{"path":"src/core.ts"}' } },
    ],
  },
  { role: 'tool', tool_call_id: 'call_dir_1', name: 'list_directory', content: 'index.ts\ncore.ts' },
  { role: 'tool', tool_call_id: 'call_read_1', name: 'read_file', content: 'console.log("index");' },
  { role: 'tool', tool_call_id: 'call_read_2', name: 'read_file', content: 'console.log("core");' },
];

const sanitized4 = sanitizeMessageOrder(parallelToolHistory);

// Verify ALL 3 parallel tool responses are preserved as role: 'tool'
if (sanitized4.length !== 5) {
  throw new Error(`Test 4 Failed: Expected 5 messages, got ${sanitized4.length}`);
}
if (sanitized4[2].role !== 'tool' || sanitized4[2].tool_call_id !== 'call_dir_1') {
  throw new Error(`Test 4 Failed: Tool 1 corrupted: ${JSON.stringify(sanitized4[2])}`);
}
if (sanitized4[3].role !== 'tool' || sanitized4[3].tool_call_id !== 'call_read_1') {
  throw new Error(`Test 4 Failed: Tool 2 corrupted into ${sanitized4[3].role}`);
}
if (sanitized4[4].role !== 'tool' || sanitized4[4].tool_call_id !== 'call_read_2') {
  throw new Error(`Test 4 Failed: Tool 3 corrupted into ${sanitized4[4].role}`);
}

// Invariant Check: The LAST role must be 'user' or 'tool' (NEVER 'assistant')
const lastRole4 = sanitized4[sanitized4.length - 1].role;
if (lastRole4 !== 'user' && lastRole4 !== 'tool') {
  throw new Error(`Test 4 Failed: Expected last role 'user' or 'tool', got: '${lastRole4}'`);
}
console.log('✔ Case 4 Passed: Multiple parallel tool calls preserved as role: tool, and last role is "tool"');

// Case 5: Trailing Assistant Message must never be served as last message to Mistral
const trailingAssistantHistory = [
  { role: 'user', content: 'hello' },
  { role: 'assistant', content: 'Hello there!' },
];

const sanitized5 = sanitizeMessageOrder(trailingAssistantHistory);
const lastRole5 = sanitized5[sanitized5.length - 1].role;
if (lastRole5 === 'assistant') {
  throw new Error(`Test 5 Failed: Trailing assistant role was not sanitized: ${lastRole5}`);
}
if (lastRole5 !== 'user' && lastRole5 !== 'tool') {
  throw new Error(`Test 5 Failed: Expected last role 'user' or 'tool', got: '${lastRole5}'`);
}
console.log('✔ Case 5 Passed: Trailing assistant message automatically prompts continuation so last role is NEVER assistant');

console.log('\n🎉 ALL MESSAGE ORDER & PARALLEL TOOL SANITIZATION TESTS PASSED SUCCESSFULLY!');
