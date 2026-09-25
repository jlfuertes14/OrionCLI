import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);

let nativeBinding = null;

try {
  nativeBinding = require('./orion-core.win32-x64-msvc.node');
} catch (e1) {
  try {
    nativeBinding = require('./orion-core.node');
  } catch (e2) {
    throw new Error(`Failed to load native binding: ${e1.message}`);
  }
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
