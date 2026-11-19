import { memo } from 'react';
import { Box, Typography } from '@mui/material';
import ErrorOutlineIcon from '@mui/icons-material/ErrorOutline';
import ToolCallChip from './ToolCallChip';
import ReasoningSteps from './ReasoningSteps';
import LoadingDots from './LoadingDots';
import CopyButton from './CopyButton';
import { formatTime } from './format';

const AssistantBody = ({ message, labels }) => (
  <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
    {message.reasoning?.length > 0 && <ReasoningSteps steps={message.reasoning} label={labels.reasoning} />}
    {message.toolCalls?.length > 0 && (
      <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5, mb: 0.5 }}>
        {message.toolCalls.map((tc, idx) => <ToolCallChip key={tc.id || `${tc.name}-${idx}`} toolCall={tc} />)}
      </Box>
    )}
    {message.error ? (
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, color: '#ef4444' }}>
        <ErrorOutlineIcon sx={{ fontSize: 18 }} />
        <Typography sx={{ color: '#ef4444', fontSize: '0.9rem' }}>
          {message.content || labels.genericError || 'Something went wrong. Please try again.'}
        </Typography>
      </Box>
    ) : message.content ? (
      <Typography component="div" sx={{ whiteSpace: 'pre-wrap', fontSize: '0.9rem', lineHeight: 1.5 }}>
        {message.content}
      </Typography>
    ) : (
      <LoadingDots />
    )}
  </Box>
);

/**
 * One chat bubble with its timestamp and, for assistant text, a copy button.
 *
 * message: { id, role: 'user' | 'assistant', content, createdAt?, error?,
 *            toolCalls?: ToolCall[], reasoning?: [{ title }] }
 */
const ChatMessage = memo(({ message, labels = {} }) => {
  const isUser = message.role === 'user';
  return (
    <Box data-testid="chat-message" data-role={message.role} sx={{ display: 'flex', justifyContent: isUser ? 'flex-end' : 'flex-start' }}>
      <Box sx={{ maxWidth: '70%', minWidth: 60 }}>
        <Box
          sx={{
            px: 2,
            py: 1.5,
            borderRadius: isUser ? '16px 16px 4px 16px' : '16px 16px 16px 4px',
            backgroundColor: isUser ? 'var(--accent-primary, #65758E)' : 'var(--theme-bg-secondary)',
            color: isUser ? '#fff' : 'var(--theme-text-primary)',
            border: isUser ? 'none' : '1px solid var(--theme-border)',
            boxShadow: '0 1px 2px rgba(0,0,0,0.08)'
          }}
        >
          {isUser ? (
            <Typography sx={{ whiteSpace: 'pre-wrap', fontSize: '0.9rem', lineHeight: 1.5 }}>{message.content}</Typography>
          ) : (
            <AssistantBody message={message} labels={labels} />
          )}
        </Box>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: isUser ? 'flex-end' : 'space-between', mt: 0.5, px: 0.5, gap: 1 }}>
          <Typography sx={{ fontSize: '0.65rem', color: 'var(--theme-text-tertiary)' }}>
            {formatTime(message.createdAt)}
          </Typography>
          {!isUser && message.content && !message.error && <CopyButton text={message.content} labels={labels} />}
        </Box>
      </Box>
    </Box>
  );
});
ChatMessage.displayName = 'ChatMessage';

export default ChatMessage;
