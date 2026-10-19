// Synthetic rendered lane: exercise the checks without claiming full explorer coverage.
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { chromium } from 'playwright-core';
const require = createRequire(import.meta.url);
const out = new URL('../../../target/ux-qa/', import.meta.url);
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.CHROME_BIN || '/usr/bin/google-chrome', headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
  // Semantic tokens make the fixture's target size and edge clearance explicit.
  await page.setContent(`<!doctype html><html lang="en"><head><title>Synthetic UX contract</title>
    <style>:root{--target-size:44px;--edge-clearance:16px;--text:#111;--surface:#fff}
    body{margin:var(--edge-clearance);color:var(--text);background:var(--surface)}
    button{min-width:var(--target-size);min-height:var(--target-size)}</style></head>
    <body><main><h1>Synthetic UX contract</h1><button>Inspect evidence</button></main></body></html>`);
  async function expectNoUxViolations() {
    const violations = await page.locator('button').evaluateAll(buttons => buttons.flatMap(button => {
      const box = button.getBoundingClientRect();
      return [box.width < 44 || box.height < 44 ? 'target size' : null,
        box.left < 16 || box.top < 16 || box.right > innerWidth - 16 || box.bottom > innerHeight - 16 ? 'edge clearance' : null].filter(Boolean);
    }));
    assert.deepEqual(violations, []);
  }
  await expectNoUxViolations();
  await page.addScriptTag({ path: require.resolve('axe-core/axe.min.js') });
  const accessibility = await page.evaluate(async () => window.axe.run(document));
  assert.deepEqual(accessibility.violations, []);
  await page.screenshot({ path: new URL('synthetic.png', out).pathname });
  await page.locator('button').evaluate(button => { button.style.cssText = 'min-width:0;min-height:0;width:8px;height:8px;position:absolute;left:16px;top:16px'; });
  await assert.rejects(expectNoUxViolations, /target size/);
  await page.locator('button').evaluate(button => { button.style.cssText = 'width:44px;height:44px;position:absolute;left:0;top:0'; });
  await assert.rejects(expectNoUxViolations, /edge clearance/);
  await page.locator('button').evaluate(button => { button.textContent = ''; });
  const negativeAccessibility = await page.evaluate(async () => window.axe.run(document));
  assert(negativeAccessibility.violations.some(v => v.id === 'button-name'));
  await writeFile(new URL('receipt.json', out), JSON.stringify({
    kind: 'synthetic-rendered-ux-contract', physical_validation: false,
    positive_layout: true, positive_accessibility: true, negative_layout_rejected: true,
    negative_accessibility_rejected: true, full_explorer_coverage: false,
  }, null, 2) + '\n');
  console.log('PASS: synthetic rendered UX positive and negative controls');
} finally { await browser.close(); }
