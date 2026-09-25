import { core } from '../src/ui/core.js';

async function runSmokeTests() {
  console.log('🧪 Starting Orion NAPI + TypeScript smoke tests...\n');

  // 1. Version test
  const version = core.version();
  console.log(`✔ Native core version: ${version}`);
  if (!version) throw new Error('Version check failed');

  // 2. List directory test
  const dirOutput = await core.listDirectory('.');
  console.log(`✔ Native list_directory succeeded (${dirOutput.length} chars)`);
  if (!dirOutput.includes('Cargo.toml')) throw new Error('Directory listing missing Cargo.toml');

  // 3. Git status test
  const gitStatus = await core.gitStatus();
  console.log(`✔ Native git_status succeeded`);
  if (!gitStatus.includes('branch')) throw new Error('Git status output unexpected');

  // 4. Compute diff test
  const oldText = 'line1\nold_line\nline3\n';
  const newText = 'line1\nnew_line\nline3\n';
  const diff = core.computeDiff(oldText, newText);
  console.log(`✔ Native compute_diff produced ${diff.length} diff chunks`);
  const hasInsert = diff.some((d) => d.tag === 'insert');
  const hasDelete = diff.some((d) => d.tag === 'delete');
  if (!hasInsert || !hasDelete) throw new Error('Diff calculation missing changes');

  // 5. Tool schemas test
  const schemas = core.getToolSchemas();
  console.log(`✔ Native getToolSchemas returned ${schemas.length} registered tools`);

  console.log('\n🎉 ALL NAPI-RS INTEGRATION TESTS PASSED SUCCESSFULLY!');
}

runSmokeTests().catch((err) => {
  console.error('❌ Test failed:', err);
  process.exit(1);
});
