/**
 * The MUI theme. ONE theme, two colour schemes, driven by ./tokens.js.
 *
 * `cssVariables` + `colorSchemeSelector: 'data-theme'` means MUI writes
 * `--mui-palette-*` under `:root` (light) and `[data-theme="dark"]` (dark),
 * and every component reads `var(--mui-palette-…)`. The app already sets
 * that attribute from ThemeContext, so MUI, the app's own CSS variables and
 * the toggle all agree — no `palette.mode` state to keep in sync, and no
 * `[data-theme="dark"] .Mui*` overrides in CSS (global-forms.css is gone).
 *
 * What used to be `!important` CSS on MUI classes lives here as
 * styleOverrides/defaultProps, so a screen can still override it with `sx`.
 */
import { createTheme } from '@mui/material/styles';
import { light, dark, severity, radius, shadow, type, breakpoints } from './tokens';

const rem = (px) => `${px / 16}rem`;

// In CSS-variables mode `theme.palette` is FROZEN to the default (light)
// scheme; only `theme.vars.palette` resolves to `var(--mui-palette-…)` and
// therefore flips with dark mode. Every override below reads colours through
// this, never through `t.palette` directly.
const pal = (t) => (t.vars || t).palette;

// The one card surface: <Card>, <Paper variant="card"> and elevation={3}
// Papers (which global-forms.css used to give a border and shadow).
const card = (t) => ({
  border: `1px solid ${pal(t).divider}`,
  borderRadius: radius.card,
  boxShadow: shadow.card,
});

const scheme = (t, sev) => ({
  palette: {
    primary: t.primary,
    secondary: t.secondary,
    success: t.success,
    warning: t.warning,
    error: t.error,
    info: t.info,
    background: { default: t.background, paper: t.surface },
    text: { primary: t.text, secondary: t.textMuted, disabled: t.textFaint },
    divider: t.border,
    action: { hover: t.hover },
    // App-specific groups. Read as theme.palette.severity.<level>.color/bg and
    // theme.palette.surface.<name>; they become CSS variables like the rest.
    severity: sev,
    surface: { muted: t.surfaceMuted, sunken: t.surfaceSunken },
  },
});

