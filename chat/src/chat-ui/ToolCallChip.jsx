import { memo } from 'react';
import { Box, Chip, CircularProgress, Tooltip } from '@mui/material';
import BuildIcon from '@mui/icons-material/Build';
import ErrorOutlineIcon from '@mui/icons-material/ErrorOutline';

/**
 * One chip per tool call: a spinner while it runs, a tool icon when done,
 * red when it failed. Arguments and result sit in the tooltip.
 *
 * toolCall: { id, name, args?, result?, status: 'running' | 'done' | 'error' }
 */
const ToolCallChip = memo(({ toolCall }) => {
  const { name, args, result } = toolCall;
  const status = toolCall.status || 'running';
  const failed = status === 'error';
  const argText = args && Object.keys(args).length > 0 ? JSON.stringify(args) : '';
  const detail = [
    argText && `args: ${argText}`,
    result !== undefined && result !== null && String(result).slice(0, 800)
  ].filter(Boolean).join('\n\n');
  const icon = status === 'running'
    ? <CircularProgress size={12} sx={{ color: 'var(--theme-text-secondary)' }} />
    : (failed ? <ErrorOutlineIcon sx={{ fontSize: 14 }} /> : <BuildIcon sx={{ fontSize: 14 }} />);

  return (
    <Tooltip
      placement="top"
      arrow
      title={detail
        ? <Box component="pre" sx={{ m: 0, whiteSpace: 'pre-wrap', fontFamily: 'monospace', fontSize: '0.7rem' }}>{detail}</Box>
        : ''}
    >
      <Chip
        icon={icon}
        label={name}
        size="small"
        data-testid="chat-tool-call"
        data-status={status}
        sx={{
          backgroundColor: 'var(--theme-bg-secondary)',
          color: failed ? '#ef4444' : 'var(--theme-text-secondary)',
          fontSize: '0.7rem',
          fontFamily: 'monospace',
          textTransform: 'uppercase',
          '& .MuiChip-icon': { color: failed ? '#ef4444' : 'var(--theme-text-secondary)', ml: '6px' }
        }}
      />
    </Tooltip>
  );
});
ToolCallChip.displayName = 'ToolCallChip';

export default ToolCallChip;
