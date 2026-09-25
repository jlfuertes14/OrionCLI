import fs from 'fs';
import path from 'path';
import { loadAllSkills, formatSkillsPrompt, saveSynthesizedSkill, getSkill } from '../src/ui/skills.js';
import { TOOLS_SCHEMA } from '../src/ui/llm.js';

console.log('🧪 Testing Hermes Self-Synthesizing Skills & Autonomous Tools...');

// 1. Verify Autonomous Tool Schemas
const toolNames = TOOLS_SCHEMA.map((t) => t.function.name);
console.log('Registered LLM tools:', toolNames);

if (!toolNames.includes('run_command')) {
  throw new Error('run_command tool is missing from TOOLS_SCHEMA');
}
if (!toolNames.includes('write_file')) {
  throw new Error('write_file tool is missing from TOOLS_SCHEMA');
}
if (!toolNames.includes('lsp_definition')) {
  throw new Error('lsp_definition tool is missing from TOOLS_SCHEMA');
}
if (!toolNames.includes('save_skill')) {
  throw new Error('save_skill tool is missing from TOOLS_SCHEMA');
}
console.log('✔ All autonomous tools (run_command, write_file, lsp_definition, save_skill) registered!');

// 2. Test Skill Persistence
const testDir = path.join(process.cwd(), '.orion', 'skills');
const testSkillName = 'test-skill-harness';
const { slug, fullPath } = saveSynthesizedSkill(
  testSkillName,
  'Test skill for self-synthesized agent harness',
  '## Instructions\nAlways verify NAPI bindings before launching the CLI.'
);

if (!fs.existsSync(fullPath)) {
  throw new Error(`Failed to create skill at ${fullPath}`);
}
console.log(`✔ Skill successfully created at ${fullPath}`);

// 3. Test Skill Discovery & Loading
const skills = loadAllSkills();
const found = skills.find((s) => s.name === slug);
if (!found) {
  throw new Error(`loadAllSkills did not find newly synthesized skill: ${slug}`);
}
console.log(`✔ Total skills discovered: ${skills.length}`);
console.log(`✔ User .agents skills count: ${skills.filter((s) => s.category === '.agents').length}`);
console.log(`✔ Discovered synthesized skill: ${found.name} (${found.category})`);

// 4. Test System Prompt Injection
const promptSection = formatSkillsPrompt();
if (!promptSection.includes(slug)) {
  throw new Error('Skills prompt section did not include synthesized skill');
}
console.log('✔ formatSkillsPrompt correctly injected the synthesized skill into the prompt!');

// 5. Test User .agents Skill Retrieval
const emilSkill = getSkill('emil-design-eng');
if (!emilSkill) {
  throw new Error('Could not find emil-design-eng from ~/.agents/skills');
}
console.log(`✔ Successfully retrieved user .agents skill: ${emilSkill.name} (${emilSkill.body.length} chars)`);

// Clean up test skill file
try {
  fs.unlinkSync(fullPath);
} catch {}

console.log('\n🎉 ALL HERMES SKILLS & AUTONOMOUS AGENT TESTS PASSED!');
