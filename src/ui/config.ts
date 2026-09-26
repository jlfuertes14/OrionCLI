import fs from 'fs';
import path from 'path';
import os from 'os';
import dotenv from 'dotenv';

export interface ProviderMeta {
  id: string;
  name: string;
  envVar: string;
  dashboardUrl: string;
  keyPlaceholder: string;
  keyPrefix?: string;
  description: string;
}

export const SUPPORTED_PROVIDERS: ProviderMeta[] = [
  {
    id: 'anthropic',
    name: 'Anthropic Claude',
    envVar: 'ANTHROPIC_API_KEY',
    dashboardUrl: 'https://console.anthropic.com/settings/keys',
    keyPlaceholder: 'sk-ant-api03-...',
    keyPrefix: 'sk-ant-',
    description: 'Claude 3.7 & 3.5 Sonnet / Haiku / Opus models',
  },
  {
    id: 'openai',
    name: 'OpenAI',
    envVar: 'OPENAI_API_KEY',
    dashboardUrl: 'https://platform.openai.com/api-keys',
    keyPlaceholder: 'sk-proj-...',
    keyPrefix: 'sk-',
    description: 'GPT-4o, GPT-4o-mini, o3-mini & o1 models',
  },
  {
    id: 'mistral',
    name: 'Mistral AI',
    envVar: 'MISTRAL_API_KEY',
    dashboardUrl: 'https://console.mistral.ai/api-keys',
    keyPlaceholder: 'paste Mistral API key...',
    description: 'Codestral & Mistral Medium 3.5 frontier models',
  },
  {
    id: 'gemini',
    name: 'Google Gemini',
    envVar: 'GEMINI_API_KEY',
    dashboardUrl: 'https://aistudio.google.com/app/apikey',
    keyPlaceholder: 'AIzaSy...',
    description: 'Gemini 2.5 Flash / Pro & 1.5 Pro multimodal models',
  },
  {
    id: 'groq',
    name: 'Groq (LPU)',
    envVar: 'GROQ_API_KEY',
    dashboardUrl: 'https://console.groq.com/keys',
    keyPlaceholder: 'gsk_...',
    description: 'Ultra-fast LPU inference (Llama 3.3 70B & DeepSeek R1)',
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    envVar: 'OPENROUTER_API_KEY',
    dashboardUrl: 'https://openrouter.ai/keys',
    keyPlaceholder: 'sk-or-v1-...',
    keyPrefix: 'sk-or-',
    description: 'Unified multi-provider router for all open & closed models',
  },
];

export function getGlobalOrionDir(): string {
  return path.join(os.homedir(), '.orion');
}

export function getGlobalEnvPath(): string {
  return path.join(getGlobalOrionDir(), '.env');
}

export function getGlobalConfigPath(): string {
  return path.join(getGlobalOrionDir(), 'config.json');
}

/**
 * Initializes configuration on startup:
 * 1. Loads workspace .env if present.
 * 2. Loads global ~/.orion/.env if present (without overwriting workspace variables).
 * 3. Loads global ~/.orion/config.json if present.
 */
export function initGlobalConfig(): void {
  // 1. Load workspace .env (existing process.cwd() .env)
  try {
    dotenv.config();
  } catch {}

  // 2. Load global ~/.orion/.env (fallback for keys across all folders)
  const globalEnv = getGlobalEnvPath();
  if (fs.existsSync(globalEnv)) {
    try {
      const parsed = dotenv.parse(fs.readFileSync(globalEnv, 'utf-8'));
      for (const [key, value] of Object.entries(parsed)) {
        if (!process.env[key] && value && !value.startsWith('your_') && !value.endsWith('_here')) {
          process.env[key] = value;
        }
      }
    } catch {}
  }

  // 3. Load global ~/.orion/config.json
  const globalConfig = getGlobalConfigPath();
  if (fs.existsSync(globalConfig)) {
    try {
      const content = fs.readFileSync(globalConfig, 'utf-8');
      const data = JSON.parse(content);
      if (data.providers && typeof data.providers === 'object') {
        for (const meta of SUPPORTED_PROVIDERS) {
          const providerData = data.providers[meta.id];
          const keyVal = providerData?.apiKey || providerData?.api_key;
          if (keyVal && !process.env[meta.envVar]) {
            process.env[meta.envVar] = keyVal;
          }
        }
      }
    } catch {}
  }
}

/**
 * Retrieves the API key for a given provider, checking:
 * 1. Current process.env (workspace .env or shell env)
 * 2. Global ~/.orion/.env
 * 3. Global ~/.orion/config.json
 */
