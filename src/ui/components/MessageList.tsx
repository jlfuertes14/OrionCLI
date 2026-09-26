import React from 'react';
import { Box, Text } from 'ink';
import { formatModelInfo } from '../theme.js';
import { LoadingHashtag } from './OpenCodeLoader.js';

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  toolName?: string;
  toolArgs?: Record<string, any>;
  model?: string;
  durationMs?: number;
  mode?: 'build' | 'plan';
}

export interface PendingToolCall {
  id: string;
  toolName: string;
  args?: Record<string, any>;
}

export interface MessageListProps {
  messages: ChatMessage[];
  streamingContent?: string;
  pendingToolCall?: PendingToolCall | null;
  activeModel?: string;
  activeMode?: 'build' | 'plan';
  maxLines?: number;
  scrollOffset?: number;
  cardWidth?: number;
}

/**
 * Format agent tool execution into a clean, minimal OpenCode 1-line label (e.g. '→ Skill "find-skills"')
 */
function formatToolLabel(
  toolName: string,
  args?: Record<string, any>,
  content?: string
): { icon: string; action: string; target: string; isError: boolean } {
  const norm = toolName.toLowerCase();
  const isErr = Boolean(
    content &&
      (content.startsWith('Error:') ||
        content.includes('[PLAN MODE SAFEGUARD]') ||
        content.includes('failed:'))
  );

  if (norm === 'use_skill' || norm === 'skill') {
    const skillName =
      args?.skill_name ||
      args?.name ||
      content?.match(/ACTIVE SKILL:\s*([^\s(]+)/)?.[1] ||
      'skill';
    return { icon: '→', action: 'Skill', target: `"${skillName}"`, isError: isErr };
  }

  if (norm === 'run_command' || norm === 'bash' || norm === 'exec') {
    const rawCmd = args?.command || '';
    const cleanCmd = rawCmd.length > 50 ? rawCmd.slice(0, 47) + '…' : rawCmd;
    return {
      icon: '→',
      action: 'Run',
      target: cleanCmd ? `"${cleanCmd}"` : (content?.includes('STDOUT') ? 'command' : ''),
      isError: isErr,
    };
  }

  if (norm === 'read_file' || norm === 'read') {
    const p = args?.path || '';
    return { icon: '→', action: 'Read', target: p ? `"${p}"` : '', isError: isErr };
  }

  if (norm === 'write_file' || norm === 'write' || norm === 'edit_file') {
    const p = args?.path || '';
    return { icon: '→', action: 'Write', target: p ? `"${p}"` : '', isError: isErr };
  }

  if (norm === 'list_directory' || norm === 'glob') {
    const p = args?.path || '.';
    return { icon: '→', action: 'List Directory', target: `"${p}"`, isError: isErr };
  }

  if (norm === 'grep_search' || norm === 'grep') {
    const q = args?.query || '';
    return { icon: '→', action: 'Search', target: q ? `"${q}"` : '', isError: isErr };
  }

  if (norm === 'lsp_definition') {
    return {
      icon: '→',
      action: 'Definition',
      target: args?.file ? `"${args.file}:${args.line}"` : '',
      isError: isErr,
    };
  }

  if (norm === 'lsp_references') {
    return {
      icon: '→',
      action: 'References',
      target: args?.file ? `"${args.file}:${args.line}"` : '',
      isError: isErr,
    };
  }

  if (norm === 'save_skill') {
    return {
      icon: '→',
      action: 'Save Skill',
      target: args?.name ? `"${args.name}"` : '',
      isError: isErr,
    };
  }

  if (norm === 'git_status') {
    return { icon: '→', action: 'Git Status', target: '', isError: isErr };
  }

  if (norm === 'git_diff') {
    return { icon: '→', action: 'Git Diff', target: '', isError: isErr };
  }

  return { icon: isErr ? '⚙' : '→', action: toolName, target: '', isError: isErr };
}

/**
 * Format human-readable comment title for tool execution (e.g. '# Search for all mobile-related skills')
 */
export function getToolDescription(
  toolName: string,
  args?: Record<string, any>,
  content?: string
): string {
  const norm = toolName.toLowerCase();
  if (norm === 'run_command' || norm === 'bash' || norm === 'exec') {
    const rawCmd = (args?.command || '').trim();
    if (/^npx\s+skills\s+find|^skills\s+find/i.test(rawCmd)) {
      const q = rawCmd.replace(/^npx\s+skills\s+find|^skills\s+find/i, '').trim();
      return q ? `Search for all ${q}-related skills` : 'Search for all skills';
    }
    if (/^npx\s+skills\s+add|^skills\s+add/i.test(rawCmd)) {
      const pkg = rawCmd.replace(/^npx\s+skills\s+add|^skills\s+add/i, '').trim();
      return `Install skill "${pkg}"`;
    }
    if (/^git\s+status/i.test(rawCmd)) return 'Check git working tree status';
    if (/^git\s+diff/i.test(rawCmd)) return 'Inspect uncommitted git diff';
    if (/^(npm\s+test|cargo\s+test|npx\s+vitest|pytest)/i.test(rawCmd)) return 'Run automated test suite';
    if (/^(npm\s+run\s+build|cargo\s+build)/i.test(rawCmd)) return 'Build and compile project';
    if (args?.description) return args.description;
    return rawCmd.length > 50 ? `Run "${rawCmd.slice(0, 47)}…"` : `Run "${rawCmd}"`;
  }
  if (norm === 'use_skill' || norm === 'skill') {
    const sName =
      args?.skill_name ||
      args?.name ||
      content?.match(/ACTIVE SKILL:\s*([^\s(]+)/)?.[1] ||
      'skill';
    return `Load and activate skill "${sName}"`;
  }
  if (norm === 'read_file' || norm === 'read') {
    return `Read file "${args?.path || ''}"`;
  }
  if (norm === 'write_file' || norm === 'write' || norm === 'edit_file') {
    return `Write file "${args?.path || ''}"`;
  }
  if (norm === 'list_directory' || norm === 'glob') {
    return `List files in "${args?.path || '.'}"`;
  }
  if (norm === 'grep_search' || norm === 'grep') {
    return `Search codebase for "${args?.query || ''}"`;
  }
  return `Execute ${toolName}`;
}

export function getCommandLine(toolName: string, args?: Record<string, any>): string {
  const norm = toolName.toLowerCase();
  if (norm === 'run_command' || norm === 'bash' || norm === 'exec') {
    return args?.command || '';
  }
  if (norm === 'use_skill' || norm === 'skill') {
    return `use_skill ${args?.skill_name || args?.name || 'skill'}`;
  }
  if (norm === 'read_file' || norm === 'read') {
    return `read ${args?.path || ''}`;
  }
  if (norm === 'write_file' || norm === 'write' || norm === 'edit_file') {
    return `write ${args?.path || ''}`;
  }
  if (norm === 'list_directory' || norm === 'glob') {
    return `ls ${args?.path || '.'}`;
  }
  if (norm === 'grep_search' || norm === 'grep') {
    return `grep "${args?.query || ''}" ${args?.path || '.'}`;
  }
  return toolName;
}

function cleanText(str: string): string {
  return str.replace(/^\*\*|\*\*$/g, '').replace(/^`|`$/g, '').trim();
}

/**
 * Tokenize line text and highlight:
 * - Bold text (**text**) in vibrant orange (#F97316)
 * - Inline code (`code`) in cyan (#38BDF8)
 * - Plain text in #E4E4E7
 */
function renderInlineText(text: string, defaultColor = '#E4E4E7'): React.ReactNode {
  const tokens = text.split(/(\*\*[^*]+\*\*|`[^`]+`)/g);

  return tokens.map((token, idx) => {
    if (!token) return null;

    if (token.startsWith('**') && token.endsWith('**') && token.length >= 4) {
      return (
        <Text key={idx} bold color="#F97316">
          {token.slice(2, -2)}
        </Text>
      );
    }

    if (token.startsWith('`') && token.endsWith('`') && token.length >= 2) {
      return (
        <Text key={idx} color="#38BDF8">
          {token.slice(1, -1)}
        </Text>
      );
    }

    return (
      <Text key={idx} color={defaultColor}>
        {token}
      </Text>
    );
  });
}

export interface ParsedSkillItem {
  skill: string;
  installs: string;
  owner: string;
  link: string;
}

export function parseSkillsFindOutput(output: string): ParsedSkillItem[] {
  const items: ParsedSkillItem[] = [];
  const lines = output.split('\n').map((l) => l.trim()).filter(Boolean);

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Pattern 1: owner/repo@skill X installs
    let match = line.match(
      /^([a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.-]+)@([a-zA-Z0-9_.-]+)\s+([0-9.]+[KkMm]?)\s*installs?/i
    );

    // Pattern 2: skill (owner/repo) X installs
    if (!match) {
      const p2 = line.match(
        /^([a-zA-Z0-9_.-]+)\s+\(([a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.-]+)\)\s*[-:]?\s*([0-9.]+[KkMm]?)\s*installs?/i
      );
      if (p2) {
        match = [p2[0], p2[2], p2[1], p2[3]];
      }
    }

    // Pattern 3: owner/repo/skill X installs
    if (!match) {
      const p3 = line.match(
        /^([a-zA-Z0-9_.-]+\/[a-zA-Z0-9_.-]+)\/([a-zA-Z0-9_.-]+)\s+([0-9.]+[KkMm]?)\s*installs?/i
      );
      if (p3) {
        match = [p3[0], p3[1], p3[2], p3[3]];
      }
    }

    if (match) {
      const owner = match[1];
      const skill = match[2];
      const installs = match[3];
      let link = `https://skills.sh/${owner}/${skill}`;

      if (i + 1 < lines.length && lines[i + 1].includes('http')) {
        const urlMatch = lines[i + 1].match(/https?:\/\/\S+/);
        if (urlMatch) {
          link = urlMatch[0];
          i++;
        }
      }

      items.push({
        skill,
        installs,
        owner,
        link: `Link (${link})`,
      });
    }
  }

  return items;
}

export function wrapCellText(text: string, width: number): string[] {
  if (!text) return [''];
  if (width <= 0) return [text];
  const lines: string[] = [];
  let remaining = text.trim();
  while (remaining.length > width) {
    let splitAt = remaining.lastIndexOf('-', width);
    if (splitAt === -1 || splitAt < Math.floor(width / 3)) {
      splitAt = remaining.lastIndexOf('/', width);
    }
    if (splitAt === -1 || splitAt < Math.floor(width / 3)) {
      splitAt = remaining.lastIndexOf('.', width);
    }
    if (splitAt === -1 || splitAt < Math.floor(width / 3)) {
      splitAt = remaining.lastIndexOf(' ', width);
    }
    if (splitAt === -1 || splitAt <= 0) {
      splitAt = width;
    } else {
      const char = remaining[splitAt];
      if (char === '-' || char === '/' || char === '.') {
        splitAt = splitAt + 1;
      }
    }
    lines.push(remaining.slice(0, splitAt).trim());
    remaining = remaining.slice(splitAt).trim();
  }
  if (remaining.length > 0) lines.push(remaining);
  return lines.length > 0 ? lines : [''];
}

export function getColumnColor(header: string, colIdx: number): string {
  const norm = header.toLowerCase();
  if (norm.includes('skill')) return '#10B981'; // Green
  if (norm.includes('install')) return '#F59E0B'; // Amber
  if (norm.includes('owner') || norm.includes('repo') || norm.includes('author')) return '#34D399'; // Mint Green
  if (norm.includes('link') || norm.includes('url')) return '#38BDF8'; // Cyan
  if (colIdx === 0) return '#10B981';
  if (colIdx === 1) return '#F59E0B';
  if (colIdx === 2) return '#34D399';
  return '#38BDF8';
}

export function renderGridTable(
  headers: string[],
  rows: string[][],
  baseKey: string,
  totalWidth: number = (process.stdout.columns || 100) - 4
): React.ReactNode[] {
  const numCols = headers.length;
  if (numCols === 0) return [];

  const isSkillsTable =
    headers.some((h) => h.toLowerCase().includes('skill')) &&
    headers.some((h) => h.toLowerCase().includes('install'));

  let colWidths: number[];

  if (isSkillsTable && numCols === 4) {
    // Exact column layout from the screenshot
    const availableWidth = Math.max(50, totalWidth - (numCols * 3 + 1));
    const skillW = 14;
    const installsW = 5;
    const ownerW = Math.max(18, Math.min(24, Math.floor(availableWidth * 0.28)));
    const linkW = Math.max(22, availableWidth - (skillW + installsW + ownerW));
    colWidths = [skillW, installsW, ownerW, linkW];
  } else {
    // Dynamic column layout for general Markdown tables
    const minWidths = headers.map((h, i) => {
      const headerLen = cleanText(h).length;
      const maxContent = Math.max(...rows.map((r) => cleanText(r[i] || '').length), 0);
      return Math.max(headerLen, Math.min(maxContent, 30));
    });

    const availableWidth = Math.max(40, totalWidth - (numCols * 3 + 1));
    const sumMin = minWidths.reduce((a, b) => a + b, 0);

    colWidths = minWidths.map((w) => {
      const ratio = w / (sumMin || 1);
      return Math.max(w, Math.min(42, Math.round(availableWidth * ratio)));
    });
  }

  const elements: React.ReactNode[] = [];

  // 1. Top border: ┌───┬───┬───┐
  const topBorder = '┌' + colWidths.map((w) => '─'.repeat(w + 2)).join('┬') + '┐';
  elements.push(
    <Box key={`${baseKey}_tbl_top`} flexDirection="row">
      <Text color="#3F3F46">{topBorder}</Text>
    </Box>
  );

  // 2. Header row (with wrapping support so e.g. "Installs" in width 6 renders "Insta\nlls"):
  const wrappedHeaders = headers.map((h, i) =>
    wrapCellText(cleanText(h || ''), colWidths[i])
  );
  const headerHeight = Math.max(...wrappedHeaders.map((w) => w.length), 1);

  for (let h = 0; h < headerHeight; h++) {
    elements.push(
      <Box key={`${baseKey}_tbl_hdr_h${h}`} flexDirection="row">
        <Text color="#3F3F46">│ </Text>
        {colWidths.map((w, colIdx) => {
          const lineText = wrappedHeaders[colIdx]?.[h] || '';
          const pad = Math.max(0, w - lineText.length);
          const isLast = colIdx === numCols - 1;
          return (
            <React.Fragment key={colIdx}>
              <Text bold color="#C084FC">
                {lineText + ' '.repeat(pad)}
              </Text>
              <Text color="#3F3F46">{isLast ? ' │' : ' │ '}</Text>
            </React.Fragment>
          );
        })}
      </Box>
    );
  }

  // 3. Header separator: ├───┼───┼───┤
  const midBorder = '├' + colWidths.map((w) => '─'.repeat(w + 2)).join('┼') + '┤';
  elements.push(
    <Box key={`${baseKey}_tbl_mid`} flexDirection="row">
      <Text color="#3F3F46">{midBorder}</Text>
    </Box>
  );

  // 4. Data rows
  rows.forEach((row, rIdx) => {
    const wrappedCells = row.map((cell, colIdx) =>
      wrapCellText(cleanText(cell || ''), colWidths[colIdx])
    );
    const rowHeight = Math.max(...wrappedCells.map((w) => w.length), 1);

    for (let h = 0; h < rowHeight; h++) {
      elements.push(
        <Box key={`${baseKey}_r${rIdx}_h${h}`} flexDirection="row">
          <Text color="#3F3F46">│ </Text>
          {colWidths.map((w, colIdx) => {
            const lineText = wrappedCells[colIdx]?.[h] || '';
            const pad = Math.max(0, w - lineText.length);
            const isLast = colIdx === numCols - 1;
            const colColor = getColumnColor(headers[colIdx] || '', colIdx);

            return (
              <React.Fragment key={colIdx}>
                <Text color={colColor}>
                  {lineText + ' '.repeat(pad)}
                </Text>
                <Text color="#3F3F46">{isLast ? ' │' : ' │ '}</Text>
              </React.Fragment>
            );
          })}
        </Box>
      );
    }

    if (rIdx < rows.length - 1) {
      elements.push(
        <Box key={`${baseKey}_r${rIdx}_sep`} flexDirection="row">
          <Text color="#3F3F46">{midBorder}</Text>
        </Box>
      );
    }
  });

  // 5. Bottom border: └───┴───┴───┘
  const botBorder = '└' + colWidths.map((w) => '─'.repeat(w + 2)).join('┴') + '┘';
  elements.push(
    <Box key={`${baseKey}_tbl_bot`} flexDirection="row">
      <Text color="#3F3F46">{botBorder}</Text>
    </Box>
  );

  return elements;
}

/**
 * Render Markdown Table cleanly into discrete lines for scrollable viewports
 */
function renderTableLines(headers: string[], rows: string[][], baseKey: string): React.ReactNode[] {
  return renderGridTable(headers, rows, baseKey);
}

/**
 * High-performance markdown parser that converts content into an array of 1-line visual nodes
 */
function renderFormattedContent(content: string): React.ReactNode[] {
  const lines = content.split('\n');
  const renderedElements: React.ReactNode[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    // 1. Code blocks (```lang ... ```)
    if (trimmed.startsWith('```')) {
      const codeLang = trimmed.slice(3).trim();
      i++;
      renderedElements.push(
        <Box
          key={`code_hdr_${i}`}
          paddingLeft={1}
          borderStyle="single"
          borderLeft={true}
          borderRight={false}
          borderTop={false}
          borderBottom={false}
          borderLeftColor="#52525B"
        >
          <Text color="#71717A">─── {codeLang || 'code'} ───</Text>
        </Box>
      );
      while (i < lines.length && !lines[i].trim().startsWith('```')) {
        const cLine = lines[i];
        renderedElements.push(
          <Box
            key={`code_ln_${i}`}
            paddingLeft={1}
            borderStyle="single"
            borderLeft={true}
            borderRight={false}
            borderTop={false}
            borderBottom={false}
            borderLeftColor="#52525B"
          >
            <Text color="#E4E4E7">{cLine || ' '}</Text>
          </Box>
        );
        i++;
      }
      if (i < lines.length && lines[i].trim().startsWith('```')) {
        i++; // skip closing ```
      }
      continue;
    }

    // 2. Markdown Table Block
    if (
      trimmed.startsWith('|') &&
      trimmed.endsWith('|') &&
      i + 1 < lines.length &&
      /^\|?(\s*:?-+:?\s*\|?)+$/.test(lines[i + 1].trim())
    ) {
      const tableLines: string[] = [];
      while (i < lines.length && lines[i].trim().startsWith('|') && lines[i].trim().endsWith('|')) {
        tableLines.push(lines[i]);
        i++;
      }

      const parseRow = (str: string) =>
        str
          .trim()
          .replace(/^\||\|$/g, '')
          .split('|')
          .map((c) => c.trim());

      const headers = parseRow(tableLines[0]);
      const rows = tableLines.slice(2).map(parseRow);

      const tableNodes = renderTableLines(headers, rows, `tbl_${i}`);
      renderedElements.push(...tableNodes);
      continue;
    }

    // 3. Horizontal Rule (--- or ***)
    if (/^---+$|^\*\*\*+$/.test(trimmed)) {
      renderedElements.push(
        <Box key={`hr_${i}`} marginY={0}>
          <Text color="#3F3F46">────────────────────────────────────────────────────────────────</Text>
        </Box>
      );
      i++;
      continue;
    }

    // 4. Headings (#, ##, ###) - Strips hashtags and displays clean bold titles
    const headingMatch = trimmed.match(/^(#{1,6})\s+(.*)$/);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const headingText = headingMatch[2].replace(/#+$/, '').trim();

      const headingColor =
        level === 1 ? '#FFFFFF' : level === 2 ? '#F97316' : '#60A5FA';

      if (renderedElements.length > 0) {
        renderedElements.push(
          <Box key={`head_sp_${i}`} marginY={0}>
            <Text>{' '}</Text>
          </Box>
        );
      }
      renderedElements.push(
        <Box key={`head_${i}`} marginY={0}>
          <Text bold color={headingColor}>
            {renderInlineText(headingText, headingColor)}
          </Text>
        </Box>
      );
      i++;
      continue;
    }

    // 5. Bullet lists with keywords (e.g. "- Keyword: rest")
    const bulletMatch = line.match(/^(\s*[-*•]\s+)([^:]+:)(.*)$/);
    if (bulletMatch) {
      const [, bullet, keyword, rest] = bulletMatch;
      renderedElements.push(
        <Box key={`bullet_${i}`} flexDirection="row">
          <Text color="#71717A">{bullet}</Text>
          <Text bold color="#F97316">
            {cleanText(keyword)}{' '}
          </Text>
          <Text color="#E4E4E7">{renderInlineText(rest)}</Text>
        </Box>
      );
      i++;
      continue;
    }

    // 6. Standard text line with inline bold & code parsing
    renderedElements.push(
      <Box key={`line_${i}`}>
        <Text>{renderInlineText(line)}</Text>
      </Box>
    );
    i++;
  }

  return renderedElements;
}

export const MessageList: React.FC<MessageListProps> = React.memo(({
  messages,
  streamingContent,
  pendingToolCall,
  activeModel = 'mistral:mistral-medium-3.5',
  activeMode = 'build',
  maxLines = 20,
  scrollOffset = 0,
  cardWidth: propCardWidth,
}) => {
  const scrollbarWidth = 2;
  const cardWidth = propCardWidth ? propCardWidth - scrollbarWidth : Math.max(30, (process.stdout.columns || 100) - 2 - scrollbarWidth);
  const innerWidth = Math.max(10, cardWidth - 3);

  // Memoize rendered message nodes for past messages so they are never re-parsed when streaming or scrolling
  const historyNodes = React.useMemo(() => {
    const nodes: React.ReactNode[] = [];

    for (const msg of messages) {
      if (msg.role === 'user') {
        const userLines = msg.content.split('\n');

        // Clean minimalist user prompt with blue accent bar without solid gray background
        userLines.forEach((uLine, uIdx) => {
          nodes.push(
            <Box
              key={`${msg.id}_${uIdx}`}
              flexDirection="row"
              width={cardWidth}
              marginY={0}
            >
              <Text color="#3B82F6" bold>█ </Text>
              <Text color="#FFFFFF" bold>{uLine}</Text>
            </Box>
          );
        });
        nodes.push(
          <Box key={`${msg.id}_bot_sp`} marginY={0}>
            <Text>{' '}</Text>
          </Box>
        );
      } else if (msg.role === 'tool') {
        const desc = getToolDescription(msg.toolName || 'tool', msg.toolArgs, msg.content);
        const cmd = getCommandLine(msg.toolName || 'tool', msg.toolArgs);
        const isError = Boolean(
          msg.content &&
            (msg.content.startsWith('Error:') ||
              msg.content.includes('[PLAN MODE SAFEGUARD]') ||
              msg.content.includes('failed:'))
        );

        const isSkillFind =
          (msg.toolArgs?.command && /skills\s+find/i.test(msg.toolArgs.command)) ||
          msg.content.includes('Install with npx skills add');

        if (isSkillFind) {
          const rawCmd = (msg.toolArgs?.command || '').trim();
          const query = rawCmd.replace(/^npx\s+skills\s+find|^skills\s+find/i, '').trim();
          const topic = query
            ? query
                .split(/\s+/)
                .map((w: string) => w.charAt(0).toUpperCase() + w.slice(1))
                .join(' ')
            : 'Mobile Design System';

          const parsedSkills = parseSkillsFindOutput(msg.content);
          const topSkills = parsedSkills.slice(0, 5);

          nodes.push(
            <Box key={msg.id} flexDirection="column" marginY={0}>
              <Box flexDirection="row">
                <Text color="#71717A"># {desc}</Text>
              </Box>
              <Box flexDirection="row">
                <Text color="#71717A">$ </Text>
                <Text color="#E4E4E7">{cmd || 'npx skills find'}</Text>
              </Box>
              <Box marginY={0}>
                <Text>{' '}</Text>
              </Box>
              <Box flexDirection="column">
                <Text bold color="#C084FC">
                  🎨 Top {topic} Skills
                </Text>
                <Text bold color="#60A5FA">
                  High-Install Skills (100K+)
                </Text>
              </Box>
              <Box marginY={0}>
                <Text>{' '}</Text>
              </Box>
              {topSkills.length > 0 ? (
                <>
                  {renderGridTable(
                    ['Skill', 'Installs', 'Owner', 'Link'],
                    topSkills.map((item) => [
                      item.skill,
                      item.installs,
                      item.owner,
                      item.link,
                    ]),
                    `${msg.id}_tbl`,
                    cardWidth
                  )}
                  {parsedSkills.length > 5 ? (
                    <Box marginY={0} paddingLeft={1}>
                      <Text color="#71717A">
                        ... +{parsedSkills.length - 5} more skills available with npx skills add
                      </Text>
                    </Box>
                  ) : null}
                </>
              ) : (
                <Box flexDirection="column">
                  <Text color="#A1A1AA">{msg.content.trim()}</Text>
                </Box>
              )}
              <Box marginY={0}>
                <Text>{' '}</Text>
              </Box>
            </Box>
          );
        } else if (cmd) {
          const outputLines = msg.content
            .split('\n')
            .map((l) => l.trim())
            .filter(Boolean);
          const hasMore = outputLines.length > 4;
          const displayLines = hasMore ? outputLines.slice(0, 3) : outputLines;

          nodes.push(
            <Box key={msg.id} flexDirection="column" marginY={0}>
              <Box flexDirection="row">
                <Text color={isError ? '#EF4444' : '#71717A'}># {desc}</Text>
              </Box>
              <Box flexDirection="row">
                <Text color="#71717A">$ </Text>
                <Text color="#E4E4E7">{cmd}</Text>
              </Box>
              {displayLines.map((line, idx) => (
                <Box key={idx}>
                  <Text color={isError ? '#EF4444' : '#A1A1AA'}>
                    {line.length > 80 ? line.slice(0, 77) + '…' : line}
                  </Text>
                </Box>
              ))}
              {hasMore && (
                <Box flexDirection="column">
                  <Text color="#71717A">...</Text>
                  <Text color="#71717A">Click to expand</Text>
                </Box>
              )}
              <Box marginY={0}>
                <Text>{' '}</Text>
              </Box>
            </Box>
          );
        } else {
          const { icon, action, target, isError: err } = formatToolLabel(
            msg.toolName || 'tool',
            msg.toolArgs,
            msg.content
          );

          nodes.push(
            <Box key={msg.id} flexDirection="row" alignItems="center" marginY={0}>
              <Text color={err ? '#EF4444' : '#71717A'}>{icon} </Text>
              <Text bold color={err ? '#EF4444' : '#D4D4D8'}>
                {action}
              </Text>
              {target ? <Text color="#A1A1AA"> {target}</Text> : null}
              {err ? <Text color="#EF4444"> [error]</Text> : null}
            </Box>
          );
        }
      } else if (msg.role === 'assistant') {
        const modelInfo = formatModelInfo(msg.model || activeModel);
        const isPlan = msg.mode === 'plan';
        const latencySec = msg.durationMs
          ? (msg.durationMs / 1000).toFixed(1) + 's'
          : '1.2s';

        const contentNodes = renderFormattedContent(msg.content);
        nodes.push(...contentNodes);

        // OpenCode Execution Pill: ▣ Build / Plan · Model · Latency
        nodes.push(
          <Box key={`${msg.id}_pill`} flexDirection="row" alignItems="center" marginY={0}>
            <Text bold color={isPlan ? '#10B981' : '#3B82F6'}>
              ▣ {isPlan ? 'Plan' : 'Build'}{' '}
            </Text>
            <Text color="#52525B">· </Text>
            <Text color="#A1A1AA">{modelInfo.displayName} </Text>
            <Text color="#52525B">· </Text>
            <Text color="#71717A">{latencySec}</Text>
          </Box>
        );
      }
    }
    return nodes;
  }, [messages, cardWidth, innerWidth, activeModel]);

  // Combine memoized history with active streaming content AND pending tool call
  const allNodes = React.useMemo(() => {
    const nodes = [...historyNodes];

    // Render active tool searching/loading (hashtag is animated before completing)
    if (pendingToolCall) {
      const desc = getToolDescription(pendingToolCall.toolName, pendingToolCall.args);
      const cmd = getCommandLine(pendingToolCall.toolName, pendingToolCall.args);

      nodes.push(
        <Box key={`pending_${pendingToolCall.id}`} flexDirection="column" marginY={0}>
          <LoadingHashtag description={desc} />
          {cmd ? (
            <Box flexDirection="row">
              <Text color="#71717A">$ </Text>
              <Text color="#E4E4E7">{cmd}</Text>
            </Box>
          ) : null}
          <Box marginY={0}>
            <Text>{' '}</Text>
          </Box>
        </Box>
      );
    }

    if (streamingContent !== undefined) {
      const streamNodes = renderFormattedContent(streamingContent);
      nodes.push(
        ...streamNodes,
        <Box key="streaming_pill" flexDirection="row" alignItems="center" marginY={0}>
          <Text bold color={activeMode === 'plan' ? '#10B981' : '#3B82F6'}>
            ▣ {activeMode === 'plan' ? 'Plan' : 'Build'}{' '}
          </Text>
          <Text color="#52525B">· </Text>
          <Text color="#A1A1AA">{formatModelInfo(activeModel).displayName} </Text>
          <Text color="#52525B">· </Text>
          <Text color="#71717A">streaming…</Text>
        </Box>
      );
    }

    return nodes;
  }, [historyNodes, pendingToolCall, streamingContent, activeMode, activeModel]);

  const totalLines = allNodes.length;
  const rawViewHeight = Math.max(4, maxLines);

  let visibleNodes: React.ReactNode[];
  let linesAbove = 0;
  let linesBelow = 0;
  let startIndex = 0;
  let endIndex = totalLines;
  let thumbTop = 0;
  let thumbBottom = 0;

  if (totalLines <= rawViewHeight) {
    visibleNodes = allNodes;
  } else {
    // Determine which scroll indicators (top/bottom) are needed
    const maxScroll = Math.max(0, totalLines - rawViewHeight);
    const clampedScroll = Math.max(0, Math.min(scrollOffset, maxScroll));
    const tentativeStart = maxScroll - clampedScroll;
    const tentativeEnd = tentativeStart + rawViewHeight;

    const needsTopHint = tentativeStart > 0;
    const needsBottomHint = totalLines - tentativeEnd > 0;
    const reservedHints = (needsTopHint ? 1 : 0) + (needsBottomHint ? 1 : 0);

    const viewHeight = Math.max(3, rawViewHeight - reservedHints);
    const effectiveMaxScroll = Math.max(0, totalLines - viewHeight);
    const effectiveScroll = Math.max(0, Math.min(scrollOffset, effectiveMaxScroll));

    startIndex = effectiveMaxScroll - effectiveScroll;
    endIndex = Math.min(totalLines, startIndex + viewHeight);

    visibleNodes = allNodes.slice(startIndex, endIndex);
    linesAbove = startIndex;
    linesBelow = totalLines - endIndex;

    // Calculate vertical scrollbar thumb
    const trackHeight = visibleNodes.length;
    const thumbHeight = Math.max(
      1,
      Math.min(trackHeight, Math.round((trackHeight / totalLines) * trackHeight))
    );
    const travelTrack = Math.max(1, trackHeight - thumbHeight);
    thumbTop = effectiveMaxScroll > 0 ? Math.round((startIndex / effectiveMaxScroll) * travelTrack) : 0;
    thumbBottom = Math.min(trackHeight, thumbTop + thumbHeight);
  }

  return (
    <Box flexDirection="column" flexGrow={1} overflow="hidden">
      {/* Scroll indicator above with mouse wheel hints */}
      {linesAbove > 0 && (
        <Box flexDirection="row" justifyContent="space-between" width="100%" paddingX={1}>
          <Text color="#71717A">
            <Text color="#F59E0B">▲ </Text>
            <Text bold color="#F59E0B">{linesAbove}</Text> lines above
            <Text color="#52525B"> · </Text>
            <Text color="#A1A1AA">Scroll Wheel ▲</Text>
            <Text color="#52525B"> / </Text>
            <Text color="#71717A">PageUp</Text>
          </Text>
          <Text color="#52525B">
            [{startIndex + 1}–{endIndex} / {totalLines} lines]
          </Text>
        </Box>
      )}

      {/* Visible Response Window with Continuous Dedicated Scrollbar Column */}
      <Box flexDirection="row" width="100%" flexGrow={1} overflow="hidden">
        <Box flexDirection="column" flexGrow={1} flexShrink={1} overflow="hidden">
          {visibleNodes}
        </Box>
        {totalLines > rawViewHeight && (
          <Box flexDirection="column" width={2} flexShrink={0} alignItems="flex-end" paddingLeft={1}>
            {Array.from({ length: visibleNodes.length }, (_, i) => {
              const isThumb = i >= thumbTop && i < thumbBottom;
              return (
                <Text key={`sb_${startIndex + i}`} bold={isThumb} color={isThumb ? '#3B82F6' : '#27272A'}>
                  {isThumb ? '█' : '│'}
                </Text>
              );
            })}
          </Box>
        )}
      </Box>

      {/* Scroll indicator below with mouse wheel hints */}
      {linesBelow > 0 && (
        <Box flexDirection="row" justifyContent="space-between" width="100%" paddingX={1}>
          <Text color="#71717A">
            <Text color="#3B82F6">▼ </Text>
            <Text bold color="#3B82F6">{linesBelow}</Text> lines below
            <Text color="#52525B"> · </Text>
            <Text color="#A1A1AA">Scroll Wheel ▼</Text>
            <Text color="#52525B"> / </Text>
            <Text color="#71717A">PageDown to return</Text>
          </Text>
          <Text color="#52525B">
            [{Math.round((endIndex / totalLines) * 100)}%]
          </Text>
        </Box>
      )}
    </Box>
  );
});
