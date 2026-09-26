import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import {
  SUPPORTED_PROVIDERS,
  getApiKeyForProvider,
  hasApiKeyForProvider,
  saveApiKey,
  maskApiKey,
} from '../config.js';
import {
  isTerminalBackspace,
  isTerminalForwardDelete,
  stripMouseSequences,
  isMouseSequence,
} from '../stdinTracker.js';
import { getClipboardText } from '../clipboard.js';

interface KeyInputModalProps {
  initialProvider?: string;
  onSave: (providerId: string, apiKey: string) => void;
  onCancel: () => void;
  notice?: string;
  cardWidth?: number;
}

export const KeyInputModal: React.FC<KeyInputModalProps> = ({
  initialProvider = 'anthropic',
  onSave,
  onCancel,
  notice,
  cardWidth,
}) => {
  const activeProvider =
    SUPPORTED_PROVIDERS.find((p) => p.id === initialProvider.toLowerCase()) ||
    SUPPORTED_PROVIDERS[0];

  const existingKey = getApiKeyForProvider(activeProvider.id);
  const isKeyConfigured = hasApiKeyForProvider(activeProvider.id);

  const [keyValue, setKeyValue] = useState('');
  const [cursorOffset, setCursorOffset] = useState(0);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isReady, setIsReady] = useState(false);

  // Mount debounce: ignore key events for 120ms to prevent key bleeding from previous Enter/shortcuts
  useEffect(() => {
    const timer = setTimeout(() => setIsReady(true), 120);
    return () => clearTimeout(timer);
  }, []);

  const handleSave = () => {
    const trimmed = keyValue.trim();
    if (!trimmed) {
      if (isKeyConfigured) {
        onSave(activeProvider.id, existingKey || '');
        return;
      }
      setErrorMessage(`Please enter a valid ${activeProvider.name} API key.`);
      return;
    }

    try {
      saveApiKey(activeProvider.id, trimmed);
      onSave(activeProvider.id, trimmed);
    } catch (err: any) {
      setErrorMessage(`Failed to save: ${err.message}`);
    }
  };

  useInput((input, key) => {
    if (key.ctrl && input.toLowerCase() === 'c') {
      process.stdout.write('\x1b[?1049l\x1b[?25h');
      process.exit(0);
    }

    if (!isReady) return;

    // Filter out mouse click / scroll escape sequences completely
    if (isMouseSequence(input)) {
      return;
    }

    // Escape cancels modal
    if (key.escape) {
      onCancel();
      return;
    }

    // Ctrl+V: Paste from clipboard
    if (key.ctrl && (input.toLowerCase() === 'v' || input === '\x16')) {
      const clip = getClipboardText();
      if (clip) {
        const cleaned = stripMouseSequences(clip)
          .replace(/[\r\n\x00-\x1F]/g, '')
          .trim();
        if (cleaned.length > 0) {
          setKeyValue((prev) => prev.slice(0, cursorOffset) + cleaned + prev.slice(cursorOffset));
          setCursorOffset((prev) => prev + cleaned.length);
          setErrorMessage(null);
        }
      }
      return;
    }

    // Ctrl+U: Clear line / erase input
    if (key.ctrl && (input.toLowerCase() === 'u' || input === '\x15')) {
      setKeyValue('');
      setCursorOffset(0);
      setErrorMessage(null);
      return;
    }

    // Ctrl+W: Erase word backwards
    if (key.ctrl && (input.toLowerCase() === 'w' || input === '\x17')) {
      if (cursorOffset > 0) {
        const before = keyValue.slice(0, cursorOffset);
        const after = keyValue.slice(cursorOffset);
        const trimmed = before.replace(/\s+$/, '');
        const lastSpace = trimmed.lastIndexOf(' ');
        const newBefore = lastSpace >= 0 ? trimmed.slice(0, lastSpace + 1) : '';
        setKeyValue(newBefore + after);
        setCursorOffset(newBefore.length);
        setErrorMessage(null);
      }
      return;
    }

    // Left / Right Arrow navigation
    if (key.leftArrow) {
      setCursorOffset((prev) => Math.max(0, prev - 1));
      return;
    }
    if (key.rightArrow) {
      setCursorOffset((prev) => Math.min(keyValue.length, prev + 1));
      return;
    }

    // Enter submits / saves
    if (key.return) {
      handleSave();
      return;
    }

    // Backspace: Erase character before cursor
    if (isTerminalBackspace(key, input)) {
      if (cursorOffset > 0) {
        setKeyValue((prev) => prev.slice(0, cursorOffset - 1) + prev.slice(cursorOffset));
        setCursorOffset((prev) => prev - 1);
        setErrorMessage(null);
      }
      return;
    }

    // Forward Delete: Erase character after cursor
    if (isTerminalForwardDelete(key)) {
      if (cursorOffset < keyValue.length) {
        setKeyValue((prev) => prev.slice(0, cursorOffset) + prev.slice(cursorOffset + 1));
        setErrorMessage(null);
      }
      return;
    }

    // Printable character input
    if (
      !key.ctrl &&
      !key.meta &&
      input.length > 0 &&
      !key.tab &&
      !key.return &&
      !key.escape &&
      !key.upArrow &&
      !key.downArrow &&
      !key.leftArrow &&
      !key.rightArrow &&
      !key.pageUp &&
      !key.pageDown
    ) {
      const cleaned = stripMouseSequences(input).replace(/[\r\n\x00-\x1F]/g, '');
      if (cleaned.length > 0) {
        setKeyValue((prev) => prev.slice(0, cursorOffset) + cleaned + prev.slice(cursorOffset));
        setCursorOffset((prev) => prev + cleaned.length);
        setErrorMessage(null);
      }
    }
  });

  // Mask display characters: Show first 6 and last 4 if long enough, dots in middle
  const displayChars = keyValue
    .split('')
    .map((c, i) => (i < 6 || i >= keyValue.length - 4 ? c : '•'))
    .join('');

  return (
    <Box flexDirection="column" borderStyle="round" borderColor="#3F3F46" paddingX={1} marginY={0}>
      {/* Title & Status */}
      <Box justifyContent="space-between" marginBottom={0}>
        <Box flexDirection="row">
          <Text bold color="#FFFFFF">✦ {activeProvider.name} API Key</Text>
          <Text color="#52525B"> · </Text>
          <Text color="#60A5FA">{activeProvider.envVar}</Text>
          {isKeyConfigured ? (
            <Text color="#10B981"> (Active: {maskApiKey(existingKey)})</Text>
          ) : (
            <Text color="#F59E0B"> (Not set)</Text>
          )}
        </Box>
        <Text color="#71717A">↵ save • Esc cancel</Text>
      </Box>

      {/* Provider URL Hint */}
      <Box marginTop={0}>
        <Text color="#71717A">Get key: </Text>
        <Text color="#A1A1AA">{activeProvider.dashboardUrl}</Text>
      </Box>

      {/* Notice (if present) */}
      {notice && (
        <Box marginTop={0}>
          <Text color="#F59E0B">● Notice: </Text>
          <Text color="#E4E4E7">{notice}</Text>
        </Box>
      )}

      {/* Input Field */}
      <Box marginTop={1} flexDirection="row">
        <Text color="#3B82F6" bold>❯ </Text>
        {keyValue.length === 0 ? (
          <>
            <Text inverse color="#FFFFFF"> </Text>
            <Text color="#71717A">Paste or type API key ({activeProvider.keyPlaceholder})</Text>
          </>
        ) : (
          <>
            <Text color="#FFFFFF">{displayChars.slice(0, cursorOffset)}</Text>
            <Text inverse color="#FFFFFF">{displayChars[cursorOffset] || ' '}</Text>
            <Text color="#FFFFFF">{displayChars.slice(cursorOffset + 1)}</Text>
          </>
        )}
      </Box>

      {/* Error message */}
      {errorMessage && (
        <Box marginTop={0}>
          <Text color="#EF4444">⚠️ {errorMessage}</Text>
        </Box>
      )}

      {/* Footer */}
      <Box marginTop={1} justifyContent="space-between">
        <Text color="#71717A">Ctrl+V paste • Ctrl+U clear • Ctrl+W delete word</Text>
        <Text color="#52525B">Saved to ~/.orion/.env</Text>
      </Box>
    </Box>
  );
};

