// ACP (Agent Client Protocol) as raw JSON-RPC over the Builder server's
// `acp:` channel. The server starts the agent and pins its working directory
// and MCP servers, so this client never names a command, path or credential.

export const ACP_PROTOCOL_VERSION = 1;

export async function openAcp(socket, { onNotification, onRequest, onExit }) {
  const channel = await socket.join(`acp:${crypto.randomUUID()}`, {});
  const pending = new Map();
  let nextId = 1;

  channel.on('jsonrpc', async (message) => {
    if (message.id !== undefined && message.method === undefined) {
      const waiter = pending.get(message.id);
      if (!waiter) return;
      pending.delete(message.id);
      if (message.error) waiter.reject(Object.assign(new Error(message.error.message), { data: message.error.data }));
      else waiter.resolve(message.result);
      return;
    }
    if (message.id !== undefined) {
      try {
        const result = await onRequest(message.method, message.params);
        channel.push('jsonrpc', { jsonrpc: '2.0', id: message.id, result });
      } catch (error) {
        channel.push('jsonrpc', { jsonrpc: '2.0', id: message.id, error: { code: -32603, message: error.message } });
      }
      return;
    }
    onNotification(message.method, message.params);
  });

  channel.on('agent_exit', (payload) => {
    for (const { reject } of pending.values()) reject(new Error('The agent stopped'));
    pending.clear();
    onExit?.(payload);
  });

  const request = (method, params) => new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    channel.push('jsonrpc', { jsonrpc: '2.0', id, method, params });
  });

  const notify = (method, params) => channel.push('jsonrpc', { jsonrpc: '2.0', method, params });

  const init = await request('initialize', {
    protocolVersion: ACP_PROTOCOL_VERSION,
    clientCapabilities: { fs: { readTextFile: false, writeTextFile: false }, terminal: false },
    clientInfo: { name: 'voipappz-builder-client', version: '0.1.0' }
  });

  return { request, notify, init, close: () => channel.leave() };
}
