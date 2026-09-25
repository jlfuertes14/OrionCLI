import 'dotenv/config';
import { core } from './core.js';
import { formatSkillsPrompt, saveSynthesizedSkill, getSkill, loadAllSkills } from './skills.js';
import { lspFindDefinition, lspFindReferences } from './lsp.js';

export interface LlmMessage {
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  name?: string;
  tool_call_id?: string;
  tool_calls?: any[];
}

export interface StreamChatOptions {
  model: string;
  messages: LlmMessage[];
  mode?: 'build' | 'plan';
  signal?: AbortSignal;
  onChunk?: (chunk: string) => void;
  onToolCall?: (toolName: string, args: Record<string, any>) => void;
  onToolResult?: (toolName: string, result: string, args?: Record<string, any>) => void;
  maxRounds?: number;
  currentRound?: number;
}

interface ProviderConfig {
  baseUrl: string;
  apiKey: string;
  modelId: string;
}

function resolveProvider(modelSpec: string): ProviderConfig {
  const [providerPrefix, ...modelParts] = modelSpec.split(':');
  let rawModel = modelParts.join(':') || providerPrefix;
  const prov = providerPrefix.toLowerCase();

  switch (prov) {
    case 'mistral': {
      const apiKey = process.env.MISTRAL_API_KEY || process.env.MISTRAL_VIBE_API_KEY || '';
      return {
        baseUrl: 'https://api.mistral.ai/v1',
        apiKey,
        modelId: rawModel || 'mistral-small-latest',
      };
    }
    case 'openrouter': {
      const apiKey = process.env.OPENROUTER_API_KEY || '';
      return {
        baseUrl: 'https://openrouter.ai/api/v1',
        apiKey,
        modelId: rawModel,
      };
    }
    case 'openai': {
      const apiKey = process.env.OPENAI_API_KEY || '';
      return {
        baseUrl: 'https://api.openai.com/v1',
        apiKey,
        modelId: rawModel || 'gpt-4o',
      };
    }
    case 'groq': {
      const apiKey = process.env.GROQ_API_KEY || '';
      return {
        baseUrl: 'https://api.groq.com/openai/v1',
        apiKey,
        modelId: rawModel || 'llama-3.3-70b-versatile',
      };
    }
    case 'gemini': {
      const apiKey = process.env.GEMINI_API_KEY || '';
      return {
        baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
        apiKey,
        modelId: rawModel || 'gemini-2.5-flash',
      };
    }
    case 'ollama': {
      const host = process.env.OLLAMA_HOST || 'http://localhost:11434';
      return {
        baseUrl: `${host}/v1`,
        apiKey: 'ollama',
        modelId: rawModel || 'llama3.3',
      };
    }
    default: {
      if (prov === 'anthropic') {
        const apiKey = process.env.ANTHROPIC_API_KEY || '';
        return {
          baseUrl: 'https://api.anthropic.com/v1',
          apiKey,
          modelId: rawModel || 'claude-3-5-sonnet-latest',
        };
      }
      if (process.env.MISTRAL_API_KEY) {
        return {
          baseUrl: 'https://api.mistral.ai/v1',
          apiKey: process.env.MISTRAL_API_KEY,
          modelId: modelSpec,
        };
      }
      return {
        baseUrl: 'https://openrouter.ai/api/v1',
        apiKey: process.env.OPENROUTER_API_KEY || '',
        modelId: modelSpec,
      };
    }
  }
}

