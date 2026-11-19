import { Box, Chip, Typography } from '@mui/material';
import SmartToyIcon from '@mui/icons-material/SmartToy';

/**
 * Shown before the first message. `notice` is any node the host wants above
 * the suggestions, for example a setup warning with its own action.
 *
 * suggestions: [{ label, prompt }]
 */
const ChatEmptyState = ({ icon, title, description, notice, suggestions = [], onSuggestion, hint }) => (
  <Box
    data-testid="chat-empty"
    sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', textAlign: 'center' }}
  >
    {icon || <SmartToyIcon sx={{ fontSize: 52, color: 'var(--accent-primary, #65758E)', mb: 2, opacity: 0.7 }} />}
    {title && (
      <Typography sx={{ color: 'var(--theme-text-primary)', fontSize: '1.3rem', fontWeight: 600, mb: 0.5 }}>{title}</Typography>
    )}
    {description && (
      <Typography sx={{ color: 'var(--theme-text-tertiary)', fontSize: '0.9rem', mb: 3 }}>{description}</Typography>
    )}
    {notice}
    {suggestions.length > 0 && (
      <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1, justifyContent: 'center', maxWidth: 560 }}>
        {suggestions.map((item) => (
          <Chip
            key={item.label}
            label={item.label}
            onClick={onSuggestion ? () => onSuggestion(item.prompt) : undefined}
            sx={{
              backgroundColor: 'var(--theme-bg-secondary)',
              color: 'var(--theme-text-secondary)',
              cursor: 'pointer',
              fontSize: '0.85rem',
              py: 0.5,
              border: '1px solid var(--theme-border)',
              '&:hover': { backgroundColor: 'var(--accent-primary, #65758E)', color: '#fff', borderColor: 'var(--accent-primary, #65758E)' }
            }}
          />
        ))}
      </Box>
    )}
    {hint && (
      <Typography sx={{ fontSize: '0.72rem', color: 'var(--theme-text-tertiary)', mt: 2 }}>{hint}</Typography>
    )}
  </Box>
);

export default ChatEmptyState;
