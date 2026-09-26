import React, { useState, useEffect } from 'react';
import { Box, Text, useInput } from 'ink';
import { hasApiKeyForProvider } from '../config.js';
import { isTerminalBackspace } from '../stdinTracker.js';

export interface ProviderDef {
  id: string;
  name: string;
  description: string;
  models: { id: string; name: string; description: string }[];
}

export const PROVIDERS: ProviderDef[] = [
  {
    id: 'anthropic',
    name: 'Anthropic',
    description: 'Claude 3.7 & 3.5 series',
    models: [
      { id: 'anthropic:claude-3-7-sonnet', name: 'claude-3-7-sonnet', description: 'Frontier hybrid reasoning & coding' },
      { id: 'anthropic:claude-3-5-sonnet', name: 'claude-3-5-sonnet', description: 'Everyday high-performance production' },
      { id: 'anthropic:claude-3-5-haiku', name: 'claude-3-5-haiku', description: 'Fastest & most cost-effective' },
      { id: 'anthropic:claude-3-opus', name: 'claude-3-opus', description: 'Deep analysis & writing flagship' },
    ],
  },
  {
    id: 'openai',
    name: 'OpenAI',
    description: 'GPT-4o & reasoning series',
    models: [
      { id: 'openai:gpt-4o', name: 'gpt-4o', description: 'Omni flagship multimodal model' },
      { id: 'openai:gpt-4o-mini', name: 'gpt-4o-mini', description: 'Ultra-fast, lightweight everyday model' },
      { id: 'openai:o3-mini', name: 'o3-mini', description: 'Advanced STEM & coding reasoning' },
      { id: 'openai:o1', name: 'o1', description: 'Deep multi-step reasoning flagship' },
    ],
  },
  {
    id: 'mistral',
    name: 'Mistral AI',
    description: 'Codestral & Mistral 3.5 series',
    models: [
      { id: 'mistral:mistral-medium-3.5', name: 'mistral-medium-3.5', description: 'Frontier 128B dense multimodal reasoning & coding (Recommended)' },
      { id: 'mistral:mistral-medium-latest', name: 'mistral-medium-latest', description: 'Mistral Medium latest flagship alias' },
      { id: 'mistral:codestral-latest', name: 'codestral-latest', description: 'Frontier code generation & reasoning' },
      { id: 'mistral:mistral-small-latest', name: 'mistral-small-latest', description: 'High-speed efficient generalist' },
      { id: 'mistral:mistral-large-latest', name: 'mistral-large-latest', description: 'Flagship multilingual reasoning (Requires Tier 2+)' },
      { id: 'mistral:ministral-8b-latest', name: 'ministral-8b-latest', description: 'Fast powerful edge model' },
      { id: 'mistral:open-mistral-7b', name: 'open-mistral-7b', description: 'Open-weights efficient model' },
    ],
  },
  {
    id: 'gemini',
    name: 'Google Gemini',
    description: 'Gemini 2.5 & 1.5 multimodal',
    models: [
      { id: 'gemini:gemini-2.5-pro', name: 'gemini-2.5-pro', description: 'Top-tier code & multi-token thinking' },
      { id: 'gemini:gemini-2.5-flash', name: 'gemini-2.5-flash', description: 'Next-gen high-speed multimodal' },
      { id: 'gemini:gemini-1.5-pro', name: 'gemini-1.5-pro', description: '2M ultra-long context window' },
    ],
  },
  {
    id: 'groq',
    name: 'Groq',
    description: 'Ultra-fast LPU inference',
    models: [
      { id: 'groq:llama-3.3-70b-versatile', name: 'llama-3.3-70b', description: '70B model with blazing inference speed' },
      { id: 'groq:llama-3.1-8b-instant', name: 'llama-3.1-8b', description: 'Instant latency lightweight model' },
      { id: 'groq:deepseek-r1-distill-llama-70b', name: 'deepseek-r1-distill', description: 'DeepSeek R1 reasoning on Groq LPU' },
    ],
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    description: 'Unified multi-provider router',
    models: [
      { id: 'openrouter:anthropic/claude-3.5-sonnet', name: 'claude-3.5-sonnet', description: 'Claude Sonnet via OpenRouter' },
      { id: 'openrouter:openai/gpt-4o', name: 'gpt-4o', description: 'GPT-4o via OpenRouter' },
      { id: 'openrouter:deepseek/deepseek-r1', name: 'deepseek-r1', description: 'DeepSeek R1 full reasoning model' },
    ],
  },
  {
    id: 'ollama',
    name: 'Ollama (Local)',
    description: 'Self-hosted local models',
    models: [
      { id: 'ollama:llama3.3', name: 'llama3.3', description: 'Meta Llama 3.3 local instance' },
      { id: 'ollama:qwen2.5-coder', name: 'qwen2.5-coder', description: 'Alibaba Qwen 2.5 Coder' },
      { id: 'ollama:deepseek-r1', name: 'deepseek-r1', description: 'DeepSeek R1 local instance' },
    ],
  },
];

