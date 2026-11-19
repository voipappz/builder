// Sample props for tests and visual checks. Not exported from the package.
export const sessions = [
  { id: 's1', title: 'Extension search field' },
  { id: 's2', title: 'Mobile call history' }
];

export const messages = [
  { id: 'm1', role: 'user', content: 'Add a search field to the extensions list', createdAt: 1760700000 },
  {
    id: 'm2',
    role: 'assistant',
    content: 'I added a search field above the table.',
    createdAt: 1760700005,
    reasoning: [{ title: 'Find the extensions screen' }],
    toolCalls: [
      { id: 't1', name: 'read_file', args: { path: 'src/components/Extensions.jsx' }, result: '…', status: 'done' },
      { id: 't2', name: 'apply_patch', args: {}, status: 'running' },
      { id: 't3', name: 'run_lint', status: 'error', result: 'exit 1' }
    ]
  }
];

export const streamingMessage = { id: 'm3', role: 'assistant', content: '', toolCalls: [] };
export const errorMessage = { id: 'm4', role: 'assistant', content: 'The run failed.', error: true };
