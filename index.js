import { createRequire } from 'node:module';
import { arch, platform } from 'node:process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);
const __dirname = dirname(fileURLToPath(import.meta.url));

function getBinaryName() {
  const p = platform; // 'win32' | 'linux' | 'darwin'
  const a = arch;     // 'x64' | 'arm64'

  if (p === 'win32' && a === 'x64')   return 'orion-core.win32-x64-msvc.node';
  if (p === 'linux' && a === 'x64')   return 'orion-core.linux-x64-gnu.node';
  if (p === 'linux' && a === 'arm64') return 'orion-core.linux-arm64-gnu.node';
  if (p === 'darwin' && a === 'x64')  return 'orion-core.darwin-x64.node';
  if (p === 'darwin' && a === 'arm64')return 'orion-core.darwin-arm64.node';

  throw new Error(`Unsupported platform: ${p} ${a}. OrionCLI supports win32-x64, linux-x64, linux-arm64, darwin-x64, darwin-arm64.`);
}

let nativeBinding = null;

try {
  const binaryName = getBinaryName();
  const binaryPath = join(__dirname, binaryName);
  nativeBinding = require(binaryPath);
} catch (e) {
  throw new Error(
    `Failed to load OrionCLI native core for ${platform}-${arch}.\n` +
    `Make sure you installed orion-cli from npm (which bundles the correct .node file).\n` +
    `Error: ${e.message}`
  );
}

export const {
  orionCoreVersion,
  napiExecuteTool,
  napiGetToolSchemas,
  napiReadFile,
  napiWriteFile,
  napiListDirectory,
  napiGitStatus,
  napiGitDiff,
  napiGrepSearch,
  napiRunCommand,
  napiComputeDiff,
  napiListSessions,
  napiDeleteSession,
} = nativeBinding;

export default nativeBinding;
