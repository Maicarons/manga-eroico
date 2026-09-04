import { test, expect } from "@playwright/test";

test.describe("manga-eroico web preview (mocked pipeline)", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("projects page shows empty state", async ({ page }) => {
    await expect(page.getByTestId("projects-page")).toBeVisible();
    await expect(page.getByTestId("projects-empty")).toBeVisible();
  });

  test("theme toggle switches to dark mode", async ({ page }) => {
    await page.getByRole("button", { name: "🌙" }).first().click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  });

  test("create project and run mocked pipeline end-to-end", async ({ page }) => {
    await page.getByTestId("new-project").click();
    await page.getByTestId("project-name").fill("E2E Manga");
    await page.getByTestId("create-confirm").click();

    await expect(page.getByTestId("workflow-page")).toBeVisible();
    await page.getByTestId("run-page").click();

    // mock pipeline takes ~1.5s for 6 steps
    await expect(page.getByTestId("event-log")).toContainText("[render] completed", {
      timeout: 10_000,
    });
  });

  test("model page shows hardware card and mock download", async ({ page }) => {
    await page.getByRole("link", { name: /模型管理|Models|モデル管理|모델 관리/ }).click();
    await expect(page.getByTestId("hardware-card")).toBeVisible();
    const btn = page.getByTestId("download-ppocrv5_det");
    await btn.click();
    await expect(page.getByTestId("hardware-card")).toBeVisible();
  });
});
