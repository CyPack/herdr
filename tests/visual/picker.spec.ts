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

// TP-FIP-VIS-05: the blocking "Add Reference to Agent..." picker renders over
// the Files stage with the current agent first and preselected.
test("vis-05 agent picker matches approved snapshot", async ({ page }) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-05-picker"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot("vis-05-picker.png");
});

// TP-FIP-VIS-06: a disabled (vanished) target row stays visibly dimmed and
// the popup remains legible on a tiny 60x20 screen.
test("vis-06 disabled row tiny picker matches approved snapshot", async ({
  page,
}) => {
  await page.goto(`file://${harness}`);
  await page.evaluate(
    (f) => (window as any).renderFixture(f),
    loadGenerated("vis-06-picker-disabled-tiny"),
  );
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-06-picker-disabled-tiny.png",
  );
});
