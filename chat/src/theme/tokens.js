/**
 * Design tokens — the ONE place a colour, radius, spacing step or type size
 * is defined in JS. Everything else (the MUI theme, sx, charts) reads these.
 *
 * Values are copied from the admin's src/index.css so the switch changes nothing on
 * screen; the CSS variables stay for plain-CSS consumers, and the MUI theme
 * (./theme.js) turns these into `--mui-palette-*` variables that
 * flip with the app's own `[data-theme="dark"]` attribute.
 *
 * Do not add a hex literal to a component: add a token here, or use the
 * palette (`theme.palette.severity.error`, `'text.secondary'`, ...).
 */

export const light = {
  background: '#ffffff',       // --theme-bg-primary
  surface: '#ffffff',          // --theme-surface
  surfaceMuted: '#f5f5f5',     // --theme-bg-secondary
  surfaceSunken: '#e8e8e8',    // --theme-bg-tertiary
  text: '#1a202c',             // --theme-text-primary
  textMuted: '#4a5568',        // --theme-text-secondary
  textFaint: '#9ca3af',        // --text-tertiary
  border: '#e2e8f0',           // --theme-border
  hover: 'rgba(0, 0, 0, 0.04)',
  primary: { main: '#5c6bc0', dark: '#3f4fb5', light: '#7986cb', contrastText: '#ffffff' },
  secondary: { main: '#EEA740', dark: '#E49728', light: '#f4c27a', contrastText: '#ffffff' },
  success: { main: '#10b981', dark: '#059669', light: '#34d399', contrastText: '#ffffff' },
  warning: { main: '#f59e0b', dark: '#d97706', light: '#fbbf24', contrastText: '#111827' },
  error: { main: '#ef4444', dark: '#dc2626', light: '#f87171', contrastText: '#ffffff' },
  info: { main: '#3b82f6', dark: '#2563eb', light: '#60a5fa', contrastText: '#ffffff' },
};

export const dark = {
  background: '#202124',
  surface: '#292a2d',
  surfaceMuted: '#292a2d',
  surfaceSunken: '#3c4043',
  text: '#e8eaed',
  textMuted: '#9aa0a6',
  textFaint: '#5f6368',
  border: '#3c4043',
  hover: 'rgba(255, 255, 255, 0.08)',
  primary: { main: '#7986cb', dark: '#5c6bc0', light: '#9fa8da', contrastText: '#ffffff' },
  secondary: { main: '#EEA740', dark: '#E49728', light: '#f4c27a', contrastText: '#111827' },
  success: { main: '#34d399', dark: '#10b981', light: '#6ee7b7', contrastText: '#052e16' },
  warning: { main: '#fbbf24', dark: '#f59e0b', light: '#fcd34d', contrastText: '#111827' },
  error: { main: '#f87171', dark: '#ef4444', light: '#fca5a5', contrastText: '#111827' },
  info: { main: '#60a5fa', dark: '#3b82f6', light: '#93c5fd', contrastText: '#111827' },
};

/**
 * Log severity: text colour + tinted background per level, light and dark.
 * The keys are the canonical event-store levels plus the syslog spellings
 * (LEVEL_CONFIG in utils/logFormatting.js reads these).
 */
export const severity = {
  light: {
    crit:  { color: '#dc2626', bg: '#fef2f2' },
    error: { color: '#ea580c', bg: '#fff7ed' },
    warn:  { color: '#ca8a04', bg: '#fefce8' },
    info:  { color: '#0891b2', bg: '#ecfeff' },
    debug: { color: '#16a34a', bg: '#f0fdf4' },
    trace: { color: '#7c3aed', bg: '#faf5ff' },
    notice: { color: '#2563eb', bg: '#eff6ff' },
    emerg: { color: '#991b1b', bg: '#fef2f2' },
    alert: { color: '#b91c1c', bg: '#fef2f2' },
  },
  dark: {
    crit:  { color: '#fca5a5', bg: 'rgba(239, 68, 68, 0.16)' },
    error: { color: '#fdba74', bg: 'rgba(234, 88, 12, 0.16)' },
    warn:  { color: '#fde047', bg: 'rgba(202, 138, 4, 0.16)' },
    info:  { color: '#67e8f9', bg: 'rgba(8, 145, 178, 0.16)' },
    debug: { color: '#86efac', bg: 'rgba(22, 163, 74, 0.16)' },
    trace: { color: '#c4b5fd', bg: 'rgba(124, 58, 237, 0.16)' },
    notice: { color: '#93c5fd', bg: 'rgba(37, 99, 235, 0.16)' },
    emerg: { color: '#fca5a5', bg: 'rgba(153, 27, 27, 0.2)' },
    alert: { color: '#fca5a5', bg: 'rgba(185, 28, 28, 0.2)' },
  },
};

/** Radius scale (px). `md` is the default `shape.borderRadius`. `card` is
    cards, dialogs and KPI tiles; `control` is buttons and inputs. */
export const radius = { sm: 4, md: 6, lg: 8, xl: 12, pill: 24, card: 14, control: 10 };

/** Shadows: `card` lifts a resting surface just off the page, `raised` is
    hover on a clickable card, `overlay` is dialogs and drawers. */
export const shadow = {
  card: '0 1px 2px rgba(20, 24, 32, 0.05), 0 1px 4px rgba(20, 24, 32, 0.05)',
  raised: '0 10px 24px rgba(15, 23, 42, 0.08)',
  overlay: '0 18px 48px rgba(20, 24, 32, 0.18)',
};

/** Type scale. Sizes in px; the theme converts to rem. */
export const type = {
  family: "'Rubik', -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Helvetica Neue', sans-serif",
  mono: "'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
  size: { xs: 11, sm: 12, md: 13, base: 14, lg: 16, xl: 20, xxl: 24 },
  weight: { regular: 400, medium: 500, semibold: 600, bold: 700 },
};

/** Screen widths in px. Spelled out (they are MUI's defaults) because the shell
    CSS hardcodes `max-width: 599px` for the shell, which must stay just under `sm`. */
export const breakpoints = { xs: 0, sm: 600, md: 900, lg: 1200, xl: 1536 };

/** The shell's "mobile" breakpoint (sidebar drawer, top-bar offset): below 600px,
    so laptops keep the sidebar rail. Screens keep their own `md` checks. */
export const MOBILE_BREAKPOINT = 'sm';
