import { test, expect, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const harness = fileURLToPath(new URL("./harness/grid.html", import.meta.url));

type FixtureCell = {
  symbol: string;
  modifiers: string[];
};

type CellFixture = {
  name: string;
  width: number;
  height: number;
  cells: FixtureCell[];
};

function loadGenerated(name: string): CellFixture {
  const path = fileURLToPath(
    new URL(`./fixtures/generated/${name}.json`, import.meta.url),
  );
  return JSON.parse(readFileSync(path, "utf8")) as CellFixture;
}

async function renderFixture(page: Page, fixture: CellFixture) {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (value: CellFixture) => (window as any).renderFixture(value),
    fixture,
  );
}

// VIS-15 (MTIME-1..4): directories and files share one descending mtime
// order, split only by deterministic local-calendar section headers.
test("vis-15 miller mtime groups match approved snapshot", async ({ page }) => {
  const fixture = loadGenerated("vis-15-mtime-groups");
  expect(fixture).toMatchObject({
    name: "vis-15-mtime-groups",
    width: 96,
    height: 24,
  });
  expect(fixture.cells).toHaveLength(fixture.width * fixture.height);

  await renderFixture(page, fixture);
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-15-mtime-groups.png",
  );
});

// VIS-16 (MTIME-4): a narrow, partially clipped column never paints a
// timestamp fragment over the file name or complete action controls.
test("vis-16 narrow miller groups match approved snapshot", async ({
  page,
}) => {
  const fixture = loadGenerated("vis-16-mtime-groups-narrow");
  expect(fixture).toMatchObject({
    name: "vis-16-mtime-groups-narrow",
    width: 44,
    height: 16,
  });
  expect(fixture.cells).toHaveLength(fixture.width * fixture.height);

  await renderFixture(page, fixture);
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-16-mtime-groups-narrow.png",
  );
});

// VIS-17 (MTIME-5): after refresh reorders an exact path into a newer date
// section, the same row remains selected rather than preserving its old index.
test("vis-17 mtime reorder keeps exact selection", async ({ page }) => {
  const fixture = loadGenerated("vis-17-mtime-reorder-selection");
  expect(fixture).toMatchObject({
    name: "vis-17-mtime-reorder-selection",
    width: 72,
    height: 18,
  });
  expect(fixture.cells).toHaveLength(fixture.width * fixture.height);

  await renderFixture(page, fixture);
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-17-mtime-reorder-selection.png",
  );
});
