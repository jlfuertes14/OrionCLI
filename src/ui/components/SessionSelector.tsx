import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import { core } from '../core.js';

export interface SessionItem {
  id: string;
  title: string;
  provider: string;
  model: string;
  updated_at: string;
}

interface SessionSelectorProps {
  onSelect: (sessionId: string) => void;
  onCancel: () => void;
}

export const SessionSelector: React.FC<SessionSelectorProps> = ({
  onSelect,
  onCancel,
}) => {
  const [sessions, setSessions] = useState<SessionItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(true);
  const [isReady, setIsReady] = useState(false);

  // Mount debounce: ignore key events for 150ms to stop key bleeding from CommandPicker
  useEffect(() => {
    const timer = setTimeout(() => setIsReady(true), 150);
    return () => clearTimeout(timer);
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const list = await core.listSessions(15);
        setSessions(list);
      } catch (e) {
        setSessions([]);
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  useInput((input, key) => {
    if (key.ctrl && input.toLowerCase() === 'c') {
      process.stdout.write('\x1b[?1049l\x1b[?25h');
      process.exit(0);
    }

    if (!isReady) return;

    if (key.escape) {
      onCancel();
      return;
    }

    if (sessions.length === 0) return;

    if (key.upArrow) {
      setSelectedIndex((prev) => (prev > 0 ? prev - 1 : sessions.length - 1));
    } else if (key.downArrow) {
      setSelectedIndex((prev) => (prev < sessions.length - 1 ? prev + 1 : 0));
    } else if (key.return) {
      if (sessions[selectedIndex]) {
        onSelect(sessions[selectedIndex].id);
      }
    } else if (input.toLowerCase() === 'd' || key.delete) {
      const target = sessions[selectedIndex];
      if (target) {
        core.deleteSession(target.id).then(() => {
          setSessions((prev) => prev.filter((s) => s.id !== target.id));
          setSelectedIndex((prev) => Math.max(0, prev - 1));
        });
      }
    }
  });

  return (
    <Box flexDirection="column" borderStyle="round" borderColor="#3F3F46" paddingX={1} marginY={0}>
      <Box justifyContent="space-between">
        <Text bold color="#FFFFFF">
          Chat Sessions
        </Text>
        <Text color="#71717A">
          ↑↓ navigate • ↵ resume • d delete • Esc cancel
        </Text>
      </Box>

      {loading && (
        <Box>
          <Text color="#71717A">Loading sessions from history…</Text>
        </Box>
      )}

      {!loading && sessions.length === 0 && (
        <Box>
          <Text color="#71717A">No saved sessions found. Start chatting to save a session!</Text>
        </Box>
      )}

      {!loading &&
        sessions.map((sess, idx) => {
          const isSelected = idx === selectedIndex;
          const shortId = sess.id.length > 8 ? sess.id.slice(0, 8) : sess.id;
          const shortTitle = sess.title.length > 36 ? sess.title.slice(0, 34) + '…' : sess.title;

          return (
            <Box key={sess.id} flexDirection="row" justifyContent="space-between">
              <Box>
                <Text color={isSelected ? '#FFFFFF' : '#3F3F46'}>
                  {isSelected ? '❯ ' : '  '}
                </Text>
                <Text bold={isSelected} color={isSelected ? '#FFFFFF' : '#D4D4D8'}>
                  {shortId}{' '}
                </Text>
                <Text bold={isSelected} color={isSelected ? '#FFFFFF' : '#A1A1AA'}>
                  {shortTitle.padEnd(38)}
                </Text>
              </Box>
              <Box>
                <Text color="#71717A">{sess.updated_at} </Text>
                <Text color={isSelected ? '#FFFFFF' : '#71717A'}>[{sess.model}]</Text>
              </Box>
            </Box>
          );
        })}
    </Box>
  );
};
