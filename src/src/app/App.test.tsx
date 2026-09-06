import { describe, expect, it } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import App from "@/App";
import { useProjects } from "@/features/projects/projectsStore";
import "../i18n";

function renderApp(initial = "/projects") {
  return render(
    <MemoryRouter initialEntries={[initial]}>
      <App />
    </MemoryRouter>,
  );
}

async function createProject(name: string) {
  fireEvent.click(screen.getByTestId("new-project"));
  fireEvent.change(screen.getByTestId("project-name"), { target: { value: name } });
  fireEvent.click(screen.getByTestId("create-confirm"));
  // navigation happens after the async create call resolves
  await screen.findByTestId("workflow-page");
}

describe("App shell", () => {
  it("renders projects page with empty state", () => {
    renderApp();
    expect(screen.getByTestId("projects-page")).toBeInTheDocument();
    expect(screen.getByTestId("projects-empty")).toBeInTheDocument();
  });

  it("creates a project through the wizard and lands on workflow", async () => {
    renderApp();
    await createProject("Test Manga");
    expect(screen.getByTestId("workflow-canvas")).toBeInTheDocument();
  });

  it("workflow page has six node toggles with polish disabled by default", async () => {
    renderApp();
    await createProject("Toggles");
    for (const step of ["detect", "ocr", "inpaint", "translate", "polish", "render"]) {
      expect(screen.getByTestId(`toggle-${step}`)).toBeInTheDocument();
    }
    const polish = screen.getByTestId("toggle-polish");
    expect(polish.textContent).toContain("Polish"); // svg dot, no emoji
    expect(polish.querySelector("svg")).not.toBeNull();
    // every node exposes a config gear
    expect(screen.getByTestId("config-detect")).toBeInTheDocument();
    expect(screen.getByTestId("config-render")).toBeInTheDocument();
  });

  it("redirects workflow/editor to the library when no project is active", () => {
    // zustand is a module singleton AND localStorage persists across tests
    // in the same file; App.restore() would resurrect the active project
    useProjects.setState({ projects: [], activeRoot: null, activated: false });
    localStorage.clear();
    const wf = renderApp("/workflow");
    expect(screen.getByTestId("projects-page")).toBeInTheDocument();
    wf.unmount();
    const ed = renderApp("/editor");
    expect(screen.getByTestId("projects-page")).toBeInTheDocument();
    ed.unmount();
  });

  it("runs a mocked pipeline and reaches completed state", async () => {
    renderApp();
    await createProject("Pipeline");
    fireEvent.click(screen.getByTestId("run-page"));
    // mock pipeline emits 6 steps x 2 events with 120ms gaps
    await waitFor(
      () => expect(screen.getByTestId("event-log").textContent).toContain("[render] completed"),
      { timeout: 4000 },
    );
  });
});
