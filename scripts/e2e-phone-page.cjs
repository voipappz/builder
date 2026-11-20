// The Builder page for the phone project, in Chromium: the chat renders, the
// pane has the project's tab and Status (no voice-bot talk button), and the
// emulator's stream draws the phone (a canvas with real pixels).
//   node e2e-phone-page.cjs <builder-url> <screenshot.png>
const { chromium } = require('playwright');

(async () => {
  const [url, shot] = process.argv.slice(2);
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1500, height: 900 } });
  const fail = async (why) => { await page.screenshot({ path: shot }).catch(() => {}); console.error('✕ ' + why); await browser.close(); process.exit(1); };

  await page.goto(url);
  await page.getByText('What should we build?').waitFor({ timeout: 30000 }).catch(() => fail('the chat did not render'));
  const tabs = await page.$$eval('[role=tab]', (t) => t.map((x) => x.textContent));
  if (tabs.join('|') !== 'VoipAppz Phone|Status') await fail(`tabs are ${JSON.stringify(tabs)}, expected VoipAppz Phone and Status`);
  if (await page.$('text=Hold to talk')) await fail('a phone project shows the voice-bot talk button');

  // The stream: a canvas in the pane's frame, at least phone-sized, not blank.
  let drawn = false;
  for (let i = 0; i < 30 && !drawn; i++) {
    await page.waitForTimeout(2000);
    const frame = page.frames().find((f) => f.url().includes(':8000'));
    const canvas = frame && await frame.$('canvas, video');
    const box = canvas && await canvas.boundingBox();
    if (!box || box.width < 200 || box.height < 400) continue;
    const png = await canvas.screenshot();
    drawn = new Set(png.subarray(0, 20000)).size > 30 && png.length > 8000;
  }
  if (!drawn) await fail('the phone stream did not draw in the pane');

  await page.screenshot({ path: shot });
  console.log('✓ The chat, the VoipAppz Phone and Status tabs, and the live phone in the pane');
  await browser.close();
})();
