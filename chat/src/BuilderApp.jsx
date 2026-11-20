import { useState } from 'react';
import { Box, Button, IconButton, MenuItem, Select, Tab, Tabs, Tooltip, Typography } from '@mui/material';
import RefreshIcon from '@mui/icons-material/Refresh';
import OpenInNewIcon from '@mui/icons-material/OpenInNew';
import AddCommentIcon from '@mui/icons-material/AddComment';
import { ChatView } from './chat-ui';
import { useBuilderChat } from './useBuilderChat';
import BotPanel from './BotPanel';
import { projectView } from './projectView';
import logo from './logo.png';

const SUGGESTIONS = [
  { label: 'Find the call history screen', prompt: 'Find the component that renders the call history list and tell me its file and line.' },
  { label: 'Explain the extensions screen', prompt: 'Explain how the extensions screen loads and shows its data.' },
  { label: 'Add a search field', prompt: 'Add a search field above the extensions list that filters by name or number as I type.' }
];

const STATUS_COLOR = { ok: 'var(--color-success)', error: 'var(--color-danger)', neutral: 'var(--theme-text-tertiary)' };

function PermissionBar({ permission, onAnswer }) {
  const { toolCall, options = [] } = permission.params || {};
  return (
    <Box data-testid="builder-permission" sx={{ p: 1.5, borderBottom: '1px solid var(--theme-border)', backgroundColor: 'var(--theme-bg-secondary)' }}>
      <Typography sx={{ fontSize: '0.85rem', color: 'var(--theme-text-primary)', mb: 1 }}>
        The agent asks to run: <strong>{toolCall?.title || 'a tool'}</strong>
      </Typography>
      <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
        {options.map((option) => (
          <Button
            key={option.optionId}
            size="small"
            variant={option.kind?.startsWith('allow') ? 'contained' : 'outlined'}
            color={option.kind?.startsWith('reject') ? 'error' : 'primary'}
            onClick={() => onAnswer(option.optionId)}
          >
            {option.name}
          </Button>
        ))}
      </Box>
    </Box>
  );
}

// A project with a page (a product, or a native app such as the phone's
// emulator stream) shows that page first, then Status (status, restart,
// actions, log); a voice bot has a Bot tab to talk to it. The Admin and User
// previews show only when Builder runs the app locally (`previews` set).
function Previews({ previews, bot, secret, view }) {
  const productUrl = view?.pageUrl || null;
  const frames = { ...(productUrl ? { product: productUrl } : {}), ...(previews || {}) };
  const [role, setRole] = useState(productUrl ? 'product' : bot ? 'bot' : 'admin');
  const [reloadKey, setReloadKey] = useState(0);
  const url = frames[role];
  const isBot = role === 'bot';

  return (
    <Box sx={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0, borderLeft: '1px solid var(--theme-border)' }}>
      <Box sx={{ display: 'flex', alignItems: 'center', px: 1, borderBottom: '1px solid var(--theme-border)', backgroundColor: 'var(--theme-bg-secondary)' }}>
        <Tabs value={role} onChange={(_, value) => setRole(value)} sx={{ minHeight: 40, '& .MuiTab-root': { minHeight: 40 } }}>
          {productUrl && <Tab value="product" label={bot.name} />}
          {bot && <Tab value="bot" label={view.statusTab} />}
          {frames.admin && <Tab value="admin" label="Admin" />}
          {frames.user && <Tab value="user" label="User" />}
        </Tabs>
        <Typography sx={{ flex: 1, ml: 2, fontSize: '0.75rem', color: 'var(--theme-text-tertiary)', fontFamily: 'monospace' }} noWrap>
          {isBot ? bot.name : url}
        </Typography>
        {!isBot && (
          <>
            <Tooltip title="Reload preview">
              <IconButton size="small" onClick={() => setReloadKey((k) => k + 1)}><RefreshIcon fontSize="small" /></IconButton>
            </Tooltip>
            <Tooltip title="Open in a new tab">
              <IconButton size="small" component="a" href={url} target="_blank" rel="noreferrer"><OpenInNewIcon fontSize="small" /></IconButton>
            </Tooltip>
          </>
        )}
      </Box>
      {/* Stays mounted, so a call keeps going while you look at the previews. */}
      {bot && (
        <Box sx={{ flex: 1, display: isBot ? 'flex' : 'none', minHeight: 0 }}>
          <BotPanel bot={bot} secret={secret} view={view} />
        </Box>
      )}
      {/* All stay mounted so each keeps its own login while switching tabs. */}
      {Object.keys(frames).map((name) => (
        <Box
          key={`${name}-${reloadKey}`}
          component="iframe"
          title={`${name} preview`}
          src={frames[name]}
          sx={{ flex: 1, border: 0, display: role === name ? 'block' : 'none', backgroundColor: '#fff' }}
        />
      ))}
    </Box>
  );
}

