import fs from 'fs';
import path from 'path';
import os from 'os';

export interface LoadedSkill {
  name: string;
  description: string;
  body: string;
  sourcePath: string;
  category: '.agents' | '.orion' | 'workspace';
}

function parseMarkdownSkill(content: string, filePath: string, category: '.agents' | '.orion' | 'workspace'): LoadedSkill | null {
  const parts = content.split('---');
  let name = '';
  let description = '';
  let body = content.trim();

  if (parts.length >= 3) {
    const frontmatter = parts[1];
    body = parts.slice(2).join('---').trim();

    for (const line of frontmatter.split('\n')) {
      const trimmed = line.trim();
      if (trimmed.startsWith('name:')) {
        name = trimmed.slice(5).trim().replace(/^["']|["']$/g, '');
      } else if (trimmed.startsWith('description:')) {
        description = trimmed.slice(12).trim().replace(/^["']|["']$/g, '');
      }
    }
  }

  if (!name) {
    // Fallback: directory name or filename
    const parentDir = path.basename(path.dirname(filePath));
    const filename = path.basename(filePath, path.extname(filePath));
    name = filename.toLowerCase() === 'skill' ? parentDir : filename;
  }

  if (!description) {
    // Extract first non-heading sentence as description if available
    const lines = body.split('\n').map((l) => l.trim()).filter((l) => l && !l.startsWith('#'));
    description = lines[0]?.slice(0, 120) || `Skill: ${name}`;
  }

  return {
    name,
    description,
    body,
    sourcePath: filePath,
    category,
  };
}

function parseTomlSkill(content: string, filePath: string, category: '.agents' | '.orion' | 'workspace'): LoadedSkill | null {
  const filename = path.basename(filePath, path.extname(filePath));
  let name = filename;
  let description = '';
  let body = '';

  const nameMatch = content.match(/name\s*=\s*["']([^"']+)["']/);
  if (nameMatch) name = nameMatch[1];

  const descMatch = content.match(/description\s*=\s*["']([^"']+)["']/);
  if (descMatch) description = descMatch[1];

  const injectMatch = content.match(/inject\s*=\s*"""([\s\S]*?)"""/);
  if (injectMatch) {
    body = injectMatch[1].trim();
  } else {
    const singleInject = content.match(/inject\s*=\s*["']([^"']+)["']/);
    if (singleInject) body = singleInject[1].trim();
  }

  return {
    name,
    description: description || `Skill: ${name}`,
    body: body || content.trim(),
    sourcePath: filePath,
    category,
  };
}

/**
 * Scan all available skills across:
 * 1. Workspace .agents/skills/ and .orion/skills/
 * 2. User ~/.agents/skills/
 * 3. User ~/.orion/skills/
 */
export function loadAllSkills(workspaceDir: string = process.cwd()): LoadedSkill[] {
  const skillsMap = new Map<string, LoadedSkill>();

  // 1. User ~/.agents/skills/ (The primary user skills library)
  const homeAgentsSkills = path.join(os.homedir(), '.agents', 'skills');
  if (fs.existsSync(homeAgentsSkills)) {
    try {
      const subdirs = fs.readdirSync(homeAgentsSkills);
      for (const sub of subdirs) {
        const fullDir = path.join(homeAgentsSkills, sub);
        try {
          if (!fs.statSync(fullDir).isDirectory()) continue;
          const skillMd = path.join(fullDir, 'SKILL.md');
          if (fs.existsSync(skillMd)) {
            const content = fs.readFileSync(skillMd, 'utf-8');
            const parsed = parseMarkdownSkill(content, skillMd, '.agents');
            if (parsed) skillsMap.set(parsed.name, parsed);
          }
        } catch {}
      }
    } catch {}
  }

  // 2. User ~/.orion/skills/ (*.toml, *.md)
  const homeOrionSkills = path.join(os.homedir(), '.orion', 'skills');
  if (fs.existsSync(homeOrionSkills)) {
    try {
      const files = fs.readdirSync(homeOrionSkills);
      for (const file of files) {
        const fullPath = path.join(homeOrionSkills, file);
        if (!fs.statSync(fullPath).isFile()) continue;
        const ext = path.extname(file).toLowerCase();
        try {
          const content = fs.readFileSync(fullPath, 'utf-8');
          if (ext === '.md') {
            const parsed = parseMarkdownSkill(content, fullPath, '.orion');
            if (parsed) skillsMap.set(parsed.name, parsed);
          } else if (ext === '.toml') {
            const parsed = parseTomlSkill(content, fullPath, '.orion');
            if (parsed) skillsMap.set(parsed.name, parsed);
          }
        } catch {}
      }
    } catch {}
  }

  // 3. Workspace .agents/skills/
  const wsAgentsSkills = path.join(workspaceDir, '.agents', 'skills');
  if (fs.existsSync(wsAgentsSkills)) {
    try {
      const subdirs = fs.readdirSync(wsAgentsSkills);
      for (const sub of subdirs) {
        const fullDir = path.join(wsAgentsSkills, sub);
        try {
          if (!fs.statSync(fullDir).isDirectory()) continue;
          const skillMd = path.join(fullDir, 'SKILL.md');
          if (fs.existsSync(skillMd)) {
            const content = fs.readFileSync(skillMd, 'utf-8');
            const parsed = parseMarkdownSkill(content, skillMd, 'workspace');
            if (parsed) skillsMap.set(parsed.name, parsed);
          }
        } catch {}
      }
    } catch {}
  }

  // 4. Workspace .orion/skills/ (Synthesized skills have top priority)
  const wsOrionSkills = path.join(workspaceDir, '.orion', 'skills');
  if (fs.existsSync(wsOrionSkills)) {
    try {
      const files = fs.readdirSync(wsOrionSkills);
      for (const file of files) {
        const fullPath = path.join(wsOrionSkills, file);
        if (!fs.statSync(fullPath).isFile()) continue;
        const ext = path.extname(file).toLowerCase();
        try {
          const content = fs.readFileSync(fullPath, 'utf-8');
          if (ext === '.md') {
            const parsed = parseMarkdownSkill(content, fullPath, 'workspace');
            if (parsed) skillsMap.set(parsed.name, parsed);
          } else if (ext === '.toml') {
            const parsed = parseTomlSkill(content, fullPath, 'workspace');
            if (parsed) skillsMap.set(parsed.name, parsed);
          }
        } catch {}
      }
    } catch {}
  }

  return Array.from(skillsMap.values()).sort((a, b) => a.name.localeCompare(b.name));
}

/**
 * Retrieve a specific skill by exact or partial name
 */
export function getSkill(name: string, workspaceDir: string = process.cwd()): LoadedSkill | undefined {
  const all = loadAllSkills(workspaceDir);
  const clean = name.toLowerCase().trim();
  return all.find((s) => s.name.toLowerCase() === clean) ||
         all.find((s) => s.name.toLowerCase().includes(clean));
}

/**
 * Generate a high-density skill catalog for system prompt injection.
 * Informs the agent of all user skills and instructs it to call `use_skill` when needed.
 */
export function formatSkillsPrompt(workspaceDir: string = process.cwd()): string {
  const skills = loadAllSkills(workspaceDir);
  if (skills.length === 0) return '';

  const workspaceSkills = skills.filter((s) => s.category === 'workspace');
  const agentSkills = skills.filter((s) => s.category === '.agents');
  const orionSkills = skills.filter((s) => s.category === '.orion');

  let prompt = '\n\n=== USER SKILL SYSTEM & AGENT CAPABILITIES ===\n';
  prompt += 'You have access to the user\'s specialized `.agents` and `.orion` skills library.\n';
  prompt += 'When a task relates to any of these areas (e.g. animation, UI polish, Vercel guidelines, brutalism, shadcn),\n';
  prompt += 'you MUST check and invoke the `use_skill` tool to load its full procedural rules into context.\n\n';

  if (workspaceSkills.length > 0) {
    prompt += '[Workspace Skills (.orion/skills - Active Defaults)]:\n';
    for (const skill of workspaceSkills) {
      prompt += `- ${skill.name}: ${skill.description}\n`;
    }
    prompt += '\n';
  }

  if (agentSkills.length > 0) {
    prompt += `[User .agents Skills (~/.agents/skills - ${agentSkills.length} available)]:\n`;
    for (const skill of agentSkills) {
      prompt += `- ${skill.name}: ${skill.description}\n`;
    }
    prompt += '\n';
  }

  if (orionSkills.length > 0) {
    prompt += `[Synthesized & Orion Skills (~/.orion/skills)]:\n`;
    for (const skill of orionSkills.slice(0, 15)) {
      prompt += `- ${skill.name}: ${skill.description}\n`;
    }
  }

  return prompt;
}

/**
 * Save a newly synthesized markdown skill to .orion/skills/<slug>.md
 */
export function saveSynthesizedSkill(
  name: string,
  description: string,
  markdownBody: string,
  workspaceDir: string = process.cwd()
): { slug: string; fullPath: string } {
  const slug = name
    .toLowerCase()
    .replace(/[^a-z0-9_-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');

  const skillsDir = path.join(workspaceDir, '.orion', 'skills');
  if (!fs.existsSync(skillsDir)) {
    fs.mkdirSync(skillsDir, { recursive: true });
  }

  const fullPath = path.join(skillsDir, `${slug}.md`);
  const content = `---
name: ${slug}
description: "${description.replace(/"/g, '\\"')}"
synthesized_at: ${new Date().toISOString()}
---

${markdownBody}
`;

  fs.writeFileSync(fullPath, content, 'utf-8');
  return { slug, fullPath };
}
