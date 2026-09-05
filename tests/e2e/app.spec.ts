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

  test("model wizard: detect, recommend, select, mock download", async ({ page }) => {
    await page.getByRole("link", { name: /模型管理|Models|モデル管理|모델 관리/ }).click();
    // step 1: hardware detection
    await expect(page.getByTestId("hardware-card")).toBeVisible();
    const next = page.getByRole("button", { name: /下一步|Next|次へ|다음/ });
    // step 2: tier recommendation
    await next.click();
    // step 3: model selection
    await next.click();
    await expect(page.getByTestId("model-ppocrv5_det")).toBeVisible();
    await expect(page.getByTestId("start-download")).toBeEnabled();
    // step 4: mock download streams to 100% and everything verifies
    await page.getByTestId("start-download").click();
    await expect(page.getByTestId("dl-ppocrv5_det")).toBeVisible();
    await expect(page.getByTestId("download-complete")).toBeVisible({ timeout: 15_000 });
  });

  test("M3 loop: run pipeline, open editor, fine-tune, export", async ({ page }) => {
    await page.getByRole("link", { name: /工作流|Workflow|ワークフロー|워크플로/ }).click();
    const run = page.getByTestId("run-page");
    await run.click();
    // mock pipeline streams events; render completion reveals the editor link
    await expect(page.getByTestId("open-editor")).toBeVisible({ timeout: 15_000 });
    await page.getByTestId("open-editor").click();
    await expect(page.getByTestId("editor-page")).toBeVisible();
    await expect(page.getByTestId("canvas")).toBeVisible();
    // fine-tune: edit the selected bubble text
    await page.getByTestId("bubble-text").fill("你好，世界！");
    // export triggers a PNG download from the Konva stage
    const downloadPromise = page.waitForEvent("download");
    await page.getByTestId("export-png").click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toBe("manga-eroico-page.png");
  });

  test("M3: project overview shows chapter tree and progress matrix", async ({ page }) => {
    await page.getByTestId("new-project").click();
    await page.getByTestId("project-name").fill("M3 Overview");
    await page.getByTestId("create-confirm").click();
    await expect(page.getByTestId("project-card").first()).toBeVisible();
    await page.getByTestId("project-card").first().click();
    await expect(page.getByTestId("project-overview")).toBeVisible();
    await expect(page.getByTestId("progress-matrix")).toBeVisible();
    await expect(page.getByTestId("row-pg_mock_2")).toBeVisible();
  });

  test("M3: polish preview adopt and dismiss per bubble", async ({ page }) => {
    await page.getByRole("link", { name: /工作流|Workflow|ワークフロー|워크플로/ }).click();
    // enable the polish node, run the page
    await page.getByTestId("toggle-polish").click();
    await page.getByTestId("run-page").click();
    await expect(page.getByTestId("polish-preview")).toBeVisible({ timeout: 15_000 });
    await page.getByTestId("polish-preview").click();
    await expect(page.getByTestId("polish-diff")).toBeVisible();
    await page.getByTestId("adopt-b001").click();
    await page.getByTestId("dismiss-b002").click();
    // adopted bubbles stay highlighted; dismissed ones fade
    await expect(page.getByTestId("diff-b001")).toBeVisible();
    await expect(page.getByTestId("diff-b002")).toBeVisible();
  });
});
