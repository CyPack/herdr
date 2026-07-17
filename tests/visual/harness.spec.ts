import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const harness = fileURLToPath(new URL("./harness/grid.html", import.meta.url));

function loadFixture(name: string): unknown {
  const path = fileURLToPath(new URL(`./fixtures/${name}.json`, import.meta.url));
  return JSON.parse(readFileSync(path, "utf8"));
}

test("every exported cell maps to exactly one grid position", async ({ page }) => {
  await page.goto(`file://${harness}`);
  const fixture = loadFixture("self-test") as { cells: unknown[] };
  await page.evaluate((f) => (window as any).renderFixture(f), fixture);
  await expect(page.locator(".cell")).toHaveCount(fixture.cells.length);
});

test("a wide symbol cannot shift following cells", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate((f) => (window as any).renderFixture(f), loadFixture("self-test"));
  const lefts = await page
    .locator(".cell")
    .evaluateAll((cells) => cells.map((cell) => cell.getBoundingClientRect().left));
  expect(lefts).toHaveLength(3);
  expect(lefts[1] - lefts[0]).toBeCloseTo(lefts[2] - lefts[1], 1);
});

test("a malformed fixture fails loudly instead of rendering empty", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await expect(
    page.evaluate(() => (window as any).renderFixture({ width: 2 })),
  ).rejects.toThrow(/malformed fixture/);
  await expect(page.locator("#error")).toContainText("malformed fixture");
});

test("a wrong cell count fails loudly", async ({ page }) => {
  await page.goto(`file://${harness}`);
  const fixture = loadFixture("self-test") as { cells: unknown[] };
  fixture.cells = fixture.cells.slice(0, 1);
  await expect(
    page.evaluate((f) => (window as any).renderFixture(f), fixture),
  ).rejects.toThrow(/expected 3 cells/);
});

test("foreground, background, and modifiers round-trip to CSS", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate((f) => (window as any).renderFixture(f), loadFixture("self-test"));
  const first = page.locator(".cell").first();
  await expect(first).toHaveCSS("color", "rgb(1, 2, 3)");
  await expect(first).toHaveCSS("background-color", "rgb(0, 0, 0)");
  const second = page.locator(".cell").nth(1);
  await expect(second).toHaveCSS("font-weight", "700");
  const third = page.locator(".cell").nth(2);
  await expect(third).toHaveCSS("text-decoration-line", "underline");
});
