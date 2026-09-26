import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import { CommandPicker, COMMANDS, SlashCommand } from './CommandPicker.js';
import { formatModelInfo } from '../theme.js';
import { getClipboardText, setClipboardText } from '../clipboard.js';
import {
  isTerminalBackspace,
  isTerminalForwardDelete,
  stripMouseSequences,
  isMouseSequence,
} from '../stdinTracker.js';

export interface LineInfo {
  text: string;
  startIndex: number;
  endIndex: number; // exclusive
}

export function wrapTextWithIndices(text: string, maxLen: number): LineInfo[] {
  if (text.length === 0) {
    return [{ text: '', startIndex: 0, endIndex: 0 }];
  }
  const lines: LineInfo[] = [];
  let currentStart = 0;
  while (currentStart < text.length) {
    const remaining = text.length - currentStart;
    if (remaining <= maxLen) {
      lines.push({
        text: text.slice(currentStart),
        startIndex: currentStart,
        endIndex: text.length,
      });
      break;
    }
    const slice = text.slice(currentStart, currentStart + maxLen);
    const breakIndex = slice.lastIndexOf(' ');
    if (text[currentStart + maxLen] === ' ') {
      lines.push({
        text: text.slice(currentStart, currentStart + maxLen + 1),
        startIndex: currentStart,
        endIndex: currentStart + maxLen + 1,
      });
      currentStart = currentStart + maxLen + 1;
    } else if (breakIndex > 0) {
      lines.push({
        text: text.slice(currentStart, currentStart + breakIndex + 1),
        startIndex: currentStart,
        endIndex: currentStart + breakIndex + 1,
      });
      currentStart = currentStart + breakIndex + 1;
    } else {
      lines.push({
        text: text.slice(currentStart, currentStart + maxLen),
        startIndex: currentStart,
        endIndex: currentStart + maxLen,
      });
      currentStart = currentStart + maxLen;
    }
  }
  return lines;
}

function getCursorLineAndCol(lines: LineInfo[], cursorOffset: number) {
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const isLast = i === lines.length - 1;
    if (
      cursorOffset >= line.startIndex &&
      (isLast ? cursorOffset <= line.endIndex : cursorOffset < line.endIndex)
    ) {
      return { lineIndex: i, col: cursorOffset - line.startIndex };
    }
  }
  return { lineIndex: 0, col: 0 };
}

function moveCursorVertical(
  lines: LineInfo[],
  cursorOffset: number,
  direction: 'up' | 'down'
): number | null {
  const { lineIndex, col } = getCursorLineAndCol(lines, cursorOffset);
  if (direction === 'up') {
    if (lineIndex === 0) return null;
    const targetLine = lines[lineIndex - 1];
    const targetCol = Math.min(col, targetLine.text.length);
    return targetLine.startIndex + targetCol;
  } else {
    if (lineIndex === lines.length - 1) return null;
    const targetLine = lines[lineIndex + 1];
    const targetCol = Math.min(col, targetLine.text.length);
    return targetLine.startIndex + targetCol;
  }
}

interface PromptInputProps {
  onSubmit: (value: string) => void;
  onSelectCommand: (command: SlashCommand) => void;
  onOpenModelSelector: () => void;
  onOpenSessionSelector: () => void;
  onOpenCommandPicker?: () => void;
  activeModel: string;
  placeholder?: string;
  disabled?: boolean;
  showTip?: boolean;
  footerRight?: React.ReactNode;
  mode?: 'build' | 'plan';
  onToggleMode?: () => void;
  onScrollUp?: () => void;
  onScrollDown?: () => void;
  cardWidth?: number;
  initialHistory?: string[];
}

