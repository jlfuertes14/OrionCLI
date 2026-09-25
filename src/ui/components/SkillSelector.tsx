import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import { loadAllSkills, LoadedSkill } from '../skills.js';

interface SkillSelectorProps {
  onSelect: (skill: LoadedSkill) => void;
  onCancel: () => void;
}

const PAGE_SIZE = 8;

export const SkillSelector: React.FC<SkillSelectorProps> = ({ onSelect, onCancel }) => {
  const [skills, setSkills] = useState<LoadedSkill[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [filter, setFilter] = useState('');
  const [isReady, setIsReady] = useState(false);

  // Mount debounce to stop key bleeding
  useEffect(() => {
    const timer = setTimeout(() => setIsReady(true), 150);
    return () => clearTimeout(timer);
  }, []);

  useEffect(() => {
    const all = loadAllSkills();
    setSkills(all);
  }, []);

  const cleanFilter = filter.toLowerCase().trim();
  const filteredSkills = skills.filter(
    (s) =>
      s.name.toLowerCase().includes(cleanFilter) ||
      s.description.toLowerCase().includes(cleanFilter) ||
      s.category.toLowerCase().includes(cleanFilter)
  );

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

    if (key.upArrow) {
      setSelectedIndex((prev) =>
        prev > 0 ? prev - 1 : Math.max(0, filteredSkills.length - 1)
      );
      return;
    }

    if (key.downArrow) {
      setSelectedIndex((prev) =>
        prev < filteredSkills.length - 1 ? prev + 1 : 0
      );
      return;
    }

    if (key.return) {
      const chosen = filteredSkills[selectedIndex] || filteredSkills[0];
      if (chosen) {
        onSelect(chosen);
      }
      return;
    }

    if (key.backspace || key.delete) {
      setFilter((prev) => prev.slice(0, -1));
      setSelectedIndex(0);
      return;
    }

    if (input && !key.ctrl && !key.meta) {
      setFilter((prev) => prev + input);
      setSelectedIndex(0);
    }
  });

  const agentsCount = skills.filter((s) => s.category === '.agents').length;
  const orionCount = skills.filter((s) => s.category === '.orion' || s.category === 'workspace').length;

  // Calculate sliding window so selectedIndex is always within view
  let windowStart = 0;
  if (filteredSkills.length > PAGE_SIZE) {
    if (selectedIndex >= PAGE_SIZE) {
      windowStart = Math.min(
        selectedIndex - PAGE_SIZE + 1,
        filteredSkills.length - PAGE_SIZE
      );
    }
  }
  const visibleSkills = filteredSkills.slice(windowStart, windowStart + PAGE_SIZE);

  return (
    <Box flexDirection="column" borderStyle="round" borderColor="#3F3F46" paddingX={1} marginY={0}>
      {/* Compact single-line header */}
      <Box justifyContent="space-between" marginBottom={0}>
        <Text bold color="#FFFFFF">
          ✦ Skills Library <Text color="#71717A">({skills.length} available)</Text>
        </Text>
        <Text color="#71717A">↑↓ scroll • ↵ activate • Esc exit</Text>
      </Box>

      {/* Filter search bar */}
      <Box flexDirection="row" marginY={0}>
        <Text color="#3B82F6">Filter: </Text>
        <Text color="#FFFFFF">{filter}</Text>
        <Text color="#3B82F6">_</Text>
      </Box>

      {filteredSkills.length === 0 ? (
        <Box marginY={1}>
          <Text color="#71717A">No skills match &quot;{filter}&quot;.</Text>
        </Box>
      ) : (
        <Box flexDirection="column" marginTop={1}>
          {visibleSkills.map((skill, idx) => {
            const actualIndex = windowStart + idx;
            const isSelected = actualIndex === selectedIndex;

            const categoryBadgeColor =
              skill.category === '.agents'
                ? '#10B981'
                : skill.category === 'workspace'
                ? '#F59E0B'
                : '#3B82F6';

            const badgeText =
              skill.category === '.agents'
                ? '[.agents]  '
                : skill.category === 'workspace'
                ? '[workspace]'
                : '[.orion]   ';

            // Clean 1-line truncation
            const displayName =
              skill.name.length > 27
                ? skill.name.slice(0, 26) + '…'
                : skill.name.padEnd(28);

            const cleanDesc = skill.description.replace(/[\r\n]+/g, ' ').trim();
            const displayDesc =
              cleanDesc.length > 40 ? cleanDesc.slice(0, 38) + '…' : cleanDesc;

            return (
              <Box key={skill.name + actualIndex} flexDirection="column">
                <Box flexDirection="row" alignItems="center">
                  <Text color={isSelected ? '#3B82F6' : '#52525B'}>
                    {isSelected ? '❯ ' : '  '}
                  </Text>
                  <Text bold={isSelected} color={isSelected ? '#FFFFFF' : '#D4D4D8'}>
                    {displayName}
                  </Text>
                  <Text color={categoryBadgeColor}> {badgeText} </Text>
                  <Text color={isSelected ? '#A1A1AA' : '#71717A'}>
                    {displayDesc}
                  </Text>
                </Box>
                {isSelected && (
                  <Box paddingLeft={4}>
                    <Text color="#52525B" italic>
                      ↳ {skill.sourcePath}
                    </Text>
                  </Box>
                )}
              </Box>
            );
          })}
        </Box>
      )}

      {/* Footer status showing scrolling position */}
      {filteredSkills.length > 0 && (
        <Box justifyContent="space-between" marginTop={1}>
          <Text color="#71717A">
            Showing {windowStart + 1}–{Math.min(windowStart + PAGE_SIZE, filteredSkills.length)} of {filteredSkills.length}
          </Text>
          <Text color="#52525B">
            {agentsCount} .agents • {orionCount} orion
          </Text>
        </Box>
      )}
    </Box>
  );
};