export const TOOLS_SCHEMA = [
  {
    type: 'function',
    function: {
      name: 'list_directory',
      description: 'List contents of a directory in the workspace',
      parameters: {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'Directory path (defaults to ".")' },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'read_file',
      description: 'Read the contents of a file',
      parameters: {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'Relative or absolute file path' },
        },
        required: ['path'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'write_file',
      description: 'Write or overwrite file contents in the local workspace. In BUILD mode, you have full autonomous authorization to create or modify code.',
      parameters: {
        type: 'object',
        properties: {
          path: { type: 'string', description: 'File path to create or overwrite' },
          content: { type: 'string', description: 'The complete new content of the file' },
        },
        required: ['path', 'content'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'run_command',
      description: 'Execute a shell command (PowerShell / sh) directly on the local machine with full developer permissions. Use to run tests (npm test, cargo test), build artifacts, install packages, check compilation, or inspect git.',
      parameters: {
        type: 'object',
        properties: {
          command: { type: 'string', description: 'The shell command to execute' },
        },
        required: ['command'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'git_status',
      description: 'Check git repository status',
      parameters: { type: 'object', properties: {} },
    },
  },
  {
    type: 'function',
    function: {
      name: 'git_diff',
      description: 'Check uncommitted git diffs',
      parameters: { type: 'object', properties: {} },
    },
  },
  {
    type: 'function',
    function: {
      name: 'grep_search',
      description: 'Search for text pattern across workspace files using ripgrep',
      parameters: {
        type: 'object',
        properties: {
          query: { type: 'string', description: 'Search term or regex' },
          path: { type: 'string', description: 'Search directory (defaults to ".")' },
        },
        required: ['query'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'lsp_definition',
      description: 'OpenCode LSP: Query local language server (rust-analyzer, tsserver, gopls, pyright) for compiler-accurate jump-to-definition of a symbol at file, line, and column.',
      parameters: {
        type: 'object',
        properties: {
          file: { type: 'string', description: 'Relative file path' },
          line: { type: 'integer', description: '1-indexed line number' },
          character: { type: 'integer', description: '1-indexed column number' },
        },
        required: ['file', 'line', 'character'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'lsp_references',
      description: 'OpenCode LSP: Query local language server for compiler-accurate workspace call-sites and references of a symbol at file, line, and column.',
      parameters: {
        type: 'object',
        properties: {
          file: { type: 'string', description: 'Relative file path' },
          line: { type: 'integer', description: '1-indexed line number' },
          character: { type: 'integer', description: '1-indexed column number' },
        },
        required: ['file', 'line', 'character'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'save_skill',
      description: 'Hermes Self-Synthesizing Skill: Persist a verified solution or pattern into .orion/skills/<name>.md so Orion remembers it in all future sessions.',
      parameters: {
        type: 'object',
        properties: {
          name: { type: 'string', description: 'Kebab-case skill name (e.g. napi-vitest-mocking)' },
          description: { type: 'string', description: 'One-sentence summary of when to apply this skill' },
          content: { type: 'string', description: 'Markdown body containing procedural steps, solution code, and traps to avoid' },
        },
        required: ['name', 'description', 'content'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'use_skill',
      description: "Load and activate specialized rules, principles, and guidelines from the user's .agents or .orion skills (e.g. 'emil-design-eng', 'ahmedragab-frontend-engineer', 'shadcn', 'vercel-design-guidelines', 'industrial-brutalist-ui', 'ui-ux-pro-max'). Call this tool whenever a task matches an available skill in the catalog to load its complete domain instructions into your context.",
      parameters: {
        type: 'object',
        properties: {
          skill_name: {
            type: 'string',
            description: "The exact name of the skill from the user's catalog (e.g. 'emil-design-eng', 'vercel-design-guidelines')",
          },
        },
        required: ['skill_name'],
      },
    },
  },
];

export async function streamChat(options: StreamChatOptions): Promise<string> {
  const {
    model,
    messages,
    mode = 'build',
    signal,
    onChunk,
    onToolCall,
    onToolResult,
    maxRounds = 25,
    currentRound = 0,
  } = options;

  if (currentRound >= maxRounds) {
    return 'Reached maximum autonomous agent loop limit (25 iterations). Halting to prevent infinite loops.';
  }

  const config = resolveProvider(model);
  if (!config.apiKey && !model.startsWith('ollama')) {
    const providerName = model.split(':')[0].toUpperCase();
    throw new Error(`Missing ${providerName}_API_KEY in .env file. Please check your credentials.`);
  }

  const skillsContext = formatSkillsPrompt();

  const basePrompt =
    mode === 'plan'
      ? `You are Orion in PLAN mode, an elite architectural analysis and planning agent.
Your primary role is to explore the codebase, analyze architecture, diagnose issues, and create comprehensive, step-by-step implementation plans.
Do NOT modify files directly or apply mutating code changes in PLAN mode.
Focus on research, architectural design, trade-offs, and clear actionable checklists.`
      : `You are Orion in BUILD mode, an autonomous high-performance agentic coding assistant built in Rust and TypeScript.
You are running directly in the user's terminal with full native execution permissions.
You can run shell commands, write files, inspect diffs, test compilation, and use compiler-accurate LSP.
Follow the ReAct agentic loop: Reason -> Act -> Observe -> Self-Correct -> Finish.
When tests fail or code errors occur, audit the output, locate mistakes, fix the code with write_file, and re-run tests until the goal is fully achieved.`;

  const systemPrompt = `${basePrompt}${skillsContext}`;

  const formattedMessages: any[] = [
    { role: 'system', content: systemPrompt },
    ...messages.map((m) => {
      const formatted: any = {
        role: m.role === 'tool' ? 'tool' : m.role,
        content: m.content,
      };
      if (m.tool_call_id) formatted.tool_call_id = m.tool_call_id;
      if (m.name) formatted.name = m.name;
      if (m.tool_calls) formatted.tool_calls = m.tool_calls;
      return formatted;
    }),
  ];

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${config.apiKey}`,
  };

  if (config.baseUrl.includes('openrouter')) {
    headers['HTTP-Referer'] = 'https://github.com/jlfuertes14/OrionCLI';
    headers['X-Title'] = 'Orion CLI';
  }

  const endpoint = `${config.baseUrl}/chat/completions`;

  const response = await fetch(endpoint, {
    method: 'POST',
    headers,
    signal,
    body: JSON.stringify({
      model: config.modelId,
      messages: formattedMessages,
      tools: TOOLS_SCHEMA,
      stream: true,
    }),
  });

  if (!response.ok) {
    const errorText = await response.text();
    throw new Error(`LLM API returned HTTP ${response.status}: ${errorText}`);
  }

  if (!response.body) {
    throw new Error('LLM API returned an empty response body.');
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let fullAssistantText = '';
  const toolCallsMap: Record<number, { id: string; name: string; arguments: string }> = {};

  let buffer = '';
  while (true) {
    if (signal?.aborted) {
      break;
    }
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() || '';

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || !trimmed.startsWith('data: ')) continue;
      const dataStr = trimmed.slice(6).trim();
      if (dataStr === '[DONE]') continue;

      try {
        const parsed = JSON.parse(dataStr);
        const choice = parsed.choices?.[0];
        if (!choice) continue;

        const contentDelta = choice.delta?.content;
        if (contentDelta) {
          fullAssistantText += contentDelta;
          if (onChunk) {
            onChunk(fullAssistantText);
          }
        }

        if (choice.delta?.tool_calls) {
          for (const tc of choice.delta.tool_calls) {
            const index = tc.index ?? 0;
            if (!toolCallsMap[index]) {
              toolCallsMap[index] = {
                id: tc.id || `call_${Date.now()}_${index}`,
                name: tc.function?.name || '',
                arguments: '',
              };
            }
            if (tc.function?.name) {
              toolCallsMap[index].name = tc.function.name;
            }
            if (tc.function?.arguments) {
              toolCallsMap[index].arguments += tc.function.arguments;
            }
          }
        }
      } catch {}
    }
  }

  const pendingCalls = Object.values(toolCallsMap);

  // If the model invoked tools, execute all of them and recursively loop!
  if (pendingCalls.length > 0) {
    const executedToolMessages: LlmMessage[] = [];

    for (const call of pendingCalls) {
      let parsedArgs: Record<string, any> = {};
      try {
        parsedArgs = JSON.parse(call.arguments || '{}');
      } catch {
        parsedArgs = {};
      }

      if (onToolCall) {
        onToolCall(call.name, parsedArgs);
      }

      let toolResult = '';

      // Plan mode safety check
      if (mode === 'plan' && (call.name === 'write_file' || (call.name === 'run_command' && isMutatingCommand(parsedArgs.command)))) {
        toolResult = `[PLAN MODE SAFEGUARD] Mutating action (${call.name}) is blocked in Plan mode. Tab to toggle into Build mode.`;
      } else {
        try {
          if (call.name === 'list_directory') {
            toolResult = await core.listDirectory(parsedArgs.path || '.');
          } else if (call.name === 'read_file') {
            toolResult = await core.readFile(parsedArgs.path);
          } else if (call.name === 'write_file') {
            toolResult = await core.writeFile(parsedArgs.path, parsedArgs.content);
          } else if (call.name === 'run_command') {
            toolResult = await core.runCommand(parsedArgs.command);
          } else if (call.name === 'git_status') {
            toolResult = await core.gitStatus();
          } else if (call.name === 'git_diff') {
            toolResult = await core.gitDiff();
          } else if (call.name === 'grep_search') {
            toolResult = await core.grepSearch(parsedArgs.query, parsedArgs.path || '.');
          } else if (call.name === 'lsp_definition') {
            toolResult = await lspFindDefinition(parsedArgs.file, parsedArgs.line, parsedArgs.character);
          } else if (call.name === 'lsp_references') {
            toolResult = await lspFindReferences(parsedArgs.file, parsedArgs.line, parsedArgs.character);
          } else if (call.name === 'save_skill') {
            const { slug, fullPath } = saveSynthesizedSkill(parsedArgs.name, parsedArgs.description, parsedArgs.content);
            toolResult = `Skill '${slug}' synthesized and persisted to ${fullPath}. Loaded automatically into all future sessions.`;
          } else if (call.name === 'use_skill') {
            const targetSkill = getSkill(parsedArgs.skill_name);
            if (targetSkill) {
              toolResult = `=== ACTIVE SKILL: ${targetSkill.name} (${targetSkill.category}) ===\nSource: ${targetSkill.sourcePath}\nDescription: ${targetSkill.description}\n\n${targetSkill.body}`;
            } else {
              const all = loadAllSkills();
              const suggestions = all.slice(0, 10).map((s) => s.name).join(', ');
              toolResult = `Skill '${parsedArgs.skill_name}' not found. Available skills include: ${suggestions}...`;
            }
          } else {
            toolResult = `Tool ${call.name} executed successfully.`;
          }
        } catch (err: any) {
          toolResult = `Error executing tool ${call.name}: ${err.message}`;
        }
      }

      if (onToolResult) {
        onToolResult(call.name, toolResult, parsedArgs);
      }

      executedToolMessages.push({
        role: 'tool',
        name: call.name,
        tool_call_id: call.id,
        content: toolResult,
      });
    }

    // Build next recursive turn
    const nextMessages: LlmMessage[] = [
      ...messages,
      {
        role: 'assistant',
        content: fullAssistantText,
        tool_calls: pendingCalls.map((c) => ({
          id: c.id,
          type: 'function',
          function: { name: c.name, arguments: c.arguments },
        })),
      },
      ...executedToolMessages,
    ];

    // Recursive agentic loop: observe outputs, recognize mistakes, self-correct!
    return streamChat({
      ...options,
      messages: nextMessages,
      currentRound: currentRound + 1,
    });
  }

  return fullAssistantText;
}

function isMutatingCommand(cmd?: string): boolean {
  if (!cmd) return false;
  const lower = cmd.toLowerCase();
  const mutators = ['rm ', 'del ', 'rmdir', 'git commit', 'git push', 'npm install', 'cargo add', 'npm run build'];
  return mutators.some((m) => lower.includes(m));
}

/**
 * Hermes Self-Synthesizing Skill Engine:
 * Analyze conversation history, distill key solution pattern and pitfalls, and save as .orion/skills/<slug>.md
 */
export async function synthesizeSkillFromSession(
  messages: LlmMessage[],
  model: string,
  topicHint?: string
): Promise<{ name: string; description: string; slug: string; fullPath: string }> {
  const config = resolveProvider(model);

  const prompt = `You are Orion's Self-Synthesizing Skill Engine (inspired by Hermes Agent).
Analyze the conversation below and extract the technical solution, debugging insights, and operational rules into a reusable skill.

Format your output STRICTLY as:
---
name: <short-kebab-slug>
description: <one-sentence description of when to apply this skill>
---
# <Title>

## Overview
<Brief context of the problem and trigger conditions>

## Verified Solution & Pattern
<Step-by-step instructions, exact code snippets, or commands>

## Gotchas & Mistakes to Avoid
<What went wrong initially, edge cases, and why the final fix works>

TOPIC HINT: ${topicHint || 'Extract the core engineering achievement and solution from this session'}`;

  const conversationTranscript = messages
    .filter((m) => m.role === 'user' || m.role === 'assistant')
    .map((m) => `${m.role.toUpperCase()}:\n${m.content}`)
    .join('\n\n');

  const response = await fetch(`${config.baseUrl}/chat/completions`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${config.apiKey}`,
    },
    body: JSON.stringify({
      model: config.modelId,
      messages: [
        { role: 'system', content: prompt },
        { role: 'user', content: `Here is the conversation transcript to synthesize:\n\n${conversationTranscript}` },
      ],
      stream: false,
    }),
  });

  if (!response.ok) {
    throw new Error(`LLM API returned ${response.status} during skill synthesis.`);
  }

  const json = await response.json();
  const text: string = json.choices?.[0]?.message?.content || '';

  const parts = text.split('---');
  let name = topicHint ? topicHint.toLowerCase().replace(/[^a-z0-9_-]/g, '-') : 'synthesized-task-solution';
  let description = 'Extracted problem-solving pattern from previous Orion session';
  let body = text;

  if (parts.length >= 3) {
    const frontmatter = parts[1];
    body = parts.slice(2).join('---').trim();
    for (const line of frontmatter.split('\n')) {
      const trimmed = line.trim();
      if (trimmed.startsWith('name:')) {
        name = trimmed.slice(5).trim().replace(/^["']|["']$/g, '');
      } else if (trimmed.startsWith('description:')) {
        description = trimmed.slice(12).trim().replace(/^["']|["']$/g, '');
      }
    }
  }

  const { slug, fullPath } = saveSynthesizedSkill(name, description, body);
  return { name, description, slug, fullPath };
}
