import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  forbidOnly: true,
  updateSnapshots: "none",
  use: {
    ...devices["Desktop Chrome"],
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1,
    locale: "en-US",
    timezoneId: "UTC",
    colorScheme: "dark",
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "on-first-retry",
  },
  expect: { toHaveScreenshot: { maxDiffPixels: 0 } },
  outputDir: "./test-results",
});
