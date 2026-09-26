import 'dotenv/config';
import React from 'react';
import { render } from 'ink';
import { Command } from 'commander';
import { Repl } from './components/Repl.js';
import { initGlobalConfig, detectDefaultModel } from './config.js';

// Load global configuration and API keys from ~/.orion/.env
initGlobalConfig();

const program = new Command();

const defaultModel = detectDefaultModel();

program
  .name('orion')
  .description('OrionBot CLI - High-performance agentic coding assistant with TypeScript React Ink UI and Rust core')
  .version('2.0.0')
  .argument('[prompt]', 'Optional one-shot instruction prompt')
  .option('-m, --model <model>', 'Override the default LLM model', defaultModel)
  .option('-r, --resume <session_id>', 'Resume a saved session by ID')
  .action((prompt, options) => {
    // Enter alternate screen buffer & set terminal background to deep pitch-black #000000 (OpenCode style)
    // Enable SGR mouse reporting mode (\x1b[?1000h\x1b[?1006h) so mouse wheel scrolling functions smoothly
    process.stdout.write(
      '\x1b[?1049h' +
      '\x1b[?1000h\x1b[?1006h' +
      '\x1b]11;#000000\x07\x1b]11;#000000\x1b\\' +
      '\x1b]10;#E4E4E7\x07\x1b]10;#E4E4E7\x1b\\' +
      '\x1b[48;2;0;0;0m\x1b[38;2;228;228;231m' +
      '\x1b[2J\x1b[3J\x1b[H'
    );

    const cleanup = () => {
      // Restore terminal background & foreground to user default, reset attributes, disable mouse reporting, exit alternate buffer
      process.stdout.write('\x1b[?1006l\x1b[?1000l\x1b]111\x07\x1b]111\x1b\\\x1b]110\x07\x1b]110\x1b\\\x1b[0m\x1b[?1049l\x1b[?25h');
      process.exit(0);
    };

    process.on('SIGINT', cleanup);
    process.on('SIGTERM', cleanup);
    process.on('exit', () => {
      process.stdout.write('\x1b[?1006l\x1b[?1000l\x1b]111\x07\x1b]111\x1b\\\x1b]110\x07\x1b]110\x1b\\\x1b[0m\x1b[?1049l\x1b[?25h');
    });
    process.on('uncaughtException', (err) => {
      process.stdout.write('\x1b[?1006l\x1b[?1000l\x1b]111\x07\x1b]111\x1b\\\x1b]110\x07\x1b]110\x1b\\\x1b[0m\x1b[?1049l\x1b[?25h\n');
      console.error('Fatal Orion CLI Error:', err);
      process.exit(1);
    });
    process.on('unhandledRejection', (reason) => {
      process.stdout.write('\x1b[?1006l\x1b[?1000l\x1b]111\x07\x1b]111\x1b\\\x1b]110\x07\x1b]110\x1b\\\x1b[0m\x1b[?1049l\x1b[?25h\n');
      console.error('Unhandled Orion CLI Rejection:', reason);
      process.exit(1);
    });

    const app = render(
      <Repl
        initialPrompt={prompt}
        initialModel={options.model}
      />,
      {
        exitOnCtrlC: true,
      }
    );

    app.waitUntilExit().then(cleanup);
  });

program.parse(process.argv);

