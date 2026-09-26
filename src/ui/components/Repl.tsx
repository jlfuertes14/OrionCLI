import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import { Header } from './Header.js';
import { PromptInput } from './PromptInput.js';
import { SlashCommand, CommandPicker, COMMANDS } from './CommandPicker.js';
import { ModelSelector } from './ModelSelector.js';
import { SessionSelector } from './SessionSelector.js';
import { SkillSelector } from './SkillSelector.js';
import { ToolApproval } from './ToolApproval.js';
import { MessageList, ChatMessage, PendingToolCall } from './MessageList.js';
import { StatusBadge, AgentStatus } from './StatusBadge.js';
import { OpenCodeLoader } from './OpenCodeLoader.js';
import { KeyInputModal } from './KeyInputModal.js';
import { core, DiffLine } from '../core.js';
import { streamChat, synthesizeSkillFromSession, LlmMessage } from '../llm.js';
import { getSkill, loadAllSkills, LoadedSkill } from '../skills.js';
import { formatCwdWithBranch } from '../theme.js';
import { hasApiKeyForModel } from '../config.js';

interface ReplProps {
  initialPrompt?: string;
  initialModel?: string;
}

export const Repl: React.FC<ReplProps> = ({
  initialPrompt,
  initialModel = 'mistral:mistral-medium-3.5',
}) => {
  const [model, setModel] = useState(initialModel);
  const [mode, setMode] = useState<'build' | 'plan'>('build');
  const [cwd, setCwd] = useState(process.cwd());
  const [gitBranch, setGitBranch] = useState<string | undefined>();
  const [status, setStatus] = useState<AgentStatus>('idle');
  const [currentTool, setCurrentTool] = useState<string | undefined>();
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [streamingContent, setStreamingContent] = useState<string | undefined>();
  const [pendingToolCall, setPendingToolCall] = useState<PendingToolCall | null>(null);
  const abortControllerRef = React.useRef<AbortController | null>(null);
  const [activeSkills, setActiveSkills] = useState<LoadedSkill[]>([]);
  const [activeSelector, setActiveSelector] = useState<'none' | 'model' | 'session' | 'command' | 'skill' | 'key'>('none');
  const [keyTargetProvider, setKeyTargetProvider] = useState<string>('anthropic');
  const [pendingPromptAfterKey, setPendingPromptAfterKey] = useState<string | null>(null);
  const [keyModalNotice, setKeyModalNotice] = useState<string | undefined>();
  const [pendingApproval, setPendingApproval] = useState<{
    toolName: string;
    args: Record<string, any>;
    diffLines?: DiffLine[];
    resolve: (approved: boolean) => void;
  } | null>(null);

  const [dimensions, setDimensions] = useState({
    rows: process.stdout.rows || 30,
    cols: process.stdout.columns || 100,
  });
  const [scrollOffset, setScrollOffset] = useState(0);

  const hasActiveSessionRef = React.useRef(messages.length > 0);
  useEffect(() => {
    hasActiveSessionRef.current = messages.length > 0;
  }, [messages.length]);

  const activeSelectorRef = React.useRef(activeSelector);
  useEffect(() => {
    activeSelectorRef.current = activeSelector;
  }, [activeSelector]);

  const handleScrollUp = (lines = 3) => {
    setScrollOffset((prev) => prev + lines);
  };

  const handleScrollDown = (lines = 3) => {
    setScrollOffset((prev) => Math.max(0, prev - lines));
  };

  // Enable terminal SGR mouse reporting mode for mouse wheel scrolling
  useEffect(() => {
    if (process.stdout.isTTY) {
      process.stdout.write('\x1b[?1000h\x1b[?1006h');
    }

    const sgrRegex = /\x1b?\[<(\d+);?(\d+)?;?(\d+)?([Mm]?)/g;
    const legacyRegex = /\x1b?\[M([\s\S]{1,3})/g;

    const originalEmit = process.stdin.emit.bind(process.stdin);

    process.stdin.emit = function (event: string | symbol, ...args: any[]): boolean {
      if (event === 'data' && args[0]) {
        const chunk = args[0];
        const str = Buffer.isBuffer(chunk) ? chunk.toString('utf-8') : String(chunk);

        if (str.includes('[<') || str.includes('\x1b[M') || str.includes('[M')) {
          let wheelUpCount = 0;
          let wheelDownCount = 0;

          let cleaned = str.replace(sgrRegex, (_match, btn) => {
            const code = parseInt(btn, 10);
            if (code === 64 || code === 68 || code === 72 || code === 80) {
              wheelUpCount++;
            } else if (code === 65 || code === 69 || code === 73 || code === 81) {
              wheelDownCount++;
            }
            return '';
          });

          cleaned = cleaned.replace(legacyRegex, (_match, data) => {
            const btn = data.charCodeAt(0) - 32;
            if (btn === 64) wheelUpCount++;
            else if (btn === 65) wheelDownCount++;
            return '';
          });

          // Only scroll if we are in an active conversation (not on initialized/new chat welcome screen)
          if (hasActiveSessionRef.current && activeSelectorRef.current === 'none') {
            if (wheelUpCount > 0) {
              handleScrollUp(wheelUpCount * 3);
            }
            if (wheelDownCount > 0) {
              handleScrollDown(wheelDownCount * 3);
            }
          }

          if (!cleaned || !cleaned.trim()) {
            // Whole chunk was mouse escape/click sequences - absorb completely to prevent text input pollution
            return false;
          }

          const newChunk = Buffer.isBuffer(chunk) ? Buffer.from(cleaned, 'utf-8') : cleaned;
          return originalEmit('data', newChunk);
        }
      }
      return (originalEmit as any).apply(this, [event, ...args]);
    };

    return () => {
      process.stdin.emit = originalEmit;
      if (process.stdout.isTTY) {
        process.stdout.write('\x1b[?1006l\x1b[?1000l');
      }
    };
  }, []);

  useEffect(() => {
    const onResize = () => {
      setDimensions({
        rows: process.stdout.rows || 30,
        cols: process.stdout.columns || 100,
      });
    };
    process.stdout.on('resize', onResize);
    return () => {
      process.stdout.off('resize', onResize);
    };
  }, []);

  const handleToggleMode = () => {
    setMode((prev) => (prev === 'build' ? 'plan' : 'build'));
  };

  useInput((input, key) => {
    if (key.pageUp || (key.shift && key.upArrow)) {
      handleScrollUp(4);
      return;
    }
    if (key.pageDown || (key.shift && key.downArrow)) {
      handleScrollDown(4);
      return;
    }

    if (key.ctrl) {
      if (input.toLowerCase() === 'p') {
        setActiveSelector((prev) => (prev === 'command' ? 'none' : 'command'));
        return;
      }
      if (
        input.toLowerCase() === 'o' ||
        input.toLowerCase() === 'k' ||
        input.toLowerCase() === 'm'
      ) {
        setActiveSelector((prev) => (prev === 'model' ? 'none' : 'model'));
        return;
      }
      if (input.toLowerCase() === 's') {
        setActiveSelector((prev) => (prev === 'session' ? 'none' : 'session'));
        return;
      }
      if (input.toLowerCase() === 'l') {
        setActiveSelector((prev) => (prev === 'skill' ? 'none' : 'skill'));
        return;
      }
    }

    if (key.escape) {
      if (status === 'thinking' || status === 'executing') {
        abortControllerRef.current?.abort();
        setStatus('idle');
        setStreamingContent(undefined);
        setPendingToolCall(null);
        setMessages((prev) => [
          ...prev,
          {
            id: String(Date.now()),
            role: 'assistant',
            content: '*Interrupted by user (esc).*',
          },
        ]);
        return;
      }
      if (activeSelector !== 'none') {
        setActiveSelector('none');
        return;
      }
    }
  });

  // Initialize git branch and handle initial prompt if provided
  useEffect(() => {
    (async () => {
      try {
        const statusOutput = await core.gitStatus();
        const branchMatch = statusOutput.match(/On branch (\S+)/);
        if (branchMatch) {
          setGitBranch(branchMatch[1]);
        }
      } catch {
        // Not a git repo or native binding not yet loaded
      }
    })();

    if (initialPrompt) {
      handlePromptSubmit(initialPrompt);
    } else if (!hasApiKeyForModel(model) && !model.startsWith('ollama')) {
      const [prov] = model.includes(':') ? model.split(':') : ['anthropic'];
      setKeyTargetProvider(prov);
      setKeyModalNotice(`No API key configured for ${prov.toUpperCase()}. Please enter your API key to get started:`);
      setActiveSelector('key');
    }
  }, []);

  const handleCommandSelect = async (cmd: SlashCommand) => {
    if (cmd.name === '/exit') {
      process.exit(0);
    }

    if (cmd.name === '/clear') {
      setMessages([]);
      return;
    }

    if (cmd.name === '/plan') {
      setMode('plan');
      return;
    }

    if (cmd.name === '/build') {
      setMode('build');
      return;
    }

    if (cmd.name === '/model') {
      setActiveSelector('model');
      return;
    }

    if (cmd.name === '/key' || cmd.name === '/keys' || cmd.name === '/config') {
      const [prov] = model.includes(':') ? model.split(':') : ['anthropic'];
      setKeyTargetProvider(prov);
      setKeyModalNotice(undefined);
      setActiveSelector('key');
      return;
    }

    if (cmd.name === '/session' || cmd.name === '/sessions') {
      setActiveSelector('session');
      return;
    }

    if (cmd.name === '/skills' || cmd.name === '/skill') {
      setActiveSelector('skill');
      return;
    }

    if (cmd.name === '/learn') {
      await executeLearnCommand();
      return;
    }

    if (cmd.name === '/lsp') {
      setMessages((prev) => [
        ...prev,
        {
          id: String(Date.now()),
          role: 'assistant',
          content: [
            '✦ OpenCode Language Server Protocol (LSP) Status:',
            '  • Rust Server: rust-analyzer (detected in ~/.cargo/bin)',
            '  • TypeScript/JS Server: typescript-language-server (via npx / local)',
            '  • Python Server: pyright-langserver / pylsp (fallback to ripgrep)',
            '  • Active Tools:',
            '    - lsp_definition: Compiler-accurate jump-to-definition (line & column)',
            '    - lsp_references: Workspace-wide callsite & type reference tracking',
            '    - grep_search: ripgrep regex wide-search fallback',
          ].join('\n'),
        },
      ]);
      return;
    }

    if (cmd.name === '/help') {
      setMessages((prev) => [
        ...prev,
        {
          id: String(Date.now()),
          role: 'assistant',
          content: [
            'Available Slash Commands:',
            '  /plan      - Switch to Plan mode (architectural analysis and planning)',
            '  /build     - Switch to Build mode (autonomous code execution)',
            '  /skills    - Browse, search, and activate skills from ~/.agents/skills',
            '  /skill     - Activate a specific skill (e.g. /skill emil-design-eng)',
            '  /learn     - Extract recent solution into persistent .orion/skills/ skill',
            '  /lsp       - Inspect compiler-grade LSP status & semantic tools',
            '  /model     - Open interactive AI provider & model selector',
            '  /key       - Configure or update provider API keys (saved to ~/.orion/.env)',
            '  /session   - Browse, resume, or delete chat sessions from SQLite',
            '  /status    - Inspect git repository status & working tree',
            '  /diff      - View uncommitted git diffs',
            '  /clear     - Reset conversation transcript',
            '  /help      - Show this command reference',
            '  /exit      - Quit Orion CLI',
          ].join('\n'),
        },
      ]);
      return;
    }

    if (cmd.name === '/status') {
      try {
        const out = await core.gitStatus();
        setMessages((prev) => [
          ...prev,
          {
            id: String(Date.now()),
            role: 'assistant',
            content: `📊 Git Status:\n${out}`,
          },
        ]);
      } catch (err: any) {
        setMessages((prev) => [
          ...prev,
          {
            id: String(Date.now()),
            role: 'assistant',
            content: `⚠️ Failed to get git status: ${err.message}`,
          },
        ]);
      }
      return;
    }

    if (cmd.name === '/diff') {
      try {
        const diffOut = await core.gitDiff();
        setMessages((prev) => [
          ...prev,
          {
            id: String(Date.now()),
            role: 'assistant',
            content: diffOut ? `± Git Diff:\n${diffOut}` : 'No uncommitted changes detected.',
          },
        ]);
      } catch (err: any) {
        setMessages((prev) => [
          ...prev,
          {
            id: String(Date.now()),
            role: 'assistant',
            content: `⚠️ Failed to get git diff: ${err.message}`,
          },
        ]);
      }
      return;
    }
  };

  const activateSkillIntoSession = (skill: LoadedSkill) => {
    setActiveSkills((prev) => [...prev.filter((s) => s.name !== skill.name), skill]);
    setMessages((prev) => [
      ...prev,
      {
        id: String(Date.now()),
        role: 'assistant',
        content: `✦ Loaded skill: ${skill.name} [${skill.category}]\n  ${skill.description}`,
      },
    ]);
  };

  const executeLearnCommand = async (topicHint?: string) => {
    if (messages.length === 0) {
      setMessages((prev) => [
        ...prev,
        {
          id: String(Date.now()),
          role: 'assistant',
          content: '⚠️ No conversation history found. Run tasks or solve problems with Orion first, then use /learn to synthesize a persistent skill.',
        },
      ]);
      return;
    }

    setStatus('thinking');
    setCurrentTool('synthesizing_skill');

    try {
      const chatHistory: LlmMessage[] = messages
        .filter((m) => m.role === 'user' || m.role === 'assistant')
        .map((m) => ({
          role: m.role as 'user' | 'assistant',
          content: m.content,
        }));

      const res = await synthesizeSkillFromSession(chatHistory, model, topicHint);
      setMessages((prev) => [
        ...prev,
        {
          id: String(Date.now()),
          role: 'assistant',
          content: [
            '✦ Hermes Self-Synthesizing Skill Engine',
            `  • Skill: ${res.name}`,
            `  • Summary: ${res.description}`,
            `  • File: .orion/skills/${res.slug}.md`,
            '',
            '✓ Synthesized & persisted! Automatically loaded into system prompt for all future sessions.',
          ].join('\n'),
        },
      ]);
    } catch (err: any) {
      setMessages((prev) => [
        ...prev,
        {
          id: String(Date.now()),
          role: 'assistant',
          content: `⚠️ Failed to synthesize skill: ${err.message}`,
        },
      ]);
    } finally {
      setStatus('idle');
      setCurrentTool(undefined);
    }
  };

  const handlePromptSubmit = async (prompt: string) => {
    if (prompt.trim() === '/skills' || prompt.trim() === '/skill') {
      setActiveSelector('skill');
      return;
    }

    if (prompt.startsWith('/skill ') || prompt.startsWith('/skills ')) {
      const skillName = prompt.replace(/^\/skills?\s*/, '').trim();
      const targetSkill = getSkill(skillName);
      if (targetSkill) {
        activateSkillIntoSession(targetSkill);
      } else {
        const all = loadAllSkills();
        const matches = all.filter((s) => s.name.toLowerCase().includes(skillName.toLowerCase()));
        if (matches.length === 1) {
          activateSkillIntoSession(matches[0]);
        } else if (matches.length > 1) {
          setMessages((prev) => [
            ...prev,
            {
              id: String(Date.now()),
              role: 'assistant',
              content: `Multiple skills match "${skillName}":\n${matches.map((m) => `  • ${m.name} [${m.category}] - ${m.description.slice(0, 60)}`).join('\n')}\n\nType /skill <exact-name> to activate.`,
            },
          ]);
        } else {
          setMessages((prev) => [
            ...prev,
            {
              id: String(Date.now()),
              role: 'assistant',
              content: `⚠️ Skill "${skillName}" not found. Type /skills to browse all ${all.length} available skills.`,
            },
          ]);
        }
      }
      return;
    }

    if (prompt.trim() === '/lsp') {
      handleCommandSelect({ name: '/lsp', description: '' });
      return;
    }

    if (prompt.trim() === '/key' || prompt.trim() === '/keys' || prompt.trim() === '/config') {
      const [prov] = model.includes(':') ? model.split(':') : ['anthropic'];
      setKeyTargetProvider(prov);
      setKeyModalNotice(undefined);
      setActiveSelector('key');
      return;
    }

    if (prompt.startsWith('/learn')) {
      const topic = prompt.replace(/^\/learn\s*/, '').trim();
      await executeLearnCommand(topic || undefined);
      return;
    }

    if (!hasApiKeyForModel(model) && !model.startsWith('ollama')) {
      const [prov] = model.includes(':') ? model.split(':') : ['anthropic'];
      setKeyTargetProvider(prov);
      setPendingPromptAfterKey(prompt);
      setKeyModalNotice(`No API key configured for ${prov.toUpperCase()}. Please enter your API key to send your prompt.`);
      setActiveSelector('key');
      return;
    }

    const userMsg: ChatMessage = {
      id: String(Date.now()),
      role: 'user',
      content: prompt,
    };
    setMessages((prev) => [...prev, userMsg]);
    setScrollOffset(0);
    setStatus('thinking');
    setStreamingContent('');
    const startTime = Date.now();

    try {
      const chatHistory: LlmMessage[] = [];

      // Inject loaded active skills into LLM system context without printing them to the CLI
      for (const skill of activeSkills) {
        chatHistory.push({
          role: 'system',
          content: `[ACTIVE LOADED SKILL: ${skill.name} (${skill.category})]\nSource: ${skill.sourcePath}\n\n${skill.body}`,
        });
      }

      for (const m of messages) {
        if (m.role === 'user' || m.role === 'assistant') {
          chatHistory.push({
            role: m.role,
            content: m.content,
          });
        }
      }
      chatHistory.push({ role: 'user', content: prompt });

      let lastChunkTime = 0;
      let pendingChunk = '';
      let chunkTimer: NodeJS.Timeout | null = null;

      const abortController = new AbortController();
      abortControllerRef.current = abortController;

      const finalResponse = await streamChat({
        model,
        messages: chatHistory,
        mode,
        signal: abortController.signal,
        onChunk: (accumulated) => {
          pendingChunk = accumulated;
          const now = Date.now();
          // Frame-throttle React state updates to 35ms (~28 FPS) to prevent terminal re-render thrashing
          if (now - lastChunkTime >= 35) {
            lastChunkTime = now;
            setStreamingContent(accumulated);
          } else if (!chunkTimer) {
            chunkTimer = setTimeout(() => {
              lastChunkTime = Date.now();
              setStreamingContent(pendingChunk);
              chunkTimer = null;
            }, 35);
          }
        },
        onToolCall: (toolName, args) => {
          if (chunkTimer) {
            clearTimeout(chunkTimer);
            chunkTimer = null;
          }
          setStatus('executing');
          setPendingToolCall({
            id: String(Date.now()),
            toolName,
            args,
          });
          const details = args?.command
            ? `run_command: ${args.command}`
            : args?.path
            ? `${toolName}: ${args.path}`
            : toolName;
          setCurrentTool(details);
        },
        onToolResult: (toolName, result, args) => {
          setPendingToolCall(null);
          setMessages((prev) => [
            ...prev,
            {
              id: String(Date.now()),
              role: 'tool',
              toolName,
              toolArgs: args,
              content: result,
            },
          ]);
        },
      });

      if (chunkTimer) {
        clearTimeout(chunkTimer);
        chunkTimer = null;
      }
      const durationMs = Date.now() - startTime;
      setStreamingContent(undefined);
      setPendingToolCall(null);
      setMessages((prev) => [
        ...prev,
        {
          id: String(Date.now()),
          role: 'assistant',
          content: finalResponse,
          model,
          durationMs,
          mode,
        },
      ]);
    } catch (e: any) {
      setStreamingContent(undefined);
      setPendingToolCall(null);
      if (e.name !== 'AbortError' && !e.message?.includes('aborted')) {
        setMessages((prev) => [
          ...prev,
          {
            id: String(Date.now()),
            role: 'assistant',
            content: `⚠️ Error: ${e.message}`,
          },
        ]);
      }
    } finally {
      setStatus('idle');
      setCurrentTool(undefined);
      setPendingToolCall(null);
      abortControllerRef.current = null;
    }
  };

  // Calculate live token estimation and context percentage
  const totalChars = messages.reduce((sum, m) => sum + m.content.length, 0);
  const approxTokens = Math.max(150, Math.round(totalChars / 4));
  const tokensFormatted =
    approxTokens >= 1000 ? `${(approxTokens / 1000).toFixed(1)}K` : `${approxTokens}`;
  const contextPct = Math.max(1, Math.min(100, Math.round((approxTokens / 128000) * 100)));
  const costFormatted = `$${((approxTokens / 1000000) * 2.0).toFixed(2)}`;

  const isWelcomeScreen = messages.length === 0;

  // Screen 1: Welcome Screen (No conversation started yet)
  // Sticky Chat Input at bottom, centered Logo above
  if (isWelcomeScreen) {
    const unifiedCardWidth = Math.max(30, dimensions.cols - 2);

    return (
      <Box
        flexDirection="column"
        height={Math.max(10, dimensions.rows - 1)}
        width={dimensions.cols}
        justifyContent="space-between"
        paddingX={1}
        paddingY={0}
      >
        {/* Centered Upper Area: Logo */}
        <Box
          flexDirection="column"
          alignItems="center"
          justifyContent="center"
          flexGrow={1}
          overflow="hidden"
        >
          <Header
            model={model}
            cwd={cwd}
            gitBranch={gitBranch}
            version={core.version()}
            compact={false}
          />
        </Box>

        {/* Permanently Sticky Bottom Area in Welcome Screen */}
        <Box flexDirection="column" flexShrink={0} marginTop={0}>
          {/* Interactive Modal Selectors */}
          {activeSelector === 'model' && (
            <ModelSelector
              currentModel={model}
              onSelect={(selectedModel) => {
                setModel(selectedModel);
                setActiveSelector('none');
              }}
              onConfigureKey={(provId) => {
                setKeyTargetProvider(provId);
                setActiveSelector('key');
              }}
              onCancel={() => setActiveSelector('none')}
            />
          )}

          {activeSelector === 'key' && (
            <Box flexDirection="column" width={unifiedCardWidth} marginTop={0}>
              <KeyInputModal
                initialProvider={keyTargetProvider}
                notice={keyModalNotice}
                cardWidth={unifiedCardWidth}
                onSave={(providerId) => {
                  setActiveSelector('none');
                  setKeyModalNotice(undefined);
                  setMessages((prev) => [
                    ...prev,
                    {
                      id: String(Date.now()),
                      role: 'assistant',
                      content: `✓ API key for ${providerId.toUpperCase()} saved to ~/.orion/.env`,
                    },
                  ]);
                  if (pendingPromptAfterKey) {
                    const nextPrompt = pendingPromptAfterKey;
                    setPendingPromptAfterKey(null);
                    handlePromptSubmit(nextPrompt);
                  }
                }}
                onCancel={() => {
                  setActiveSelector('none');
                  setPendingPromptAfterKey(null);
                  setKeyModalNotice(undefined);
                }}
              />
            </Box>
          )}

          {activeSelector === 'session' && (
            <SessionSelector
              onSelect={(sessionId) => {
                setActiveSelector('none');
              }}
              onCancel={() => setActiveSelector('none')}
            />
          )}

          {activeSelector === 'command' && (
            <CommandPicker
              commands={COMMANDS}
              onSelect={(cmd) => {
                setActiveSelector('none');
                handleCommandSelect(cmd);
              }}
              onCancel={() => setActiveSelector('none')}
            />
          )}

          {activeSelector === 'skill' && (
            <SkillSelector
              onSelect={(selectedSkill) => {
                setActiveSelector('none');
                activateSkillIntoSession(selectedSkill);
              }}
              onCancel={() => setActiveSelector('none')}
            />
          )}

          {/* Sticky Prompt Input Card */}
          {activeSelector === 'none' && !pendingApproval && (
            <Box
              flexDirection="column"
              width={unifiedCardWidth}
              marginTop={0}
            >
              <PromptInput
                onSubmit={handlePromptSubmit}
                onSelectCommand={handleCommandSelect}
                onOpenModelSelector={() => setActiveSelector('model')}
                onOpenSessionSelector={() => setActiveSelector('session')}
                onOpenCommandPicker={() => setActiveSelector('command')}
                activeModel={model}
                mode={mode}
                onToggleMode={handleToggleMode}
                disabled={status === 'thinking' || status === 'executing'}
                showTip={false}
                placeholder='Ask anything... (e.g. "Fix a TODO")'
                cardWidth={unifiedCardWidth}
                initialHistory={messages.filter((m) => m.role === 'user').map((m) => m.content)}
                footerRight={null}
              />
            </Box>
          )}

          {/* Live Status & Loader Bar */}
          <Box
            flexDirection="row"
            justifyContent="space-between"
            alignItems="center"
            width={unifiedCardWidth}
            marginTop={0}
          >
            <Box flexDirection="row" alignItems="center">
              {status === 'thinking' || status === 'executing' ? (
                <OpenCodeLoader
                  showInterrupt={true}
                  label={status === 'executing' && currentTool ? currentTool : undefined}
                />
              ) : (
                <Box flexDirection="row">
                  <Text color="#FFFFFF">tab </Text>
                  <Text color="#71717A">{mode === 'build' ? 'plan' : 'build'}  </Text>
                  <Text color="#FFFFFF">ctrl+o </Text>
                  <Text color="#71717A">models</Text>
                </Box>
              )}
            </Box>

            <Box flexDirection="row" alignItems="center">
              <Text color="#71717A">
                {tokensFormatted} ({contextPct}%) · {costFormatted}{'  '}
              </Text>
              <Text bold color="#FFFFFF">
                ctrl+p{' '}
              </Text>
              <Text color="#71717A">commands</Text>
            </Box>
          </Box>

          {/* OpenCode Bottom Status Bar: ~\Desktop\Projects\orion-cli:main          0.1.0 */}
          <Box flexDirection="row" justifyContent="space-between" width={unifiedCardWidth} marginTop={0}>
            <Text color="#71717A">{formatCwdWithBranch(cwd, gitBranch)}</Text>
            <Text color="#71717A">{core.version()}</Text>
          </Box>
        </Box>
      </Box>
    );
  }

  // Screen 2: Active Conversation Screen (Sticky Chat Input at bottom, scrollable response above)
  const unifiedCardWidth = Math.max(30, dimensions.cols - 2);
  const bottomHeight =
    (activeSelector !== 'none' ? 14 : 0) +
    (pendingApproval ? 8 : 0) +
    6; // PromptInput (4) + Live Status & Loader Bar (1) + Directory Status bar (1)

  const availableMessageLines = Math.max(4, dimensions.rows - bottomHeight - 2);

  return (
    <Box
      flexDirection="column"
      height={Math.max(10, dimensions.rows - 1)}
      width={dimensions.cols}
      justifyContent="space-between"
      paddingX={1}
      paddingY={0}
    >
      {/* Scrollable Upper Area: Orion Response History */}
      <Box flexDirection="column" flexGrow={1} flexShrink={1} overflow="hidden">
        <MessageList
          messages={messages}
          streamingContent={streamingContent}
          pendingToolCall={pendingToolCall}
          activeModel={model}
          activeMode={mode}
          maxLines={availableMessageLines}
          scrollOffset={scrollOffset}
          cardWidth={unifiedCardWidth}
        />
      </Box>

      {/* Permanently Sticky Bottom Area: Modals, PromptInput, Status & Metrics Bar & Directory Bar */}
      <Box flexDirection="column" flexShrink={0} marginTop={0}>
        {/* Interactive Modal Selectors */}
        {activeSelector === 'model' && (
          <ModelSelector
            currentModel={model}
            onSelect={(selectedModel) => {
              setModel(selectedModel);
              setActiveSelector('none');
            }}
            onConfigureKey={(provId) => {
              setKeyTargetProvider(provId);
              setActiveSelector('key');
            }}
            onCancel={() => setActiveSelector('none')}
          />
        )}

        {activeSelector === 'key' && (
          <Box flexDirection="column" width={unifiedCardWidth} marginTop={0}>
            <KeyInputModal
              initialProvider={keyTargetProvider}
              notice={keyModalNotice}
              cardWidth={unifiedCardWidth}
              onSave={(providerId) => {
                setActiveSelector('none');
                setKeyModalNotice(undefined);
                setMessages((prev) => [
                  ...prev,
                  {
                    id: String(Date.now()),
                    role: 'assistant',
                    content: `✓ API key for ${providerId.toUpperCase()} saved to ~/.orion/.env`,
                  },
                ]);
                if (pendingPromptAfterKey) {
                  const nextPrompt = pendingPromptAfterKey;
                  setPendingPromptAfterKey(null);
                  handlePromptSubmit(nextPrompt);
                }
              }}
              onCancel={() => {
                setActiveSelector('none');
                setPendingPromptAfterKey(null);
                setKeyModalNotice(undefined);
              }}
            />
          </Box>
        )}

        {activeSelector === 'session' && (
          <SessionSelector
            onSelect={(sessionId) => {
              setActiveSelector('none');
            }}
            onCancel={() => setActiveSelector('none')}
          />
        )}

        {activeSelector === 'command' && (
          <CommandPicker
            commands={COMMANDS}
            onSelect={(cmd) => {
              setActiveSelector('none');
              handleCommandSelect(cmd);
            }}
            onCancel={() => setActiveSelector('none')}
          />
        )}

        {activeSelector === 'skill' && (
          <SkillSelector
            onSelect={(selectedSkill) => {
              setActiveSelector('none');
              activateSkillIntoSession(selectedSkill);
            }}
            onCancel={() => setActiveSelector('none')}
          />
        )}

        {pendingApproval && (
          <ToolApproval
            toolName={pendingApproval.toolName}
            args={pendingApproval.args}
            diffLines={pendingApproval.diffLines}
            onApprove={() => {
              pendingApproval.resolve(true);
              setPendingApproval(null);
            }}
            onReject={() => {
              pendingApproval.resolve(false);
              setPendingApproval(null);
            }}
          />
        )}

        {/* Signature OpenCode Input Box with Blue Left Accent Line - STICKY AT BOTTOM */}
        {activeSelector === 'none' && !pendingApproval && (
          <Box flexDirection="column" width={unifiedCardWidth} marginTop={0}>
            <PromptInput
              onSubmit={handlePromptSubmit}
              onSelectCommand={handleCommandSelect}
              onOpenModelSelector={() => setActiveSelector('model')}
              onOpenSessionSelector={() => setActiveSelector('session')}
              onOpenCommandPicker={() => setActiveSelector('command')}
              activeModel={model}
              mode={mode}
              onToggleMode={handleToggleMode}
              disabled={status === 'thinking' || status === 'executing'}
              showTip={false}
              onScrollUp={handleScrollUp}
              onScrollDown={handleScrollDown}
              cardWidth={unifiedCardWidth}
              initialHistory={messages.filter((m) => m.role === 'user').map((m) => m.content)}
              footerRight={null}
            />
          </Box>
        )}

        {/* Live Status & Loader Bar: OpenCodeLoader (Left) inline with Tokens, Status, Price & Percentage (Right) */}
        <Box
          flexDirection="row"
          justifyContent="space-between"
          alignItems="center"
          width={unifiedCardWidth}
          marginTop={0}
        >
          {/* Left: OpenCodeLoader when thinking/executing, or mode shortcuts when idle */}
          <Box flexDirection="row" alignItems="center">
            {status === 'thinking' || status === 'executing' ? (
              <OpenCodeLoader
                showInterrupt={true}
                label={status === 'executing' && currentTool ? currentTool : undefined}
              />
            ) : status === 'waiting_approval' ? (
              <Text bold color="#F59E0B">
                [Action Required] Approval pending
              </Text>
            ) : (
              <Box flexDirection="row">
                <Text color="#FFFFFF">tab </Text>
                <Text color="#71717A">{mode === 'build' ? 'plan' : 'build'}  </Text>
                <Text color="#FFFFFF">ctrl+o </Text>
                <Text color="#71717A">models</Text>
              </Box>
            )}
          </Box>

          {/* Right: Inline Tokens, Percentage, Price and Commands Shortcut */}
          <Box flexDirection="row" alignItems="center">
            <Text color="#71717A">
              {tokensFormatted} ({contextPct}%) · {costFormatted}{'  '}
            </Text>
            <Text bold color="#FFFFFF">
              ctrl+p{' '}
            </Text>
            <Text color="#71717A">commands</Text>
          </Box>
        </Box>

        {/* OpenCode Bottom Status Bar: ~\Desktop\Projects\orion-cli:main          0.1.0 */}
        <Box flexDirection="row" justifyContent="space-between" width={unifiedCardWidth} marginTop={0}>
          <Text color="#71717A">{formatCwdWithBranch(cwd, gitBranch)}</Text>
          <Text color="#71717A">{core.version()}</Text>
        </Box>
      </Box>
    </Box>
  );
};

