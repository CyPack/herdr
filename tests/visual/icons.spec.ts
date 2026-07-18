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

// TP-FIP-VIS-03: a mixed-kind directory (directory, both symlink kinds,
// broken link, special, source/config/document/image/archive files) renders
// its ASCII-profile icons from real exported Ratatui cells.
test("vis-03 ascii icon classes match approved snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-03-icons-ascii"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot("vis-03-icons-ascii.png");
});

// TP-FIP-VIS-04: the same mixed-kind state stays legible on a tiny 60x16
// screen — icons survive complete while names truncate by display cells.
test("vis-04 tiny screen icons match approved snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-04-icons-tiny"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot("vis-04-icons-tiny.png");
});