// One theme per text direction. Always build it fresh with createTheme: spreading
// an already-built theme into createTheme drops cssVariables/colorSchemes, and
// dark mode stops working. `locales` are MUI's translations (the admin's src/i18n/muiLocales.js).
export const createAppTheme = (direction = 'ltr', ...locales) => createTheme({
  direction,
  cssVariables: { colorSchemeSelector: 'data-theme' },
  colorSchemes: {
    light: scheme(light, severity.light),
    dark: scheme(dark, severity.dark),
  },
  breakpoints: { values: breakpoints },
  shape: { borderRadius: radius.md },
  typography: {
    fontFamily: type.family,
    fontSize: type.size.base,
    h1: { fontWeight: type.weight.bold, fontSize: rem(32), lineHeight: 1.2 },
    h2: { fontWeight: type.weight.bold, fontSize: rem(28), lineHeight: 1.25 },
    h3: { fontWeight: type.weight.semibold, fontSize: rem(24), lineHeight: 1.3 },
    h4: { fontWeight: type.weight.bold, fontSize: rem(type.size.xxl), lineHeight: 1.3 },
    h5: { fontWeight: type.weight.semibold, fontSize: rem(type.size.xl), lineHeight: 1.35 },
    h6: { fontWeight: type.weight.semibold, fontSize: rem(type.size.lg), lineHeight: 1.4 },
    subtitle1: { fontWeight: type.weight.medium, fontSize: rem(type.size.base) },
    subtitle2: { fontWeight: type.weight.semibold, fontSize: rem(type.size.md) },
    body1: { fontSize: rem(type.size.base) },
    body2: { fontSize: rem(type.size.md) },
    button: { fontWeight: type.weight.medium, textTransform: 'none' },
    caption: { fontSize: rem(type.size.xs) },
    overline: { fontSize: rem(type.size.xs), letterSpacing: '0.06em', textTransform: 'uppercase' },
    // Not a MUI built-in: <Typography variant="mono"> for ids, times, log lines.
    mono: { fontFamily: type.mono, fontSize: rem(type.size.sm) },
  },
  zIndex: {
    // Screens open inside TopBar dialogs (Events, Logs, Schema); their menus,
    // tooltips and popovers must sit above the modal layer, not under it.
    modal: 1300,
    snackbar: 1400,
    tooltip: 1500,
  },
  components: {
    MuiButtonBase: {
      defaultProps: { disableRipple: false },
      styleOverrides: {
        // One focus ring for everything clickable; mouse clicks stay quiet.
        root: ({ theme: t }) => ({
          '&.Mui-focusVisible': {
            outline: `2px solid ${pal(t).primary.main}`,
            outlineOffset: 2,
          },
        }),
      },
    },
    MuiButton: {
      defaultProps: { disableElevation: true },
      styleOverrides: {
        root: { borderRadius: radius.control, fontWeight: type.weight.medium },
        sizeSmall: { fontSize: rem(type.size.sm), padding: '3px 10px' },
      },
    },
    MuiIconButton: { defaultProps: { size: 'small' } },
    // `mono` needs no component variant: Typography renders any typography key.
    MuiTypography: { defaultProps: { variantMapping: { mono: 'span' } } },
    MuiPaper: {
      defaultProps: { elevation: 0 },
      styleOverrides: {
        rounded: { borderRadius: radius.lg },
        // global-forms.css gave every elevation={3} Paper a border and a soft
        // shadow; kept, so those screens look the same.
        elevation3: ({ theme: t }) => card(t),
      },
      variants: [
        // <Paper variant="card">: the same surface as <Card>.
        { props: { variant: 'card' }, style: ({ theme: t }) => card(t) },
      ],
    },
    MuiCard: { defaultProps: { elevation: 0 }, styleOverrides: { root: ({ theme: t }) => card(t) } },
    MuiTextField: { defaultProps: { size: 'small' } },
    MuiFormControl: { defaultProps: { size: 'small' } },
    MuiSelect: { defaultProps: { size: 'small' } },
    MuiOutlinedInput: {
      styleOverrides: {
        root: ({ theme: t }) => ({
          borderRadius: radius.control,
          backgroundColor: pal(t).surface.muted,
          transition: 'background-color 0.2s',
          '&:hover': { backgroundColor: pal(t).action.hover },
          '&.Mui-focused': { backgroundColor: pal(t).background.paper },
          // Focus is the primary colour now (it was the orange secondary via
          // global-forms.css) — one focus colour across inputs and buttons.
          '&.Mui-focused .MuiOutlinedInput-notchedOutline': { borderWidth: 2 },
        }),
      },
    },
    MuiInputBase: { styleOverrides: { root: { fontSize: rem(type.size.base) } } },
    MuiInputLabel: { styleOverrides: { root: { fontWeight: type.weight.medium } } },
    MuiFormHelperText: { styleOverrides: { root: { fontSize: rem(type.size.sm) } } },
    MuiFormControlLabel: { styleOverrides: { label: { fontWeight: type.weight.medium } } },
    // Switches were brand-orange app-wide (global-forms.css); keep that look.
    MuiSwitch: { defaultProps: { color: 'secondary' } },
    MuiChip: {
      defaultProps: { size: 'small' },
      styleOverrides: {
        root: { borderRadius: radius.md, fontWeight: type.weight.medium, fontSize: rem(type.size.sm), maxWidth: '100%' },
        sizeSmall: { height: 24 },
        // Chip already ellipsises its label; the cap stops one long resource
        // name from making a chip wider than the row that holds it.
        label: { maxWidth: 280 },
      },
    },
    MuiTooltip: {
      defaultProps: { arrow: true, enterDelay: 300 },
      styleOverrides: { tooltip: { fontSize: rem(type.size.sm) } },
    },
    MuiAlert: { styleOverrides: { root: { borderRadius: radius.lg } } },
    MuiDialog: {
      defaultProps: { fullWidth: true, maxWidth: 'md' },
      styleOverrides: {
        paper: { borderRadius: radius.card, boxShadow: shadow.overlay },
        // A fullscreen dialog fills the screen: square corners, no shadow.
        paperFullScreen: { borderRadius: 0, boxShadow: 'none' },
      },
    },
    MuiDialogTitle: { styleOverrides: { root: { fontWeight: type.weight.semibold, fontSize: rem(type.size.lg) } } },
    MuiDialogActions: {
      styleOverrides: {
        // Dialog primaries were brand-orange (Save / Create / Update) and
        // Cancel a plain text button; unchanged, minus the !important.
        root: ({ theme: t }) => ({
          '& .MuiButton-contained:not(.MuiButton-containedError):not(.MuiButton-containedWarning)': {
            backgroundColor: pal(t).secondary.main,
            color: pal(t).secondary.contrastText,
            '&:hover': { backgroundColor: pal(t).secondary.dark },
          },
          '& .MuiButton-text': { color: pal(t).text.primary },
        }),
      },
    },
    MuiTableCell: {
      styleOverrides: {
        root: ({ theme: t }) => ({
          fontSize: rem(type.size.md),
          padding: '8px 14px',
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          maxWidth: 320,
          borderBottom: `1px solid ${pal(t).divider}`,
        }),
        head: ({ theme: t }) => ({
          fontWeight: type.weight.semibold,
          fontSize: rem(type.size.xs),
          letterSpacing: '0.04em',
          textTransform: 'uppercase',
          color: pal(t).text.secondary,
          position: 'sticky',
          top: 0,
          zIndex: 2,
          backgroundColor: pal(t).surface.muted,
          borderBottom: `2px solid ${pal(t).divider}`,
          paddingTop: 10,
          paddingBottom: 10,
        }),
      },
    },
    MuiTableRow: {
      styleOverrides: {
        root: ({ theme: t }) => ({
          '&.MuiTableRow-hover:hover, &:hover': { backgroundColor: pal(t).action.hover },
        }),
      },
    },
    MuiTableSortLabel: {
      styleOverrides: {
        root: ({ theme: t }) => ({
          fontWeight: type.weight.semibold,
          '&.Mui-active, &.Mui-active .MuiTableSortLabel-icon': { color: pal(t).secondary.main },
        }),
      },
    },
    MuiTablePagination: {
      styleOverrides: {
        selectLabel: ({ theme: t }) => ({ fontSize: rem(type.size.sm), color: pal(t).text.secondary }),
        displayedRows: ({ theme: t }) => ({ fontSize: rem(type.size.sm), color: pal(t).text.secondary }),
      },
    },
    MuiSkeleton: { defaultProps: { animation: 'wave' }, styleOverrides: { root: { borderRadius: radius.md } } },
    // Menu paper is capped so a long option (a customer name like "LAURUS
    // AFRICA SECURITIES LTD") can't stretch the dropdown across the screen;
    // the item then ellipsises inside it instead of overflowing.
    MuiMenu: { styleOverrides: { paper: { maxWidth: 'min(480px, calc(100vw - 32px))' } } },
    MuiMenuItem: {
      styleOverrides: {
        root: {
          fontSize: rem(type.size.base),
          overflow: 'hidden',
          // MenuItem is a flex row: a label element only shrinks once its
          // automatic min-width is cleared. ListItemIcon is left alone so its
          // gutter survives.
          '& > .MuiBox-root, & > .MuiTypography-root, & > span': {
            minWidth: 0,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          },
        },
      },
    },
  },
}, ...locales);

export default createAppTheme('ltr');
