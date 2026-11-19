import { memo, useCallback, useEffect, useRef, useState } from 'react';
import { IconButton, Tooltip } from '@mui/material';
import ContentCopyIcon from '@mui/icons-material/ContentCopy';
import CheckIcon from '@mui/icons-material/Check';

const CopyButton = memo(({ text, labels = {} }) => {
  const [copied, setCopied] = useState(false);
  const timer = useRef(null);
  useEffect(() => () => clearTimeout(timer.current), []);
  const handleCopy = useCallback(() => {
    if (!navigator.clipboard) return;
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      clearTimeout(timer.current);
      timer.current = setTimeout(() => setCopied(false), 1500);
    }).catch(() => {});
  }, [text]);
  const label = copied ? (labels.copied || 'Copied!') : (labels.copy || 'Copy');
  return (
    <Tooltip title={label} placement="top">
      <IconButton size="small" aria-label={label} onClick={handleCopy} sx={{ opacity: 0.5, '&:hover': { opacity: 1 }, p: 0.25 }}>
        {copied
          ? <CheckIcon sx={{ fontSize: 14, color: '#22c55e' }} />
          : <ContentCopyIcon sx={{ fontSize: 14, color: 'var(--theme-text-secondary)' }} />}
      </IconButton>
    </Tooltip>
  );
});
CopyButton.displayName = 'CopyButton';

export default CopyButton;