export const PromptInput: React.FC<PromptInputProps> = React.memo(({
  onSubmit,
  onSelectCommand,
  onOpenModelSelector,
  onOpenSessionSelector,
  onOpenCommandPicker,
  activeModel,
  placeholder = 'Ask anything... "Fix a TODO"',
  disabled = false,
  showTip = false,
  footerRight,
  mode = 'build',
  onToggleMode,
  onScrollUp,
  onScrollDown,
  cardWidth,
  initialHistory,
}) => {
  const [value, setValue] = useState('');
  const [cursorOffset, setCursorOffset] = useState(0);
  const [isAllSelected, setIsAllSelected] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [history, setHistory] = useState<string[]>(initialHistory || []);
  const [historyIndex, setHistoryIndex] = useState<number>(-1);
  const [draftValue, setDraftValue] = useState<string>('');

  useEffect(() => {
    if (initialHistory && initialHistory.length > 0) {
      setHistory(initialHistory);
    }
  }, [initialHistory]);

  useEffect(() => {
    if (cursorOffset > value.length) {
      setCursorOffset(value.length);
    }
  }, [value, cursorOffset]);

  const isCommandMode = value.startsWith('/') || value.startsWith('?');
  const cleanFilter = value.replace(/^[/?]/, '');

  const filteredCommands = isCommandMode
    ? COMMANDS.filter((cmd) => cmd.name.toLowerCase().includes(cleanFilter.toLowerCase()))
    : [];

  const { displayName, provider } = formatModelInfo(activeModel);

  const effectiveCardWidth = cardWidth || Math.max(30, (process.stdout.columns || 100) - 2);
  const innerWidth = Math.max(10, effectiveCardWidth - 1);
  const maxLineLength = Math.max(10, innerWidth - 5);

  const handleSubmit = (val: string) => {
    const trimmed = val.trim();
    if (!trimmed || disabled) return;

    if (trimmed.startsWith('/') || trimmed.startsWith('?')) {
      const match = COMMANDS.find(
        (c) => c.name.toLowerCase() === trimmed.toLowerCase()
      );
      const chosen =
        match ||
        filteredCommands[selectedIndex] ||
        filteredCommands[0];

      if (chosen) {
        setValue('');
        setCursorOffset(0);
        setSelectedIndex(0);
        onSelectCommand(chosen);
      }
      return;
    }

    // Save submitted prompt to history
    setHistory((prev) => {
      if (prev.length > 0 && prev[prev.length - 1] === trimmed) {
        return prev;
      }
      return [...prev, trimmed];
    });
    setHistoryIndex(-1);
    setDraftValue('');
    setValue('');
    setCursorOffset(0);
    onSubmit(trimmed);
  };

  useInput((input, key) => {
    // 0. Filter mouse click / scroll escape sequences completely
    if (isMouseSequence(input)) {
      return;
    }

    // 1. Ctrl+A: Select All text in prompt input
    if (key.ctrl && (input.toLowerCase() === 'a' || input === '\x01')) {
      if (value.length > 0) {
        setIsAllSelected(true);
      }
      return;
    }

    // 2. Ctrl+V: Paste from system clipboard
    if (key.ctrl && (input.toLowerCase() === 'v' || input === '\x16')) {
      const clip = getClipboardText();
      if (clip) {
        const cleaned = stripMouseSequences(clip)
          .replace(/\r\n|\r|\n/g, ' ')
          .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F]/g, '');
        if (cleaned.length > 0) {
          if (isAllSelected) {
            setValue(cleaned);
            setCursorOffset(cleaned.length);
            setIsAllSelected(false);
          } else {
            setValue((prev) => prev.slice(0, cursorOffset) + cleaned + prev.slice(cursorOffset));
            setCursorOffset((prev) => prev + cleaned.length);
          }
        }
      }
      return;
    }

    // 3. Ctrl shortcuts
    if (key.ctrl) {
      if (input.toLowerCase() === 'c' || input === '\x03') {
        if (isAllSelected && value.length > 0) {
          setClipboardText(value);
          return;
        }
        process.stdout.write('\x1b[?1049l\x1b[?25h');
        process.exit(0);
      }

      if (input.toLowerCase() === 'e') {
        setCursorOffset(value.length);
        return;
      }

      // Ctrl+U: Clear text before cursor (or clear input)
      if (input.toLowerCase() === 'u' || input === '\x15') {
        if (isAllSelected) {
          setValue('');
          setCursorOffset(0);
          setIsAllSelected(false);
        } else {
          setValue((prev) => prev.slice(cursorOffset));
          setCursorOffset(0);
        }
        return;
      }

      // Ctrl+W: Erase previous word
      if (input.toLowerCase() === 'w' || input === '\x17') {
        if (isAllSelected) {
          setValue('');
          setCursorOffset(0);
          setIsAllSelected(false);
          return;
        }
        if (cursorOffset > 0) {
          const before = value.slice(0, cursorOffset);
          const after = value.slice(cursorOffset);
          const trimmed = before.replace(/\s+$/, '');
          const lastSpace = trimmed.lastIndexOf(' ');
          const newBefore = lastSpace >= 0 ? trimmed.slice(0, lastSpace + 1) : '';
          setValue(newBefore + after);
          setCursorOffset(newBefore.length);
        }
        return;
      }

      // Ctrl+P opens command palette
      if (input.toLowerCase() === 'p') {
        if (onOpenCommandPicker) {
          onOpenCommandPicker();
        } else {
          setValue('/');
          setCursorOffset(1);
        }
        return;
      }

      // Ctrl+O, Ctrl+K, or Ctrl+M opens model selector
      if (
        input.toLowerCase() === 'o' ||
        input.toLowerCase() === 'k' ||
        input.toLowerCase() === 'm'
      ) {
        onOpenModelSelector();
        return;
      }

      // Ctrl+S opens saved sessions
      if (input.toLowerCase() === 's') {
        onOpenSessionSelector();
        return;
      }
    }

    if (disabled) return;

    // 4. Return / Enter
    if (key.return) {
      if (isAllSelected) {
        setIsAllSelected(false);
      }
      handleSubmit(value);
      return;
    }

    // 5. Backspace (Erase character before cursor)
    if (isTerminalBackspace(key, input)) {
      if (isAllSelected) {
        setValue('');
        setCursorOffset(0);
        setIsAllSelected(false);
        return;
      }
      if (cursorOffset > 0) {
        setValue((prev) => prev.slice(0, cursorOffset - 1) + prev.slice(cursorOffset));
        setCursorOffset((prev) => prev - 1);
      }
      return;
    }

    // 6. Forward Delete (Erase character after cursor)
    if (isTerminalForwardDelete(key)) {
      if (isAllSelected) {
        setValue('');
        setCursorOffset(0);
        setIsAllSelected(false);
        return;
      }
      if (cursorOffset < value.length) {
        setValue((prev) => prev.slice(0, cursorOffset) + prev.slice(cursorOffset + 1));
      }
      return;
    }

    // Fallback: If terminal sent raw delete and cursor is at end, treat as backspace
    if (key.delete) {
      if (isAllSelected) {
        setValue('');
        setCursorOffset(0);
        setIsAllSelected(false);
        return;
      }
      if (cursorOffset > 0) {
        setValue((prev) => prev.slice(0, cursorOffset - 1) + prev.slice(cursorOffset));
        setCursorOffset((prev) => prev - 1);
      }
      return;
    }

    // 7. Left Arrow
    if (key.leftArrow) {
      if (isAllSelected) {
        setIsAllSelected(false);
        setCursorOffset(0);
      } else {
        setCursorOffset((prev) => Math.max(0, prev - 1));
      }
      return;
    }

    // 8. Right Arrow
    if (key.rightArrow) {
      if (isAllSelected) {
        setIsAllSelected(false);
        setCursorOffset(value.length);
      } else {
        setCursorOffset((prev) => Math.min(value.length, prev + 1));
      }
      return;
    }

    // 9. PageUp / PageDown or Shift+Up/Down scrolls Orion response
    if (key.pageUp || (key.shift && key.upArrow)) {
      if (onScrollUp) {
        onScrollUp();
        return;
      }
    }

    if (key.pageDown || (key.shift && key.downArrow)) {
      if (onScrollDown) {
        onScrollDown();
        return;
      }
    }

    // 10. Up Arrow
    if (key.upArrow) {
      if (isCommandMode) {
        setSelectedIndex((prev) =>
          prev > 0 ? prev - 1 : Math.max(0, filteredCommands.length - 1)
        );
        return;
      }

      // Vertical cursor navigation across wrapped lines
      if (value.length > 0) {
        const lines = wrapTextWithIndices(value, maxLineLength);
        const nextOffset = moveCursorVertical(lines, cursorOffset, 'up');
        if (nextOffset !== null) {
          setCursorOffset(nextOffset);
          return;
        }
      }

      // Reached top line: Navigate prompt history
      if (history.length > 0) {
        if (historyIndex === -1) {
          setDraftValue(value);
          const nextIdx = history.length - 1;
          setHistoryIndex(nextIdx);
          setValue(history[nextIdx]);
          setCursorOffset(history[nextIdx].length);
        } else if (historyIndex > 0) {
          const nextIdx = historyIndex - 1;
          setHistoryIndex(nextIdx);
          setValue(history[nextIdx]);
          setCursorOffset(history[nextIdx].length);
        }
        return;
      }
      if (value === '' && onScrollUp) {
        onScrollUp();
        return;
      }
    }

    // 11. Down Arrow
    if (key.downArrow) {
      if (isCommandMode) {
        setSelectedIndex((prev) =>
          prev < filteredCommands.length - 1 ? prev + 1 : 0
        );
        return;
      }

      // Vertical cursor navigation across wrapped lines
      if (value.length > 0) {
        const lines = wrapTextWithIndices(value, maxLineLength);
        const nextOffset = moveCursorVertical(lines, cursorOffset, 'down');
        if (nextOffset !== null) {
          setCursorOffset(nextOffset);
          return;
        }
      }

      // Reached bottom line: Navigate forward in prompt history
      if (historyIndex !== -1) {
        if (historyIndex < history.length - 1) {
          const nextIdx = historyIndex + 1;
          setHistoryIndex(nextIdx);
          setValue(history[nextIdx]);
          setCursorOffset(history[nextIdx].length);
        } else {
          setHistoryIndex(-1);
          setValue(draftValue);
          setCursorOffset(draftValue.length);
        }
        return;
      }
      if (value === '' && onScrollDown) {
        onScrollDown();
        return;
      }
    }

    // 12. Tab
    if (key.tab) {
      if (isCommandMode) {
        const target = filteredCommands[selectedIndex] || filteredCommands[0];
        if (target) {
          setValue(target.name);
          setCursorOffset(target.name.length);
        }
        return;
      }
      if (onToggleMode) {
        onToggleMode();
      } else {
        onOpenModelSelector();
      }
      return;
    }

    // 13. Escape
    if (key.escape) {
      if (isCommandMode) {
        setValue('');
        setCursorOffset(0);
        setSelectedIndex(0);
      } else if (isAllSelected) {
        setIsAllSelected(false);
      }
      return;
    }

    // 14. Printable Characters & Multi-character input (pastes/typing)
    if (
      !key.ctrl &&
      !key.meta &&
      input.length > 0 &&
      !key.upArrow &&
      !key.downArrow &&
      !key.leftArrow &&
      !key.rightArrow &&
      !key.pageUp &&
      !key.pageDown &&
      !key.tab &&
      !key.return &&
      !key.escape
    ) {
      // Home / End terminal escape sequence support
      if (input === '\x1b[H' || input === '\x1b[1~' || input === '\x1bOH') {
        setCursorOffset(0);
        return;
      }
      if (input === '\x1b[F' || input === '\x1b[4~' || input === '\x1bOF') {
        setCursorOffset(value.length);
        return;
      }

      // Filter mouse escape sequences and control characters
      const cleaned = stripMouseSequences(input)
        .replace(/\r\n|\r|\n/g, ' ')
        .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F]/g, '');

      if (cleaned.length > 0) {
        if (isAllSelected) {
          setValue(cleaned);
          setCursorOffset(cleaned.length);
          setIsAllSelected(false);
        } else {
          setValue((prev) => prev.slice(0, cursorOffset) + cleaned + prev.slice(cursorOffset));
          setCursorOffset((prev) => prev + cleaned.length);
        }
        setSelectedIndex(0);
      }
    }
  });

  const accentColor = mode === 'plan' ? '#10B981' : '#3B82F6';

  return (
    <Box flexDirection="column" marginY={0}>
      {isCommandMode && (
        <CommandPicker
          commands={filteredCommands}
          selectedIndex={selectedIndex}
          filter={value}
        />
      )}

      {/* OpenCode Signature Input with Continuous Left Accent Block */}
      <Box flexDirection="column" width={effectiveCardWidth} marginY={0}>
        {/* Text Input Row(s): Supports word-wrapped multiline without layout distortion */}
        {value.length === 0 ? (
          (() => {
            const pLines = wrapTextWithIndices(placeholder || '', maxLineLength);
            return pLines.map((pLine, idx) => {
              const isFirst = idx === 0;
              return (
                <Box key={`placeholder-${idx}`} flexDirection="row" width={effectiveCardWidth} marginY={0}>
                  <Text color={accentColor} bold>█ </Text>
                  {disabled ? (
                    <Text color="#71717A">{pLine.text || 'Thinking…'}</Text>
                  ) : isFirst ? (
                    <>
                      <Text inverse color="#FFFFFF">
                        {pLine.text.charAt(0) || ' '}
                      </Text>
                      <Text color="#71717A">{pLine.text.slice(1)}</Text>
                    </>
                  ) : (
                    <Text color="#71717A">{pLine.text}</Text>
                  )}
                </Box>
              );
            });
          })()
        ) : (
          (() => {
            const lines = wrapTextWithIndices(value, maxLineLength);
            return lines.map((line, idx) => {
              const isLastLine = idx === lines.length - 1;
              const hasCursor = !disabled && (
                cursorOffset >= line.startIndex &&
                (isLastLine ? cursorOffset <= line.endIndex : cursorOffset < line.endIndex)
              );
              const cursorInLine = cursorOffset - line.startIndex;

              let content: React.ReactNode = <Text color="#FFFFFF">{line.text}</Text>;
              if (isAllSelected) {
                content = <Text inverse color="#FFFFFF">{line.text}</Text>;
              } else if (hasCursor) {
                if (cursorInLine < line.text.length) {
                  content = (
                    <>
                      <Text color="#FFFFFF">{line.text.slice(0, cursorInLine)}</Text>
                      <Text inverse color="#FFFFFF">{line.text[cursorInLine]}</Text>
                      <Text color="#FFFFFF">{line.text.slice(cursorInLine + 1)}</Text>
                    </>
                  );
                } else {
                  content = (
                    <>
                      <Text color="#FFFFFF">{line.text}</Text>
                      <Text inverse color="#FFFFFF"> </Text>
                    </>
                  );
                }
              }

              return (
                <Box key={`input-line-${idx}`} flexDirection="row" width={effectiveCardWidth} marginY={0}>
                  <Text color={accentColor} bold>█ </Text>
                  {content}
                </Box>
              );
            });
          })()
        )}

        {/* Status Line: Build / Plan · Model Provider */}
        <Box flexDirection="row" width={effectiveCardWidth} marginY={0}>
          <Text color={accentColor} bold>█ </Text>
          <Text bold color={mode === 'plan' ? '#10B981' : '#60A5FA'}>
            {mode === 'plan' ? 'Plan' : 'Build'}
          </Text>
          <Text color="#52525B"> · </Text>
          <Text bold color="#FFFFFF">
            {displayName}
          </Text>
          <Text color="#71717A"> {provider}</Text>
        </Box>
      </Box>

      {/* Sub-card shortcut or metrics row (Right-aligned under card) */}
      {footerRight !== undefined ? (
        footerRight ? (
          <Box justifyContent="flex-end" width={effectiveCardWidth} marginTop={0}>
            {footerRight}
          </Box>
        ) : null
      ) : showTip ? null : (
        <Box justifyContent="flex-end" width={effectiveCardWidth} marginTop={0}>
          <Box flexDirection="row">
            <Text color="#FFFFFF">tab </Text>
            <Text color="#71717A">{mode === 'build' ? 'plan' : 'build'}  </Text>
            <Text color="#FFFFFF">ctrl+p </Text>
            <Text color="#71717A">commands  </Text>
            <Text color="#FFFFFF">ctrl+o </Text>
            <Text color="#71717A">models</Text>
          </Box>
        </Box>
      )}

      {/* Tip Banner (when in welcome/idle screen) */}
      {showTip && (
        <Box marginTop={1} justifyContent="center">
          <Text color="#F59E0B">● Tip</Text>
          <Text color="#71717A"> Press </Text>
          <Text bold color="#FFFFFF">
            Tab
          </Text>
          <Text color="#71717A"> to toggle {mode === 'build' ? 'Plan' : 'Build'} mode, </Text>
          <Text bold color="#FFFFFF">
            ctrl+p
          </Text>
          <Text color="#71717A"> for commands, or </Text>
          <Text bold color="#FFFFFF">
            ctrl+o
          </Text>
          <Text color="#71717A"> for models</Text>
        </Box>
      )}
    </Box>
  );
});
