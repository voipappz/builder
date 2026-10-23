# Branding

`icon.svg` is the official phone-helmet mark from
https://voipappz.github.io/images/icon.svg (2026-10-23).
App, tray, installer and browser icons use this mark.

The website palette is peach `#FDAE98`, yellow `#FFF56E`, cream `#FFF8EC`,
mint `#3CC39B`, and black outlines. Builder's loading and error screens use
these colors, rounded cards and offset shadows.

Regenerate desktop assets with:

```sh
npx --yes @tauri-apps/cli@2 icon branding/icon.svg --output desktop/icons
```

`icon-source.png` is retained as the legacy keypad source; it is no longer used.
