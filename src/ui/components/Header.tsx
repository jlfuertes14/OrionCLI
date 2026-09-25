import React from 'react';
import { Box, Text } from 'ink';

// Scaled Braille Rocket Ship (8 lines, proportional monochrome shading)
const SPACESHIP = [
  " ⡟⣹⠟⡻⢶⢤⣀",
  " ⣿⣡⢎⣴⣦⣄⠙⢳⣦⡀",
  " ⠹⣇⠸⣿⣿⣏⣤⡀⠙⣷⣖⣒⣲⣦⣄",
  "  ⠻⣆⡀ ⢿⣿⡿⣠⠞⣿⡆  ⢹⡄",
  "   ⠙⣿⣦⡀⢀⡼⣿⡾⣥⡟⠛⠳⣌⣧",
  "    ⠈⢻⣷⢯⣾⣉⡷⣼⣷⣦⣄⠘⠿",
  "     ⢸⣿ ⠈⢿ ⠹⣿⣿⣿⢦⡀",
  "     ⠈⠻⢦⣤⣈⣳⣄⠈⠙⠺⣭⣷",
];

const ORION_TITLE = [
  " ██████╗  ██████╗  ██╗  ██████╗  ███╗   ██╗",
  "██╔═══██╗ ██╔══██╗ ██║ ██╔═══██╗ ████╗  ██║",
  "██║   ██║ ██████╔╝ ██║ ██║   ██║ ██╔██╗ ██║",
  "██║   ██║ ██╔══██╗ ██║ ██║   ██║ ██║╚██╗██║",
  "╚██████╔╝ ██║  ██║ ██║ ╚██████╔╝ ██║ ╚████║",
  " ╚═════╝  ╚═╝  ╚═╝ ╚═╝  ╚═════╝  ╚═╝  ╚═══╝",
];

interface HeaderProps {
  model: string;
  cwd: string;
  gitBranch?: string;
  version?: string;
  compact?: boolean;
}

export const Header: React.FC<HeaderProps> = React.memo(({
  compact = false,
}) => {
  // In OpenCode, during conversation (Image 1), no top header is shown to maximize terminal focus
  if (compact) {
    return null;
  }

  // Welcome Screen (Image 2): Centered, borderless floating block logo & spaceship
  return (
    <Box
      flexDirection="column"
      alignItems="center"
      justifyContent="center"
      marginY={1}
      paddingX={1}
    >
      <Box flexDirection="row" alignItems="center" justifyContent="center">
        {/* Scaled Rocket Ship Braille Art */}
        <Box flexDirection="column" marginRight={2}>
          {SPACESHIP.map((line, idx) => {
            const color = idx < 3 ? '#FFFFFF' : idx < 6 ? '#D4D4D8' : '#71717A';
            return (
              <Text key={idx} color={color}>
                {line}
              </Text>
            );
          })}
        </Box>

        {/* Shaded Orion Geometric Block Title */}
        <Box flexDirection="column">
          {ORION_TITLE.map((line, idx) => (
            <Text key={idx} bold color="#FFFFFF">
              {line}
            </Text>
          ))}
        </Box>
      </Box>
    </Box>
  );
});

