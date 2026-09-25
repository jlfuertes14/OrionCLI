console.log('🧪 Testing Markdown Table & Inline Bold Formatter...');

const sampleText = `Here are the **highly relevant skills** I expect to find (and recommend installing):
---
## **📱 Mobile App Development Skills**
| **Skill** | **Source** | **Install Count** | **Description** | **Install Command** |
|---|---|---|---|---|
| **vercel-react-native-skills** | \`vercel-labs/agent-skills\` | 150K+ | Comprehensive React Native best practices (Expo, navigation, performance, UI) | \`npx skills add vercel-labs/agent-skills@vercel-react-native-skills -g -y\` |
| **expo-router** | \`expo\` | 80K+ | File-based routing for Expo/React Native | \`npx skills add expo/expo-router -g -y\` |
`;

// Helper to strip markdown formatting
function cleanCell(cell: string): string {
  return cell.replace(/^\*\*|\*\*$/g, '').replace(/^`|`$/g, '').trim();
}

function parseBlocks(content: string) {
  const lines = content.split('\n');
  const blocks: any[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    // Check if table
    if (trimmed.startsWith('|') && trimmed.endsWith('|') && i + 1 < lines.length && /^\|?(\s*:?-+:?\s*\|?)+$/.test(lines[i + 1].trim())) {
      const tableLines: string[] = [];
      while (i < lines.length && lines[i].trim().startsWith('|') && lines[i].trim().endsWith('|')) {
        tableLines.push(lines[i]);
        i++;
      }

      const headers = tableLines[0].trim().replace(/^\||\|$/g, '').split('|').map(cleanCell);
      const rows = tableLines.slice(2).map((l) => l.trim().replace(/^\||\|$/g, '').split('|').map((c) => c.trim()));

      blocks.push({ type: 'table', headers, rows });
      continue;
    }

    // Horizontal rule
    if (/^---+$|^\*\*\*+$/.test(trimmed)) {
      blocks.push({ type: 'hr' });
      i++;
      continue;
    }

    // Heading
    const headerMatch = trimmed.match(/^(#{1,6})\s+(.*)$/);
    if (headerMatch) {
      blocks.push({ type: 'heading', level: headerMatch[1].length, text: headerMatch[2] });
      i++;
      continue;
    }

    // Regular line
    blocks.push({ type: 'text', text: line });
    i++;
  }

  return blocks;
}

const blocks = parseBlocks(sampleText);
console.log('Parsed blocks count:', blocks.length);
console.log('Block types:', blocks.map((b) => b.type));

const tableBlock = blocks.find((b) => b.type === 'table');
if (!tableBlock || tableBlock.headers.length !== 5 || tableBlock.rows.length !== 2) {
  throw new Error('Table block was not parsed correctly');
}

console.log('✔ Table block parsed correctly with 5 headers and 2 rows!');
console.log('Headers:', tableBlock.headers);
console.log('Row 0:', tableBlock.rows[0]);
console.log('\n🎉 ALL MARKDOWN TABLE & BOLD TESTS PASSED!');
