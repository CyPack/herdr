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

// TP-FIP-VIS-02 / TP-TRAIL-T7-RENDER-05: after descending through the
// nonzero child `beta` into `deep`, the accumulating Trail keeps both exact
// ancestor selections visible and never substitutes the first row.
test("vis-02 trail retains exact ancestor highlights", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-02-resident-focus"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot("vis-02-resident-focus.png");
});
