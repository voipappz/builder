import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

// The package is shared with VoipAppz Builder, which must have no connection
// to the app's Assistant, VML or Bot code. Keep every such import out.
const here = dirname(fileURLToPath(import.meta.url));
const FORBIDDEN = [
  /from ['"]\.\.\/\.\.\/(?!node_modules)/,           // reaching into the app
  /useAIChat|aiMockService|AIChatSidebarContext/,
  /providersApi|ProviderDialog/,
  /\/api\/vmls|vmlsApi|useVMLChat/,
  /Bots?\//,
  /AuthContext|UserAuthContext|['"].*\/config['"]/,
  /\bfetch\(|axios|EventSource|WebSocket/
];

describe('chat-ui package boundary', () => {
  const files = readdirSync(here).filter(f => /\.(jsx?|css)$/.test(f) && !f.includes('.test.'));
  it.each(files)('%s has no app, Assistant, VML, Bot or network dependency', (file) => {
    const src = readFileSync(join(here, file), 'utf8');
    for (const rule of FORBIDDEN) expect(src).not.toMatch(rule);
  });
});
