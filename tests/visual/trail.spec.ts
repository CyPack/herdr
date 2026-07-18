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

// VIS-07 (trail LAW 1/2): four accumulated Miller trail columns rendered
// from real exported Ratatui cells — the selected entry stays emphasized in
// every ancestor column and the deepest column is inside the window.
test("vis-07 trail depth matches approved snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-07-trail-depth"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot("vis-07-trail-depth.png");
});

// VIS-08 (trail LAW 1 rebranch): after reselecting a sibling in the ROOT
// column the old branch is discarded — only root + the new docs column
// remain, with the file selection emphasized, never a placeholder column.
test("vis-08 trail rebranch matches approved snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-08-trail-rebranch"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-08-trail-rebranch.png",
  );
});

// VIS-09 (trail LAW 3): a file activation opens the resizable right-side
// detail panel — bordered, titled with the file name, showing the prepared
// text preview — while the sibling columns stay visible left of it.
test("vis-09 trail detail panel matches approved snapshot", async ({
  page,
}) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-09-trail-detail-panel"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-09-trail-detail-panel.png",
  );
});