interface ModelSelectorProps {
  currentModel: string;
  onSelect: (modelId: string) => void;
  onCancel: () => void;
  onConfigureKey?: (providerId: string) => void;
}

export const ModelSelector: React.FC<ModelSelectorProps> = ({
  currentModel,
  onSelect,
  onCancel,
  onConfigureKey,
}) => {
  const [selectedProviderIdx, setSelectedProviderIdx] = useState(0);
  const [step, setStep] = useState<'provider' | 'model'>('provider');
  const [selectedModelIdx, setSelectedModelIdx] = useState(0);
  const [isReady, setIsReady] = useState(false);

  // Mount debounce: ignore key events for 150ms to stop key bleeding from CommandPicker
  useEffect(() => {
    const timer = setTimeout(() => setIsReady(true), 150);
    return () => clearTimeout(timer);
  }, []);

  const activeProvider = PROVIDERS[selectedProviderIdx];
  const activeModels = activeProvider ? activeProvider.models : [];

  useInput((input, key) => {
    if (key.ctrl && input.toLowerCase() === 'c') {
      process.stdout.write('\x1b[?1049l\x1b[?25h');
      process.exit(0);
    }

    if (!isReady) return;

    if (key.escape) {
      if (step === 'model') {
        setStep('provider');
      } else {
        onCancel();
      }
      return;
    }

    if (step === 'provider') {
      if (input.toLowerCase() === 'k' && onConfigureKey && activeProvider.id !== 'ollama') {
        onConfigureKey(activeProvider.id);
        return;
      }

      if (key.upArrow) {
        setSelectedProviderIdx((prev) => (prev > 0 ? prev - 1 : PROVIDERS.length - 1));
      } else if (key.downArrow) {
        setSelectedProviderIdx((prev) => (prev < PROVIDERS.length - 1 ? prev + 1 : 0));
      } else if (key.return) {
        setStep('model');
        setSelectedModelIdx(0);
      }
    } else {
      if (key.upArrow) {
        setSelectedModelIdx((prev) => (prev > 0 ? prev - 1 : activeModels.length - 1));
      } else if (key.downArrow) {
        setSelectedModelIdx((prev) => (prev < activeModels.length - 1 ? prev + 1 : 0));
      } else if (key.return) {
        const chosen = activeModels[selectedModelIdx];
        if (chosen) {
          onSelect(chosen.id);
          // If no key is set, also prompt to configure key
          if (!hasApiKeyForProvider(activeProvider.id) && onConfigureKey && activeProvider.id !== 'ollama') {
            onConfigureKey(activeProvider.id);
          }
        }
      } else if (isTerminalBackspace(key, input) || key.leftArrow) {
        setStep('provider');
      }
    }
  });

  return (
    <Box flexDirection="column" borderStyle="round" borderColor="#3F3F46" paddingX={1} marginY={0}>
      <Box justifyContent="space-between">
        <Text bold color="#FFFFFF">
          {step === 'provider' ? 'Select AI Provider' : `Select Model · ${activeProvider.name}`}
        </Text>
        <Text color="#71717A">
          {step === 'provider' ? '↑↓ navigate • ↵ select • k set key • Esc cancel' : '↑↓ navigate • ↵ select • Esc/← back'}
        </Text>
      </Box>

      {step === 'provider' ? (
        PROVIDERS.map((prov, idx) => {
          const isSelected = idx === selectedProviderIdx;
          const hasKey = hasApiKeyForProvider(prov.id);
          return (
            <Box key={prov.id} flexDirection="row" justifyContent="space-between">
              <Box flexDirection="row">
                <Text color={isSelected ? '#FFFFFF' : '#3F3F46'}>
                  {isSelected ? '❯ ' : '  '}
                </Text>
                <Text bold={isSelected} color={isSelected ? '#FFFFFF' : '#D4D4D8'}>
                  {prov.name.padEnd(16)}
                </Text>
                <Text color={isSelected ? '#E4E4E7' : '#71717A'}>
                  {prov.description}
                </Text>
              </Box>
              {prov.id !== 'ollama' && (
                <Text color={hasKey ? '#10B981' : '#F59E0B'}>
                  {hasKey ? '✓ configured' : '● key needed'}
                </Text>
              )}
            </Box>
          );
        })
      ) : (
        activeModels.map((m, idx) => {
          const isSelected = idx === selectedModelIdx;
          const isCurrent = m.id === currentModel;
          return (
            <Box key={m.id} flexDirection="row">
              <Text color={isSelected ? '#FFFFFF' : '#3F3F46'}>
                {isSelected ? '❯ ' : '  '}
              </Text>
              <Text bold={isSelected} color={isSelected ? '#FFFFFF' : '#D4D4D8'}>
                {m.name.padEnd(26)}
              </Text>
              <Text color={isSelected ? '#E4E4E7' : '#71717A'}>
                {m.description}
              </Text>
              {isCurrent && (
                <Text color="#A1A1AA"> (active)</Text>
              )}
            </Box>
          );
        })
      )}
    </Box>
  );
};
