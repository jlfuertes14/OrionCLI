import React from 'react';
import { Box, Text } from 'ink';

export interface DiffLine {
  tag: string; // 'insert' | 'delete' | 'equal'
  text: string;
}

interface DiffViewerProps {
  filePath?: string;
  lines: DiffLine[];
  maxLines?: number;
}

export const DiffViewer: React.FC<DiffViewerProps> = ({
  filePath,
  lines,
  maxLines = 20,
}) => {
  const displayed = lines.slice(0, maxLines);
  const remaining = lines.length - maxLines;

  return (
    <Box flexDirection="column" borderStyle="single" borderColor="gray" paddingX={1} marginY={1}>
      {filePath && (
        <Box marginBottom={1}>
          <Text bold color="yellow">
            ± Diff: {filePath}
          </Text>
        </Box>
      )}

      {displayed.map((line, idx) => {
        if (line.tag === 'insert') {
          return (
            <Text key={idx} color="green">
              + {line.text.replace(/\r?\n$/, '')}
            </Text>
          );
        }
        if (line.tag === 'delete') {
          return (
            <Text key={idx} color="red">
              - {line.text.replace(/\r?\n$/, '')}
            </Text>
          );
        }
        return (
          <Text key={idx} color="gray">
            {'  '}{line.text.replace(/\r?\n$/, '')}
          </Text>
        );
      })}

      {remaining > 0 && (
        <Box marginTop={1}>
          <Text color="gray">… and {remaining} more lines</Text>
        </Box>
      )}
    </Box>
  );
};
