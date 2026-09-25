import React, { useState, useEffect } from 'react';
import { Box, Text } from 'ink';

interface OpenCodeLoaderProps {
  compact?: boolean;
  showInterrupt?: boolean;
  label?: string;
}

// 6 shades of blue, slate, and gray representing OpenCode's shimmering wave
const PALETTE = [
  '#3B82F6', // Bright Electric Blue
  '#60A5FA', // Sky Blue
  '#93C5FD', // Soft Blue
  '#64748B', // Slate
  '#475569', // Dark Slate
  '#334155', // Deep Slate
];

export const OpenCodeLoader: React.FC<OpenCodeLoaderProps> = React.memo(({
  compact = false,
  showInterrupt = true,
  label,
}) => {
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const timer = setInterval(() => {
      setTick((prev) => (prev + 1) % PALETTE.length);
    }, 85);
    return () => clearInterval(timer);
  }, []);

  return (
    <Box flexDirection="row" alignItems="center" marginY={0}>
      {/* 6 Shimmering Block Bars */}
      <Box flexDirection="row" marginY={0}>
        {PALETTE.map((_, idx) => {
          const colorIdx = (tick + idx) % PALETTE.length;
          return (
            <Text key={idx} color={PALETTE[colorIdx]} bold>
              █
            </Text>
          );
        })}
      </Box>

      {/* Trailing Animated Dots */}
      <Text color="#60A5FA">···</Text>

      {label ? (
        <Text color="#D4D4D8"> {label}</Text>
      ) : null}

      {/* Esc Interrupt Prompt (Image 1) */}
      {!compact && showInterrupt && (
        <Box flexDirection="row" marginLeft={2}>
          <Text bold color="#E4E4E7">
            esc{' '}
          </Text>
          <Text color="#71717A">interrupt</Text>
        </Box>
      )}
    </Box>
  );
});

const HASHTAG_PALETTE = ['#38BDF8', '#60A5FA', '#93C5FD', '#818CF8', '#A78BFA'];

export const LoadingHashtag: React.FC<{ description: string }> = React.memo(({ description }) => {
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const timer = setInterval(() => {
      setTick((prev) => (prev + 1) % HASHTAG_PALETTE.length);
    }, 90);
    return () => clearInterval(timer);
  }, []);

  const color = HASHTAG_PALETTE[tick];
  const dotColor = HASHTAG_PALETTE[(tick + 2) % HASHTAG_PALETTE.length];

  return (
    <Box flexDirection="row" alignItems="center" marginY={0}>
      <Text bold color={color}># </Text>
      <Text color="#F4F4F5">{description}</Text>
      <Text color={dotColor}> ···</Text>
    </Box>
  );
});
