import { useEffect } from "react";
import { Route, Routes, Navigate } from "react-router-dom";
import Layout from "./app/Layout";
import ProjectsPage from "@/features/projects/ProjectsPage";
import WorkflowPage from "@/features/workflow/WorkflowPage";
import EditorPage from "@/features/editor/EditorPage";
import ModelsPage from "@/features/models/ModelsPage";
import SettingsPage from "@/features/settings/SettingsPage";
import { useTheme } from "@/lib/theme";
import { useProjects } from "@/features/projects/projectsStore";

export default function App() {
  const theme = useTheme((s) => s.theme);

  useEffect(() => {
    document.documentElement.dataset["theme"] = theme;
  }, [theme]);

  // Reopen the last active project on the backend at startup (Tauri state
  // is process-local; browser mock mode resolves instantly).
  useEffect(() => {
    void useProjects.getState().restore();
  }, []);

  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<Navigate to="/projects" replace />} />
        <Route path="/projects" element={<ProjectsPage />} />
        <Route path="/workflow" element={<WorkflowPage />} />
        <Route path="/editor" element={<EditorPage />} />
        <Route path="/models" element={<ModelsPage />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  );
}
