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

// VIS-11 (trail LAW 2 manual viewport): after the initial auto-follow, the
// user can scroll left to the loaded root ancestors without the next frame
// snapping back to the active end.
test("vis-11 trail horizontal scroll matches approved snapshot", async ({
  page,
}) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-11-trail-horizontal-scroll"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-11-trail-horizontal-scroll.png",
  );
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

// VIS-10 (trail LAW 5 / FIP-D1 acceptance): a sidebar deep-link constructs
// the whole trail from a fresh state — every ancestor column open with its
// selection emphasized and the target file's detail panel open.
test("vis-10 trail deep link matches approved snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-10-trail-deep-link"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-10-trail-deep-link.png",
  );
});

// VIS-13 (FMR-1): filtered entries are explicit without replacing the
// actionable rows in the same Trail column.
test("vis-13 trail directory omissions match approved snapshot", async ({
  page,
}) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-13-trail-directory-omissions"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-13-trail-directory-omissions.png",
  );
});

// VIS-14 (FMR-3): a PDF selection stays in native Trail authority and renders
// an explicit metadata-only fallback instead of binary garbage or a blank.
test("vis-14 trail metadata preview matches approved snapshot", async ({
  page,
}) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-14-trail-metadata-preview"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-14-trail-metadata-preview.png",
  );
});