export default function BuilderApp({ info }) {
  const chat = useBuilderChat(info);
  const view = projectView(info.bot, info.workspaceName);
  const [input, setInput] = useState('');

  const send = (text) => {
    chat.send(text ?? input);
    setInput('');
  };

  return (
    <Box sx={{ display: 'flex', height: '100vh', backgroundColor: 'var(--theme-bg-primary)' }}>
      <Box sx={{ width: '42%', minWidth: 380, display: 'flex', flexDirection: 'column' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, px: 2, py: 1, borderBottom: '1px solid var(--theme-border)' }}>
          <Box component="img" src={logo} alt="VoipAppz" sx={{ height: 22 }} />
          <Typography sx={{ fontWeight: 600, color: 'var(--theme-text-primary)' }}>Builder</Typography>
          <Typography sx={{ flex: 1, fontSize: '0.75rem', color: 'var(--theme-text-tertiary)', fontFamily: 'monospace' }} noWrap title={info.workspace}>
            {info.workspaceName}
          </Typography>
          {info.portalUrl && (
            <Tooltip title="Open the VoipAppz portal in your browser">
              <IconButton size="small" component="a" href={info.portalUrl} target="_blank" rel="noreferrer"><OpenInNewIcon fontSize="small" /></IconButton>
            </Tooltip>
          )}
          {chat.modes.available.length > 0 && (
            <Tooltip title={chat.modes.available.find((m) => m.id === chat.modes.current)?.description || 'How the agent asks before acting'}>
              <Select
                size="small"
                value={chat.modes.current || ''}
                onChange={(event) => chat.setMode(event.target.value)}
                inputProps={{ 'aria-label': 'Agent mode' }}
                sx={{ fontSize: '0.75rem', height: 28 }}
              >
                {chat.modes.available.map((mode) => (
                  <MenuItem key={mode.id} value={mode.id} sx={{ fontSize: '0.8rem' }}>{mode.name}</MenuItem>
                ))}
              </Select>
            </Tooltip>
          )}
          <Tooltip title="New conversation">
            <span>
              <IconButton size="small" onClick={chat.newSession} disabled={chat.busy}><AddCommentIcon fontSize="small" /></IconButton>
            </span>
          </Tooltip>
        </Box>
        {chat.permission && <PermissionBar permission={chat.permission} onAnswer={chat.answerPermission} />}
        <Box sx={{ flex: 1, minHeight: 0 }}>
          <ChatView
            title={info.agent}
            subtitle={
              <Box component="span" sx={{ color: STATUS_COLOR[chat.status.tone] }}>
                {chat.status.label}
                {info.apiMcp ? ` · API tools: ${info.apiUrl}` : ' · API tools off'}
              </Box>
            }
            messages={chat.messages}
            busy={chat.busy}
            inputValue={input}
            onInputChange={setInput}
            onSend={send}
            onStop={chat.stop}
            emptyState={{
              title: 'What should we build?',
              description: view ? view.description : `The agent works in ${info.workspaceName}. Changes show in the previews.`,
              suggestions: view ? view.suggestions : SUGGESTIONS,
              onSuggestion: send
            }}
          />
        </Box>
      </Box>
      <Previews previews={info.previews} bot={info.bot} secret={info.secret} view={view} />
    </Box>
  );
}
