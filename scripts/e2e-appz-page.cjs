// Opens the Builder page in Chromium and checks what a user sees in appz
// mode: the built-in chat renders with no page errors, there are no
// Admin/User previews, and the appz's tab frames its first page (the Calls
// dashboard at /app/dashboard, a LiveView rendering the catalog's React).
//
//   node scripts/e2e-appz-page.cjs <builder-url> [screenshot.png]
//
// Playwright comes from NODE_PATH (CI: the app checkout's node_modules).
const { chromium } = require('playwright');

const [url, shot] = process.argv.slice(2);
const redact = (s) => String(s).replace(/secret=[^&\s]+/g, 'secret=…');

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1400, height: 850 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push('page error: ' + e.message));
  page.on('requestfailed', (r) => errors.push('request failed: ' + redact(r.url())));
  let ok = true;
  const check = (cond, label) => { console.log(`${cond ? '✓' : '✕'} ${label}`); ok &&= cond; };
  try {
    await page.goto(url);
    await page.getByText('What should we build?').waitFor({ timeout: 30000 });
    check(true, 'the built-in chat renders');
    const loaded = await page.evaluate(() => [...document.scripts].map((s) => s.src).find((s) => s.includes('/builder/client.js')));
    check(Boolean(loaded), 'the chat comes from Builder itself (/builder/client.js)');
    check(await page.getByRole('tab', { name: 'Admin' }).count() === 0, 'no Admin/User previews');
    check(await page.getByRole('tab', { name: 'Status' }).count() === 1, 'the Status tab is there');
    const frame = page.frameLocator('iframe').first();
    await frame.getByText('Recent calls').first().waitFor({ timeout: 60000 });
    check(true, "the appz's tab shows its Calls dashboard");
    check(errors.length === 0, 'no page errors' + (errors.length ? ':\n  ' + errors.join('\n  ') : ''));
  } catch (e) {
    check(false, redact(e.message));
  } finally {
    if (shot) await page.screenshot({ path: shot }).catch(() => {});
    await browser.close();
  }
  process.exit(ok ? 0 : 1);
})();
