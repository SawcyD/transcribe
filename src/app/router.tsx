import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "../components/layout/AppLayout";
import { RecordingOverlay } from "../overlay/RecordingOverlay";
import { BuddyOverlay } from "../buddy/BuddyOverlay";
import { AssistantDrawer } from "../assistant/AssistantDrawer";
import { AboutPage } from "../pages/AboutPage";
import { AssistantPage } from "../pages/AssistantPage";
import { DictationPage } from "../pages/DictationPage";
import { DictionaryPage } from "../pages/DictionaryPage";
import { HistoryPage } from "../pages/HistoryPage";
import { HomePage } from "../pages/HomePage";
import { SettingsPage } from "../pages/SettingsPage";
import { TransformsPage } from "../pages/TransformsPage";

export function AppRouter() {
  return (
    <HashRouter>
      <Routes>
        <Route path="/overlay" element={<RecordingOverlay />} />
        <Route path="/buddy" element={<BuddyOverlay />} />
        <Route path="/assistant" element={<AssistantDrawer />} />
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
    </HashRouter>
  );
}
