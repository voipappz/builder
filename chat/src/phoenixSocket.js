// Minimal Phoenix v2 websocket client for Builder's local server.
// Frames are JSON arrays: [joinRef, ref, topic, event, payload].

const HEARTBEAT_MS = 30000;

export function connectSocket(url, { onClose } = {}) {
  const ws = new WebSocket(url);
  const channels = new Map();
  const replies = new Map();
  let ref = 0;
  let heartbeat;

  const send = (frame) => ws.send(JSON.stringify(frame));
  const nextRef = () => String(++ref);

  const opened = new Promise((resolve, reject) => {
    ws.addEventListener('open', () => {
      heartbeat = setInterval(() => send([null, nextRef(), 'phoenix', 'heartbeat', {}]), HEARTBEAT_MS);
      resolve();
    });
    ws.addEventListener('error', () => reject(new Error('Cannot reach the Builder server')));
  });

  ws.addEventListener('close', () => {
    clearInterval(heartbeat);
    for (const { reject } of replies.values()) reject(new Error('Connection closed'));
    replies.clear();
    onClose?.();
  });

  ws.addEventListener('message', (event) => {
    const [, msgRef, topic, name, payload] = JSON.parse(event.data);
    if (name === 'phx_reply' && replies.has(msgRef)) {
      const { resolve, reject } = replies.get(msgRef);
      replies.delete(msgRef);
      if (payload.status === 'ok') resolve(payload.response);
      else reject(new Error(payload.response?.reason || 'Request refused'));
      return;
    }
    channels.get(topic)?.dispatch(name, payload);
  });

  const join = async (topic, payload = {}) => {
    await opened;
    const joinRef = nextRef();
    const listeners = new Map();
    const channel = {
      on(name, callback) {
        listeners.set(name, [...(listeners.get(name) || []), callback]);
      },
      push(name, body) {
        send([joinRef, nextRef(), topic, name, body]);
      },
      dispatch(name, body) {
        (listeners.get(name) || []).forEach((callback) => callback(body));
      },
      leave() {
        send([joinRef, nextRef(), topic, 'phx_leave', {}]);
        channels.delete(topic);
      }
    };
    channels.set(topic, channel);
    await new Promise((resolve, reject) => {
      replies.set(joinRef, { resolve, reject });
      send([joinRef, joinRef, topic, 'phx_join', payload]);
    });
    return channel;
  };

  return { join, close: () => ws.close() };
}
