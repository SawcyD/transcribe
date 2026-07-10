import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "../components/layout/AppLayout";
import { RecordingOverlay } from "../overlay/RecordingOverlay";
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
        <Route element={<AppLayout />}>
          <Route index element={<HomePage />} />
          <Route path="history" element={<HistoryPage />} />
          <Route path="dictionary" element={<DictionaryPage />} />
          <Route path="transforms" element={<TransformsPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
        <Route path="*" element={<Navigate replace to="/" />} />
      </Routes>
    </HashRouter>
  );
}
