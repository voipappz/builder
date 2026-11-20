import { useCallback, useEffect, useRef, useState } from 'react';
import { connectSocket } from './phoenixSocket';
import { openAcp } from './acpConnection';
import { addUserMessage, applyUpdate, finishTurn } from './chatState';
import { reconnectDelay } from './projectView';

const AUTH_REQUIRED = -32000;
// Builder works in its own worktree, so file edits need no prompt there;
// commands and anything outside still ask.
const PREFERRED_MODE = 'acceptEdits';

function socketUrl(info) {
  const scheme = window.location.protocol === 'https:' ? 'wss' : 'ws';
  const query = new URLSearchParams({ secret: info.secret, vsn: '2.0.0' });
  return `${scheme}://${window.location.host}${info.socketPath}?${query}`;
}

function describeError(error) {
  if (error?.code === AUTH_REQUIRED || /auth/i.test(error?.message || '')) {
    return 'Claude Code is not signed in. Run `claude` in a terminal and sign in, then reload.';
  }
  const details = error?.data?.details || error?.data?.message;
  const message = error?.message || 'Unknown error';
  return details ? `${message}: ${String(details).slice(0, 400)}` : message;
}

/**
 * One chat with the agent over ACP: messages for ChatView, busy state, the
 * pending permission request, and actions. The server picks the agent and
 * its tools; this hook only speaks the protocol.
 */
export function useBuilderChat(info) {
  const [messages, setMessages] = useState([]);
  const [status, setStatus] = useState({ tone: 'neutral', label: 'Connecting…' });
  const [busy, setBusy] = useState(false);
  const [permission, setPermission] = useState(null);
  const [modes, setModes] = useState({ current: null, available: [] });
  const acpRef = useRef(null);
  const sessionRef = useRef(null);

  const startSession = useCallback(async () => {
    const acp = acpRef.current;
    const session = await acp.request('session/new', { cwd: '', mcpServers: [] });
    sessionRef.current = session.sessionId;
    const available = session.modes?.availableModes || [];
    let current = session.modes?.currentModeId || null;
    if (current !== PREFERRED_MODE && available.some((m) => m.id === PREFERRED_MODE)) {
      await acp.request('session/set_mode', { sessionId: session.sessionId, modeId: PREFERRED_MODE });
      current = PREFERRED_MODE;
    }
    setModes({ current, available });
    setMessages([]);
    setStatus({ tone: 'ok', label: 'Ready' });
  }, []);

  // One connection at a time. When it drops (Builder restarted the agent,
  // the server restarted), reconnect with a short backoff and start a new
  // session: the old one ended with the agent.
  useEffect(() => {
    let closed = false;
    let socket = null;
    let timer = null;
    let attempt = 0;

    const reconnect = () => {
      if (closed) return;
      acpRef.current = null;
      sessionRef.current = null;
      setBusy(false);
      setPermission(null);
      setStatus({ tone: 'neutral', label: 'Reconnecting…' });
      clearTimeout(timer);
      timer = setTimeout(connect, reconnectDelay(attempt++));
    };

    function connect() {
      if (closed) return;
      let dropped = false;
      const drop = () => { if (!dropped) { dropped = true; reconnect(); } };
      socket = connectSocket(socketUrl(info), { onClose: drop });
      (async () => {
        try {
          const acp = await openAcp(socket, {
            onNotification: (method, params) => {
              if (method === 'session/update' && params?.sessionId === sessionRef.current) {
                if (params.update?.sessionUpdate === 'current_mode_update') {
                  setModes((m) => ({ ...m, current: params.update.currentModeId }));
                  return;
                }
                setMessages((current) => applyUpdate(current, params.update));
              }
            },
            onRequest: (method, params) => {
              if (method !== 'session/request_permission') {
                return Promise.reject(new Error(`${method} is not supported by this client`));
              }
              return new Promise((resolve) => setPermission({ params, resolve }));
            },
            onExit: (payload) => {
              setBusy(false);
              setStatus({ tone: 'error', label: `The agent stopped: ${payload?.message || payload?.error || 'no reason given'}` });
            }
          });
          if (closed || dropped) return;
          acpRef.current = acp;
          setStatus({ tone: 'neutral', label: 'Starting a session…' });
          await startSession();
          attempt = 0;
        } catch (error) {
          if (closed) return;
          // A socket that never opened or closed early reconnects; an agent
          // error (not signed in, say) stays on screen.
          if (dropped || /reach the Builder server|Connection closed/.test(error?.message || '')) drop();
          else setStatus({ tone: 'error', label: describeError(error) });
        }
      })();
    }

    connect();
    return () => {
      closed = true;
      clearTimeout(timer);
      socket?.close();
    };
  }, [info, startSession]);

  const send = useCallback((text) => {
    const prompt = String(text || '').trim();
    const acp = acpRef.current;
    if (!prompt || !acp || !sessionRef.current) return;
    setMessages((current) => addUserMessage(current, prompt));
    setBusy(true);
    acp.request('session/prompt', { sessionId: sessionRef.current, prompt: [{ type: 'text', text: prompt }] })
      .then((result) => setMessages((current) => finishTurn(current, { stopReason: result?.stopReason })))
      .catch((error) => setMessages((current) => finishTurn(current, { error: describeError(error) })))
      .finally(() => setBusy(false));
  }, []);

  const answerPermission = useCallback((optionId) => {
    setPermission((current) => {
      current?.resolve(optionId
        ? { outcome: { outcome: 'selected', optionId } }
        : { outcome: { outcome: 'cancelled' } });
      return null;
    });
  }, []);

  const stop = useCallback(() => {
    if (!acpRef.current || !sessionRef.current) return;
    answerPermission(null);
    acpRef.current.notify('session/cancel', { sessionId: sessionRef.current });
  }, [answerPermission]);

  const newSession = useCallback(() => {
    if (!acpRef.current || busy) return;
    startSession().catch((error) => setStatus({ tone: 'error', label: describeError(error) }));
  }, [busy, startSession]);

  const setMode = useCallback((modeId) => {
    if (!acpRef.current || !sessionRef.current) return;
    acpRef.current.request('session/set_mode', { sessionId: sessionRef.current, modeId })
      .then(() => setModes((m) => ({ ...m, current: modeId })))
      .catch((error) => setStatus({ tone: 'error', label: describeError(error) }));
  }, []);

  return { messages, status, busy, permission, modes, setMode, send, stop, newSession, answerPermission };
}
