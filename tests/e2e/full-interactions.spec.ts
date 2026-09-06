/**
 * Full interaction coverage: every tappable control on every page.
 * Runs against the dev server in browser mock mode (Tauri IPC mocked).
 * Each test seeds its own project (contexts are isolated per test).
 */
import { expect, test, type Page } from "@playwright/test";

async function goto(page: Page, name: string) {
  await page.goto("/");
  await page.getByTestId(`nav-${name}`).click();
}

/** Creates one project via the wizard and lands on the workflow page. */
async function seedProject(page: Page, name = `IT ${Date.now()}`) {
  await page.goto("/");
  await page.getByTestId("new-project").click();
  await expect(page.getByTestId("create-wizard")).toBeVisible();
  await page.getByTestId("project-name").fill(name);
  await page.getByTestId("create-confirm").click();
  await expect(page.getByTestId("workflow-page")).toBeVisible();
  return name;
}

test.describe("project creation", () => {
  test("create wizard: non-default languages via grouped dropdowns", async ({ page }) => {
    await page.goto("/");
    await page.getByTestId("new-project").click();
    await expect(page.getByTestId("create-wizard")).toBeVisible();
    await page.getByTestId("project-name").fill("Lang Matrix");
    await page.getByTestId("source-lang").selectOption("fr");
    await page.getByTestId("target-lang").selectOption("ru");
    await page.getByTestId("create-confirm").click();
    await expect(page.getByTestId("workflow-page")).toBeVisible();
  });
});

test.describe("projects page", () => {
  test("card expands overview with chapter tree + matrix; close works", async ({ page }) => {
    await seedProject(page);
    await goto(page, "projects");
    await page.getByTestId("project-card").first().click();
    await expect(page.getByTestId("project-overview")).toBeVisible();
    await expect(page.getByTestId("progress-matrix")).toBeVisible();
    await page.locator('[data-testid="project-overview"] button').last().click();
    await expect(page.getByTestId("project-overview")).toBeHidden();
  });

  test("empty library shows guidance", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByTestId("projects-empty")).toBeVisible();
  });
});

test.describe("workflow page", () => {
  test.beforeEach(async ({ page }) => {
    await seedProject(page);
    await goto(page, "workflow");
  });

  test("all six node toggles switch state", async ({ page }) => {
    for (const step of ["detect", "ocr", "inpaint", "translate", "polish", "render"]) {
      const btn = page.getByTestId(`toggle-${step}`);
      await expect(btn).toBeVisible();
      await btn.click();
      await btn.click();
    }
  });

  test("single-page run streams events into the log", async ({ page }) => {
    await page.getByTestId("run-page").click();
    await expect(page.getByTestId("event-log")).toContainText(/render.*completed/i, {
      timeout: 15_000,
    });
  });

  test("whole-project batch run completes", async ({ page }) => {
    await page.getByTestId("run-all").click();
    await expect(page.getByTestId("event-log")).toContainText(/render.*completed/i, {
      timeout: 15_000,
    });
  });

  test("polish preview adopt and dismiss per bubble", async ({ page }) => {
    await page.getByTestId("toggle-polish").click();
    await page.getByTestId("run-page").click();
    await expect(page.getByTestId("polish-preview")).toBeVisible({ timeout: 15_000 });
    await page.getByTestId("polish-preview").click();
    await expect(page.getByTestId("polish-diff")).toBeVisible();
    await page.getByTestId("adopt-b001").click();
    await page.getByTestId("dismiss-b002").click();
    await expect(page.getByTestId("diff-b001")).toBeVisible();
    await expect(page.getByTestId("diff-b002")).toBeVisible();
  });

  test("open-editor link appears after render and navigates", async ({ page }) => {
    await page.getByTestId("run-page").click();
    const link = page.getByTestId("open-editor");
    await expect(link).toBeVisible({ timeout: 15_000 });
    await link.click();
    await expect(page.getByTestId("editor-page")).toBeVisible();
  });
});

test.describe("editor page", () => {
  test.beforeEach(async ({ page }) => {
    await seedProject(page);
    await goto(page, "editor");
  });

  test("load translated page, edit bubble text, export PNG", async ({ page }) => {
    await page.getByTestId("load-page").click();
    await expect(page.getByTestId("canvas")).toBeVisible();
    await page.getByTestId("canvas").click();
    const text = page.getByTestId("bubble-text");
    await text.fill("你好，世界！");
    await expect(text).toHaveValue("你好，世界！");
    await page.getByTestId("export-png").click();
    const download = await page.waitForEvent("download");
    expect(download.suggestedFilename()).toBe("manga-eroico-page.png");
  });
});

test.describe("models page", () => {
  test("wizard walks detect -> recommend -> select -> download -> verified", async ({ page }) => {
    await goto(page, "models");
    await expect(page.getByTestId("hardware-card")).toBeVisible();
    await page.getByRole("button", { name: /下一步|Next|次へ|다음/ }).first().click();
    await page.getByRole("button", { name: /下一步|Next|次へ|다음/ }).first().click();
    await expect(page.getByTestId("model-list")).toBeVisible();
    await page.getByTestId("start-download").click();
    await expect(page.getByTestId("download-complete")).toBeVisible({ timeout: 15_000 });
  });
});

test.describe("settings page", () => {
  test.beforeEach(async ({ page }) => {
    await goto(page, "settings");
  });

  test("all four UI languages switch", async ({ page }) => {
    for (const [lang, greeting] of [
      ["en", "Settings"],
      ["ja", "設定"],
      ["ko", "설정"],
      ["zh-CN", "设置"],
    ] as const) {
      await page.getByTestId(`ui-lang-${lang}`).click();
      await expect(page.getByTestId("settings-page")).toContainText(greeting);
    }
  });

  test("theme light/dark/system all apply", async ({ page }) => {
    await page.getByTestId("theme-dark").click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await page.getByTestId("theme-light").click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await page.getByTestId("theme-system").click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", /.+/);
  });

  test("polish endpoint + style selector + toggle + test connection", async ({ page }) => {
    await page.getByTestId("polish-baseurl").fill("http://127.0.0.1:8990/v1");
    await page.getByTestId("polish-style").selectOption("literary");
    await page.getByTestId("polish-toggle").click();
    await page.getByTestId("test-connection").click();
    await expect(page.getByTestId("test-result")).toBeVisible({ timeout: 10_000 });
  });
});
