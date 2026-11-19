import { Box } from '@mui/material';

const LoadingDots = () => (
  <Box className="va-chat-loading" data-testid="chat-loading">
    <Box className="va-chat-loading-dots">
      <Box className="va-chat-loading-dot" />
      <Box className="va-chat-loading-dot" />
      <Box className="va-chat-loading-dot" />
    </Box>
  </Box>
);

export default LoadingDots;
