// Entry of the chat client that VoipAppz Builder's local server loads.
// It runs on the Builder server's origin and reads
// what it needs from the page's `voipappz-builder:info` meta tag.
import { createRoot } from 'react-dom/client';
import { CssBaseline, ThemeProvider } from '@mui/material';
import { createAppTheme } from './theme/theme';
import './index.css';
import BuilderApp from './BuilderApp';

const meta = document.querySelector('meta[name="voipappz-builder:info"]');
const root = createRoot(document.getElementById('root'));

if (!meta) {
  root.render(<p style={{ padding: '2rem' }}>Open this through VoipAppz Builder.</p>);
} else {
  const info = JSON.parse(meta.content);
  root.render(
    <ThemeProvider theme={createAppTheme('ltr')}>
      <CssBaseline />
      <BuilderApp info={info} />
    </ThemeProvider>
  );
}
