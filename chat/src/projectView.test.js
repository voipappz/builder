import { describe, it, expect } from 'vitest';
import { projectView, reconnectDelay } from './projectView';

describe('projectView', () => {
  it('shows a native app (the phone) by its page, with no talk button', () => {
    const view = projectView({ kind: 'app', name: 'VoipAppz Phone', pageUrl: 'http://127.0.0.1:8000/#!x' }, 'phone');
    expect(view).toMatchObject({ canTalk: false, pageUrl: 'http://127.0.0.1:8000/#!x', statusTab: 'Status', logLabel: 'App log' });
    expect(view.description).not.toMatch(/bot/i);
    expect(view.suggestions.map((s) => s.label)).toContain('Explain this app');
  });

  it('a native app without a page has only Status', () => {
    const view = projectView({ kind: 'app', name: 'Phone' }, 'phone');
    expect(view).toMatchObject({ pageUrl: null, statusTab: 'Status', canTalk: false });
  });

  it('a product shows its page and Status', () => {
    expect(projectView({ kind: 'product', name: 'Web', pageUrl: 'http://127.0.0.1:4200/' }, 'web'))
      .toMatchObject({ canTalk: false, statusTab: 'Status', logLabel: 'Product log' });
  });

  it('only a voice bot can be talked to', () => {
    const view = projectView({ kind: 'bot', name: 'Noa', pageUrl: 'http://x/' }, 'noa');
    expect(view).toMatchObject({ canTalk: true, pageUrl: null, statusTab: 'Bot', logLabel: 'Bot log' });
  });

  it('no project, no view', () => {
    expect(projectView(null, 'x')).toBeNull();
  });
});

describe('reconnectDelay', () => {
  it('backs off 1, 2, 4 seconds, then every 5', () => {
    expect([0, 1, 2, 3, 9].map(reconnectDelay)).toEqual([1000, 2000, 4000, 5000, 5000]);
  });
});