export function getApiKeyForProvider(providerId: string): string | undefined {
  const meta = SUPPORTED_PROVIDERS.find((p) => p.id === providerId.toLowerCase());
  if (!meta) {
    if (providerId.toLowerCase() === 'ollama') return 'ollama';
    return undefined;
  }

  // Check process.env first
  const envVal = process.env[meta.envVar];
  if (envVal && !envVal.startsWith('your_') && !envVal.endsWith('_here') && envVal.trim().length > 0) {
    return envVal.trim();
  }

  // Fallback for Mistral alternate env var
  if (meta.id === 'mistral') {
    const vibeVal = process.env.MISTRAL_VIBE_API_KEY;
    if (vibeVal && !vibeVal.startsWith('your_') && !vibeVal.endsWith('_here') && vibeVal.trim().length > 0) {
      return vibeVal.trim();
    }
  }

  return undefined;
}

/**
 * Returns true if the provider or model has a valid, active API key.
 */
export function hasApiKeyForProvider(providerId: string): boolean {
  if (providerId.toLowerCase() === 'ollama') return true;
  const key = getApiKeyForProvider(providerId);
  return !!key && key.trim().length > 0;
}

/**
 * Returns true if the specified model string (e.g. 'anthropic:claude-3-5-sonnet')
 * has a configured API key.
 */
export function hasApiKeyForModel(modelString: string): boolean {
  const [providerRaw] = modelString.includes(':') ? modelString.split(':') : [modelString];
  const prov = (providerRaw || 'anthropic').toLowerCase();
  return hasApiKeyForProvider(prov);
}

/**
 * Returns the preferred default model based on which API keys are available on the system.
 */
export function detectDefaultModel(): string {
  if (process.env.ORION_PROVIDER && process.env.ORION_MODEL) {
    return `${process.env.ORION_PROVIDER}:${process.env.ORION_MODEL}`;
  }
  if (hasApiKeyForProvider('anthropic')) {
    return 'anthropic:claude-3-5-sonnet';
  }
  if (hasApiKeyForProvider('mistral')) {
    return 'mistral:mistral-medium-3.5';
  }
  if (hasApiKeyForProvider('openai')) {
    return 'openai:gpt-4o';
  }
  if (hasApiKeyForProvider('gemini')) {
    return 'gemini:gemini-2.5-flash';
  }
  if (hasApiKeyForProvider('groq')) {
    return 'groq:llama-3.3-70b-versatile';
  }
  if (hasApiKeyForProvider('openrouter')) {
    return 'openrouter:anthropic/claude-3.5-sonnet';
  }
  // Default to Anthropic if no keys set yet
  return 'anthropic:claude-3-5-sonnet';
}

/**
 * Persists an API key globally to ~/.orion/.env and ~/.orion/config.json,
 * and sets it into the current process.env.
 */
export function saveApiKey(providerId: string, apiKey: string): { success: boolean; path: string } {
  const cleanKey = apiKey.trim();
  const meta = SUPPORTED_PROVIDERS.find((p) => p.id === providerId.toLowerCase());
  if (!meta) {
    throw new Error(`Unsupported provider '${providerId}'`);
  }

  // 1. Set into current process.env immediately
  process.env[meta.envVar] = cleanKey;

  // 2. Ensure ~/.orion directory exists
  const orionDir = getGlobalOrionDir();
  if (!fs.existsSync(orionDir)) {
    fs.mkdirSync(orionDir, { recursive: true });
  }

  // 3. Write or update ~/.orion/.env
  const envPath = getGlobalEnvPath();
  let envLines: string[] = [];
  if (fs.existsSync(envPath)) {
    try {
      envLines = fs.readFileSync(envPath, 'utf-8').split('\n');
    } catch {
      envLines = [];
    }
  }

  let found = false;
  const newLines = envLines.map((line) => {
    const trimmed = line.trim();
    if (trimmed.startsWith(`${meta.envVar}=`)) {
      found = true;
      return `${meta.envVar}=${cleanKey}`;
    }
    return line;
  });

  if (!found) {
    newLines.push(`${meta.envVar}=${cleanKey}`);
  }

  fs.writeFileSync(envPath, newLines.join('\n').trim() + '\n', 'utf-8');

  // 4. Update ~/.orion/config.json
  const configPath = getGlobalConfigPath();
  let configData: any = { providers: {} };
  if (fs.existsSync(configPath)) {
    try {
      configData = JSON.parse(fs.readFileSync(configPath, 'utf-8'));
    } catch {
      configData = { providers: {} };
    }
  }
  if (!configData.providers) configData.providers = {};
  configData.providers[meta.id] = {
    apiKey: cleanKey,
    updatedAt: new Date().toISOString(),
  };
  fs.writeFileSync(configPath, JSON.stringify(configData, null, 2), 'utf-8');

  return { success: true, path: envPath };
}

/**
 * Masks an API key for safe display (e.g. "sk-ant-...4a9f")
 */
export function maskApiKey(key?: string): string {
  if (!key || key.trim().length === 0) return 'Not set';
  const trimmed = key.trim();
  if (trimmed.length <= 8) return '••••••••';
  const prefix = trimmed.slice(0, Math.min(7, Math.floor(trimmed.length / 3)));
  const suffix = trimmed.slice(-4);
  return `${prefix}••••${suffix}`;
}
