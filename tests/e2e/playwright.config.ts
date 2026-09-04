import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  timeout: 60_000,
  use: {
    baseURL: "http://localhost:5173",
  },
  webServer: {
    command: "pnpm --filter @manga-eroico/app dev",
    url: "http://localhost:5173",
    reuseExistingServer: true,
    timeout: 120_000,
  },
  projects: [
    {
      name: "chromium",
      use: { browserName: "chromium" },
    },
  ],
});
