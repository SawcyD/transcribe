import { lazy, Suspense } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "../components/layout/AppLayout";

// Every window (main, dock, bubble, agent dock) loads this same app.
// Loading each screen on demand means the small floating windows don't parse
// every page at startup, and the main window shows Home without waiting for
// Settings, History, etc.
const RecordingOverlay = lazy(() => import("../overlay/RecordingOverlay").then((m) => ({ default: m.RecordingOverlay })));
const AssistantDrawer = lazy(() => import("../assistant/AssistantDrawer").then((m) => ({ default: m.AssistantDrawer })));
const AgentDock = lazy(() => import("../agent/AgentDock").then((m) => ({ default: m.AgentDock })));
const Whiteboard = lazy(() => import("../whiteboard/Whiteboard").then((m) => ({ default: m.Whiteboard })));
const Planner = lazy(() => import("../planner/Planner").then((m) => ({ default: m.Planner })));
const HomePage = lazy(() => import("../pages/HomePage").then((m) => ({ default: m.HomePage })));
const DictationPage = lazy(() => import("../pages/DictationPage").then((m) => ({ default: m.DictationPage })));
const HistoryPage = lazy(() => import("../pages/HistoryPage").then((m) => ({ default: m.HistoryPage })));
const DictionaryPage = lazy(() => import("../pages/DictionaryPage").then((m) => ({ default: m.DictionaryPage })));
const TransformsPage = lazy(() => import("../pages/TransformsPage").then((m) => ({ default: m.TransformsPage })));
const AssistantPage = lazy(() => import("../pages/AssistantPage").then((m) => ({ default: m.AssistantPage })));
const SettingsPage = lazy(() => import("../pages/SettingsPage").then((m) => ({ default: m.SettingsPage })));
const AboutPage = lazy(() => import("../pages/AboutPage").then((m) => ({ default: m.AboutPage })));

export function AppRouter() {
  return (
    <HashRouter>
      <Suspense fallback={null}>
        <Routes>
          <Route path="/overlay" element={<RecordingOverlay />} />
          <Route path="/assistant" element={<AssistantDrawer />} />
          <Route path="/agent" element={<AgentDock />} />
          <Route path="/whiteboard" element={<Whiteboard />} />
          <Route path="/planner" element={<Planner />} />
          <Route element={<AppLayout />}>
            <Route index element={<HomePage />} />
            <Route path="dictation" element={<DictationPage />} />
            <Route path="history" element={<HistoryPage />} />
            <Route path="dictionary" element={<DictionaryPage />} />
            <Route path="transforms" element={<TransformsPage />} />
            <Route path="assistant-settings" element={<AssistantPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="about" element={<AboutPage />} />
          </Route>
          <Route path="*" element={<Navigate replace to="/" />} />
        </Routes>
      </Suspense>
    </HashRouter>
  );
}
