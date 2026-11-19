import { forwardRef } from 'react';
import { Box, IconButton, TextField, Typography } from '@mui/material';
import SendIcon from '@mui/icons-material/Send';
import StopIcon from '@mui/icons-material/Stop';

/**
 * Controlled message input. Enter sends, Shift+Enter adds a line. While busy
 * the send button becomes a stop button.
 */
const ChatComposer = forwardRef(({
  value,
  onChange,
  onSend,
  onStop,
  busy = false,
  disabled = false,
  placeholder = 'Ask anything...',
  hint = 'Press Enter to send, Shift+Enter for new line',
  labels = {}
}, ref) => {
  const canSend = !busy && !disabled && value.trim().length > 0;
  const send = () => { if (canSend) onSend(value); };
  const handleKeyDown = (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent?.isComposing) {
      e.preventDefault();
      send();
    }
  };

  return (
    <Box sx={{ p: 2, borderTop: '1px solid var(--theme-border)', backgroundColor: 'var(--theme-bg-secondary)' }}>
      <Box sx={{ display: 'flex', alignItems: 'flex-end', gap: 1, maxWidth: 800, mx: 'auto' }}>
        <TextField
          inputRef={ref}
          fullWidth
          multiline
          maxRows={4}
          placeholder={placeholder}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={busy || disabled}
          slotProps={{ htmlInput: { 'aria-label': labels.input || 'Message' } }}
          sx={{
            '& .MuiOutlinedInput-root': {
              borderRadius: '16px',
              backgroundColor: 'var(--theme-bg-primary)',
              color: 'var(--theme-text-primary)',
              '& fieldset': { borderColor: 'var(--theme-border)' },
              '&:hover fieldset': { borderColor: 'var(--theme-text-secondary)' },
              '&.Mui-focused fieldset': { borderColor: 'var(--accent-primary, #65758E)' }
            },
            '& .MuiInputBase-input': { '&::placeholder': { color: 'var(--theme-text-tertiary)', opacity: 1 } }
          }}
        />
        {busy ? (
          <IconButton
            onClick={onStop}
            disabled={!onStop}
            aria-label={labels.stop || 'Stop'}
            sx={{ backgroundColor: '#ef4444', color: '#fff', '&:hover': { backgroundColor: '#dc2626' } }}
          >
            <StopIcon />
          </IconButton>
        ) : (
          <IconButton
            onClick={send}
            disabled={!canSend}
            aria-label={labels.send || 'Send'}
            sx={{
              backgroundColor: 'var(--accent-primary, #65758E)',
              color: '#fff',
              '&:hover': { opacity: 0.9, backgroundColor: 'var(--accent-primary, #65758E)' },
              '&.Mui-disabled': { backgroundColor: 'var(--theme-bg-secondary)', color: 'var(--theme-text-tertiary)' }
            }}
          >
            <SendIcon />
          </IconButton>
        )}
      </Box>
      {hint && (
        <Typography sx={{ textAlign: 'center', fontSize: '0.65rem', color: 'var(--theme-text-tertiary)', mt: 1 }}>
          {hint}
        </Typography>
      )}
    </Box>
  );
});
ChatComposer.displayName = 'ChatComposer';

export default ChatComposer;
