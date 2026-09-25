import chalk from 'chalk';

// High-end OpenCode 1:1 Palette (Minimalist Obsidian & Pitch Black)
export const theme = {
  // Pitch black & Obsidian surfaces
  bg: '#000000',
  cardBg: '#18181B',
  surface: '#121212',

  // Monochromatic text & grays
  white: chalk.hex('#FFFFFF'),
  primary: chalk.hex('#F4F4F5'),
  secondary: chalk.hex('#D4D4D8'),
  muted: chalk.hex('#71717A'),
  subtle: chalk.hex('#52525B'),
  border: '#27272A',
  borderHighlight: '#3F3F46',

  // OpenCode Signature Accents
  opencodeBlue: '#3B82F6', // Blue vertical accent line, "Build", and "▣"
  opencodeAmber: '#F59E0B', // Amber "● Tip" dot and bullet category labels
  opencodeCyan: '#38BDF8',
  opencodeGreen: '#10B981',
  opencodeRed: '#EF4444',

  // Chalk instances for terminal output
  blue: chalk.hex('#3B82F6'),
  amber: chalk.hex('#F59E0B'),
  green: chalk.hex('#10B981'),
  red: chalk.hex('#EF4444'),

  symbols: {
    arrow: '❯',
    square: '▣',
    dot: '·',
    bullet: '•',
    bar: '│',
    caret: '█',
  },
};

// Formats provider and model name cleanly for OpenCode style: "Mistral Medium 3.5 Mistral"
export function formatModelInfo(modelString: string): { displayName: string; provider: string } {
  const [providerRaw, modelRaw] = modelString.includes(':')
    ? modelString.split(':')
    : ['', modelString];

  const providerMap: Record<string, string> = {
    mistral: 'Mistral',
    anthropic: 'Anthropic',
    openai: 'OpenAI',
    google: 'Google',
    groq: 'Groq',
    openrouter: 'OpenRouter',
    ollama: 'Ollama',
  };

  const modelMap: Record<string, string> = {
    'mistral-medium-3.5': 'Mistral Medium 3.5',
    'mistral-medium-3-5': 'Mistral Medium 3.5',
    'mistral-medium-latest': 'Mistral Medium',
    'codestral-latest': 'Codestral Latest',
    'mistral-large-latest': 'Mistral Large',
    'mistral-small-latest': 'Mistral Small',
    'pixtral-large-latest': 'Pixtral Large',
    'ministral-8b-latest': 'Ministral 8B',
    'claude-3-5-sonnet': 'Claude 3.5 Sonnet',
    'claude-3-5-haiku': 'Claude 3.5 Haiku',
    'gpt-4o': 'GPT-4o',
    'gpt-4o-mini': 'GPT-4o Mini',
    'gemini-1.5-pro': 'Gemini 1.5 Pro',
    'gemini-1.5-flash': 'Gemini 1.5 Flash',
    'deepseek-r1': 'DeepSeek R1',
  };

  const provider =
    providerMap[(providerRaw || '').toLowerCase()] ||
    (providerRaw ? providerRaw.charAt(0).toUpperCase() + providerRaw.slice(1) : 'Orion');

  const displayName = modelMap[modelRaw || ''] || modelRaw || modelString;

  return { displayName, provider };
}

// Formats directory path with ~ for user home and :branch suffix (e.g. ~\Desktop\Projects\orion-cli:main)
export function formatCwdWithBranch(cwd: string, gitBranch?: string): string {
  const home = process.env.USERPROFILE || process.env.HOME || '';
  let formatted = cwd;
  if (home && cwd.startsWith(home)) {
    formatted = '~' + cwd.slice(home.length);
  }
  return gitBranch ? `${formatted}:${gitBranch}` : formatted;
}

