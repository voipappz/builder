import { memo } from 'react';
import { Box, Chip, Tooltip, Typography } from '@mui/material';
import PsychologyIcon from '@mui/icons-material/Psychology';

/** steps: [{ title }] */
const ReasoningSteps = memo(({ steps, label = 'Reasoning' }) => (
  <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
    <Tooltip title={label}>
      <PsychologyIcon sx={{ fontSize: 16, color: 'var(--accent-primary, #65758E)' }} />
    </Tooltip>
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
      <Typography sx={{ fontSize: '0.65rem', color: 'var(--theme-text-tertiary)', textTransform: 'uppercase', fontWeight: 600 }}>
        {label}
      </Typography>
      {steps.map((step, idx) => (
        <Box key={`${step.title}-${idx}`} sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Chip
            label={`STEP ${idx + 1}`}
            size="small"
            sx={{ backgroundColor: 'var(--theme-bg-secondary)', color: 'var(--theme-text-secondary)', fontSize: '0.65rem', height: 20 }}
          />
          <Typography sx={{ fontSize: '0.75rem', color: 'var(--theme-text-secondary)' }}>{step.title}</Typography>
        </Box>
      ))}
    </Box>
  </Box>
));
ReasoningSteps.displayName = 'ReasoningSteps';

export default ReasoningSteps;
