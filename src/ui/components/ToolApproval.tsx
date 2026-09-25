import React from 'react';
import { Box, Text, useInput } from 'ink';
import { DiffViewer, DiffLine } from './DiffViewer.js';

interface ToolApprovalProps {
  toolName: string;
  args: Record<string, any>;
  diffLines?: DiffLine[];
  onApprove: () => void;
  onReject: () => void;
  onExplain?: () => void;
}

export const ToolApproval: React.FC<ToolApprovalProps> = ({
  toolName,
  args,
  diffLines,
  onApprove,
  onReject,
  onExplain,
}) => {
  useInput((input, key) => {
    if (key.ctrl && input.toLowerCase() === 'c') {
      process.stdout.write('\x1b[?1049l\x1b[?25h');
      process.exit(0);
    }

    if (input.toLowerCase() === 'y' || key.return) {
      onApprove();
    } else if (input.toLowerCase() === 'n' || key.escape) {
      onReject();
    } else if (input.toLowerCase() === 'e' && onExplain) {
      onExplain();
    }
  });

  return (
    <Box flexDirection="column" borderStyle="round" borderColor="#3F3F46" paddingX={1} marginY={0}>
      <Box>
        <Text bold color="#FFFFFF">✦ Tool Approval Request: </Text>
        <Text bold color="#E4E4E7">{toolName}</Text>
      </Box>

      {diffLines && diffLines.length > 0 ? (
        <DiffViewer filePath={args.path} lines={diffLines} />
      ) : (
        <Box marginY={0} flexDirection="column">
          <Text color="#71717A">Arguments:</Text>
          <Text color="#D4D4D8">
            {JSON.stringify(args, null, 2)}
          </Text>
        </Box>
      )}

      <Box marginTop={0}>
        <Text color="#FFFFFF" bold>[y] Approve</Text>
        <Text color="#52525B"> │ </Text>
        <Text color="#A1A1AA" bold>[n] Reject</Text>
        {onExplain && (
          <>
            <Text color="#52525B"> │ </Text>
            <Text color="#71717A" bold>[e] Explain</Text>
          </>
        )}
      </Box>
    </Box>
  );
};
