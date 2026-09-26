import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';

export interface SlashCommand {
  name: string;
  description: string;
}

export const COMMANDS: SlashCommand[] = [
  { name: '/plan', description: 'Switch to Plan mode (read-only architectural planning)' },
  { name: '/build', description: 'Switch to Build mode (autonomous code generation & tools)' },
  { name: '/skills', description: 'Browse, search, and activate skills from ~/.agents/skills and .orion/skills' },
  { name: '/skill', description: 'Activate a specific skill into the current conversation (e.g. /skill emil-design-eng)' },
  { name: '/learn', description: 'Synthesize solution into persistent skill in .orion/skills/ (Hermes engine)' },
  { name: '/lsp', description: 'Inspect compiler-grade LSP status & semantic tools (OpenCode strategy)' },
  { name: '/model', description: 'Switch active LLM model or provider' },
  { name: '/key', description: 'Configure or update provider API keys (saved globally to ~/.orion/.env)' },
  { name: '/help', description: 'Show all available commands and shortcuts' },
  { name: '/status', description: 'Show git status and active workspace details' },
  { name: '/diff', description: 'Inspect current uncommitted git changes' },
  { name: '/clear', description: 'Clear conversation history and reset context' },
  { name: '/sessions', description: 'List and resume past session transcripts' },
  { name: '/exit', description: 'Exit Orion CLI' },
];

interface CommandPickerProps {
  commands?: SlashCommand[];
  selectedIndex?: number;
  filter?: string;
  onSelect?: (command: SlashCommand) => void;
  onCancel?: () => void;
}

export const CommandPicker: React.FC<CommandPickerProps> = ({
  commands = COMMANDS,
  selectedIndex: controlledIndex,
  filter: controlledFilter,
  onSelect,
  onCancel,
}) => {
  const isStandalone = !!onSelect;
  const [internalIndex, setInternalIndex] = useState(0);
  const [internalFilter, setInternalFilter] = useState('');

  const filter = isStandalone ? internalFilter : (controlledFilter || '');
  const cleanFilter = filter.replace(/^[/?]/, '').toLowerCase();

  const filteredCommands = commands.filter((cmd) =>
    cmd.name.toLowerCase().includes(cleanFilter) ||
    cmd.description.toLowerCase().includes(cleanFilter)
  );

  const selectedIndex = isStandalone ? internalIndex : (controlledIndex || 0);

  useInput((input, key) => {
    if (!isStandalone) return;

    if (key.escape) {
      onCancel?.();
      return;
    }

    if (key.upArrow) {
      setInternalIndex((prev) =>
        prev > 0 ? prev - 1 : Math.max(0, filteredCommands.length - 1)
      );
      return;
    }

    if (key.downArrow) {
      setInternalIndex((prev) =>
        prev < filteredCommands.length - 1 ? prev + 1 : 0
      );
      return;
    }

    if (key.return) {
      const chosen = filteredCommands[selectedIndex] || filteredCommands[0];
      if (chosen && onSelect) {
        onSelect(chosen);
      }
      return;
    }

    if (key.backspace || key.delete) {
      setInternalFilter((prev) => prev.slice(0, -1));
      setInternalIndex(0);
      return;
    }

    // Filter typing
    if (input && !key.ctrl && !key.meta) {
      setInternalFilter((prev) => prev + input);
      setInternalIndex(0);
    }
  });

  if (filteredCommands.length === 0) {
    return (
      <Box borderStyle="round" borderColor="#3F3F46" paddingX={1} marginY={0}>
        <Text color="#71717A">No commands match &quot;{filter}&quot;</Text>
      </Box>
    );
  }

  // Display at most 6 commands around the selected index to keep height small
  const MAX_VISIBLE = 6;
  let startIdx = 0;
  if (filteredCommands.length > MAX_VISIBLE) {
    if (selectedIndex >= MAX_VISIBLE) {
      startIdx = Math.min(selectedIndex - MAX_VISIBLE + 1, filteredCommands.length - MAX_VISIBLE);
    }
  }
  const visibleCommands = filteredCommands.slice(startIdx, startIdx + MAX_VISIBLE);

  return (
    <Box flexDirection="column" borderStyle="round" borderColor="#3F3F46" paddingX={1} marginY={0}>
      <Box justifyContent="space-between" marginBottom={0}>
        <Text bold color="#FFFFFF">
          Commands {filter ? `(${filter})` : ''}
        </Text>
        <Text color="#71717A">
          ↑↓ navigate • ↵ select • Esc close
        </Text>
      </Box>

      {visibleCommands.map((cmd, offset) => {
        const idx = startIdx + offset;
        const isSelected = idx === selectedIndex;
        return (
          <Box key={cmd.name} flexDirection="row">
            <Text color={isSelected ? '#3B82F6' : '#3F3F46'}>
              {isSelected ? '❯ ' : '  '}
            </Text>
            <Text bold={isSelected} color={isSelected ? '#FFFFFF' : '#A1A1AA'}>
              {cmd.name.padEnd(12)}
            </Text>
            <Text color={isSelected ? '#E4E4E7' : '#71717A'}>
              {cmd.description}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
};

