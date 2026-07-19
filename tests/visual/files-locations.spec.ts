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

const cases = [
  {
    name: "vis-18-files-locations-wide",
    width: 220,
    height: 40,
    contract:
      "wide composition preserves agents, 24-cell locations, Trail, and detail",
  },
  {
    name: "vis-19-files-locations-home-origin",
    width: 120,
    height: 28,
    contract: "deep descent retains the last explicit Home origin",
  },
  {
    name: "vis-20-files-locations-nested-origin",
    width: 44,
    height: 24,
    contract: "nested favorite wins only after its explicit activation",
  },
  {
    name: "vis-21-files-locations-standard",
    width: 44,
    height: 24,
    contract: "standard rail coexists with fractional Trail edges",
  },
  {
    name: "vis-22-files-locations-compact-closed",
    width: 30,
    height: 24,
    contract: "compact closed state gives the full body to Trail",
  },
  {
    name: "vis-23-files-locations-compact-open",
    width: 30,
    height: 24,
    contract: "compact Locations action opens a bounded drawer",
  },
  {
    name: "vis-24-files-locations-pending",
    width: 96,
    height: 24,
    contract: "pending root keeps the prior Trail visible",
  },
  {
    name: "vis-25-files-locations-failure",
    width: 96,
    height: 24,
    contract: "typed root failure keeps the prior Trail visible",
  },
] as const;

// VIS-18..25 (FCL-6): each snapshot is rendered from the real Ratatui
// TestBackend buffer. Chromium only maps exact prepared cells to pixels.
for (const visual of cases) {
  test(`${visual.name}: ${visual.contract}`, async ({ page }) => {
    const fixture = loadGenerated(visual.name);
    expect(fixture).toMatchObject({
      name: visual.name,
      width: visual.width,
      height: visual.height,
    });
    expect(fixture.cells).toHaveLength(fixture.width * fixture.height);

    await renderFixture(page, fixture);
    await expect(page.locator("#grid")).toHaveScreenshot(`${visual.name}.png`);
  });
}
