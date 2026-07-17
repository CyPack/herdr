import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const harness = fileURLToPath(new URL("./harness/grid.html", import.meta.url));

function loadFixture(name: string): { cells: Array<{ symbol: string }> } {
  const path = fileURLToPath(new URL(`./fixtures/${name}.json`, import.meta.url));
  return JSON.parse(readFileSync(path, "utf8"));
}

test("the unmutated grid matches its approved snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate((f) => (window as any).renderFixture(f), loadFixture("self-test"));
  // Approved baselines are only written by an explicit --update-snapshots run;
  // playwright.config.ts pins updateSnapshots: "none" for ordinary runs.
  await expect(page.locator("#grid")).toHaveScreenshot("mutation-base.png");
});

test("a one-cell mutation changes the rendered pixels", async ({ page }) => {
  await page.goto(`file://${harness}`);
  const fixture = loadFixture("self-test");
  await page.evaluate((f) => (window as any).renderFixture(f), fixture);
  const base = await page.locator("#grid").screenshot();

  // Mutate the visible white-on-default cell; a fg==bg cell would be a
  // pixel-invisible mutation and prove nothing.
  fixture.cells[1].symbol = "Z";
  await page.evaluate((f) => (window as any).renderFixture(f), fixture);
  const mutated = await page.locator("#grid").screenshot();

  expect(mutated.equals(base), "one visible-cell mutation must change pixels").toBe(false);
});
