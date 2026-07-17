import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const harness = fileURLToPath(new URL("./harness/grid.html", import.meta.url));

function loadGenerated(name: string): unknown {
  const path = fileURLToPath(
    new URL(`./fixtures/generated/${name}.json`, import.meta.url),
  );
  return JSON.parse(readFileSync(path, "utf8"));
}

// TP-FIP-VIS-01: the default terminal Stage and the activated Native Files
// Stage render from real exported Ratatui cells and match approved snapshots.
test("vis-01 terminal stage matches approved snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate((f) => (window as any).renderFixture(f), loadGenerated("vis-01-terminal"));
  await expect(page.locator("#grid")).toHaveScreenshot("vis-01-terminal.png");
});

test("vis-01 files stage matches approved snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate((f) => (window as any).renderFixture(f), loadGenerated("vis-01-files"));
  await expect(page.locator("#grid")).toHaveScreenshot("vis-01-files.png");
});
