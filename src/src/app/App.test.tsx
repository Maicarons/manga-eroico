import { describe, expect, it } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import App from "@/App";
import "../i18n";

function renderApp(initial = "/projects") {
  return render(
    <MemoryRouter initialEntries={[initial]}>
      <App />
    </MemoryRouter>,
  );
}

describe("App shell", () => {
  it("renders projects page with empty state", () => {
    renderApp();
    expect(screen.getByTestId("projects-page")).toBeInTheDocument();
    expect(screen.getByTestId("projects-empty")).toBeInTheDocument();
  });

  it("creates a project through the wizard and lands on workflow", async () => {
    renderApp();
    fireEvent.click(screen.getByTestId("new-project"));
    fireEvent.change(screen.getByTestId("project-name"), { target: { value: "Test Manga" } });
    fireEvent.click(screen.getByTestId("create-confirm"));
    // navigation happens after the async create call resolves
    await screen.findByTestId("workflow-page");
    expect(screen.getByTestId("workflow-canvas")).toBeInTheDocument();
  });

  it("workflow page has six node toggles with polish disabled by default", () => {
    renderApp("/workflow");
    for (const step of ["detect", "ocr", "inpaint", "translate", "polish", "render"]) {
      expect(screen.getByTestId(`toggle-${step}`)).toBeInTheDocument();
    }
    const polish = screen.getByTestId("toggle-polish");
    expect(polish.textContent).toContain("⚪"); // disabled glyph
  });

  it("runs a mocked pipeline and reaches completed state", async () => {
    renderApp("/workflow");
    fireEvent.click(screen.getByTestId("run-page"));
    // mock pipeline emits 6 steps x 2 events with 120ms gaps
    await waitFor(
      () => expect(screen.getByTestId("event-log").textContent).toContain("[render] completed"),
      { timeout: 4000 },
    );
  });
});
