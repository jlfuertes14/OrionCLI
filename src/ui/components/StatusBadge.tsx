import React from 'react';
import { Box, Text } from 'ink';
import { OpenCodeLoader } from './OpenCodeLoader.js';

export type AgentStatus = 'idle' | 'thinking' | 'executing' | 'waiting_approval' | 'done' | 'error';

interface StatusBadgeProps {
  status: AgentStatus;
  currentTool?: string;
  tokens?: { prompt: number; completion: number };
  latencyMs?: number;
}

export const StatusBadge: React.FC<StatusBadgeProps> = React.memo(({
  status,
  currentTool,
  tokens,
  latencyMs,
}) => {
  return (
    <Box flexDirection="row" alignItems="center" marginTop={0}>
      {(status === 'thinking' || status === 'executing') && (
        <Box marginRight={2}>
          <OpenCodeLoader
            label={status === 'executing' && currentTool ? currentTool : undefined}
          />
        </Box>
      )}

      {status === 'waiting_approval' && (
        <Box marginRight={2}>
          <Text bold color="#FFFFFF">
            [Action Required] Approval pending
          </Text>
        </Box>
      )}

      {tokens && (
        <Box flexGrow={1} justifyContent="flex-end">
          <Text color="gray">
            Tokens: {tokens.prompt + tokens.completion} ({tokens.prompt} in / {tokens.completion} out)
          </Text>
          {latencyMs !== undefined && (
            <Text color="gray"> │ {latencyMs}&nbsp;ms</Text>
          )}
        </Box>
      )}
    </Box>
  );
});
