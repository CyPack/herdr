import { test, expect } from "@playwright/test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const harness = fileURLToPath(new URL("./harness/grid.html", import.meta.url));

type CellFixture = {
  name: string;
  width: number;
  height: number;
  cells: unknown[];
};

function loadGenerated(name: string): CellFixture {
  const path = fileURLToPath(
    new URL(`./fixtures/generated/${name}.json`, import.meta.url),
  );
  return JSON.parse(readFileSync(path, "utf8")) as CellFixture;
}

// VIS-12: a real Ratatui Trail buffer at an offset inside the second
// mixed-width column. Both the clipped leading suffix and the beginning of
// the trailing column must remain visible.
test("vis-12 fractional miller scroll matches approved snapshot", async ({
  page,
}) => {
  const fixture = loadGenerated("vis-12-fractional-miller-scroll");
  expect(fixture).toMatchObject({
    name: "vis-12-fractional-miller-scroll",
    width: 60,
    height: 20,
  });
  expect(fixture.cells).toHaveLength(fixture.width * fixture.height);

  await page.goto(`file://${harness}`);
  await page.evaluate(
    (value) => (window as any).renderFixture(value),
    fixture,
  );
  await expect(page.locator("#grid")).toHaveScreenshot(
    "vis-12-fractional-miller-scroll.png",
  );
});
