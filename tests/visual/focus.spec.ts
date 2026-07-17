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

// TP-FIP-VIS-02: after descending through the nonzero child `beta` into
// `deep`, the resident column visibly highlights `beta`, not the first row.
test("vis-02 resident column highlights the exact entered child", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-02-resident-focus"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot("vis-02-resident-focus.png");
});
