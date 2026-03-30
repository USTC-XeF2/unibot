import "./App.css";
import NiceModal from "@ebay/nice-modal-react";
import { QueryClientProvider } from "@tanstack/react-query";
import { createHashRouter, RouterProvider } from "react-router";
import { TooltipProvider } from "@/components/ui/tooltip";
import MainLayout from "@/layouts/MainLayout";
import { queryClient } from "@/lib/query-client";
import ChatWindowView from "@/views/chat/chat-window";
import AboutView from "@/views/main/about";
import DashboardView from "@/views/main/dashboard";
import LogsView from "@/views/main/logs";
import SettingsView from "@/views/main/settings";
import UserSettingsView from "@/views/main/user-settings";
import UserManagementView from "@/views/main/users";

function App() {
  const router = createHashRouter([
    {
      path: "/",
      element: <MainLayout />,
      children: [
        { index: true, element: <DashboardView /> },
        { path: "users", element: <UserManagementView /> },
        { path: "user/:userId", element: <UserSettingsView /> },
        { path: "logs", element: <LogsView /> },
        { path: "settings", element: <SettingsView /> },
        { path: "about", element: <AboutView /> },
      ],
    },
    {
      path: "/chat/:userId",
      element: <ChatWindowView />,
    },
  ]);

  return (
    <NiceModal.Provider>
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>
          <RouterProvider router={router} />
        </TooltipProvider>
      </QueryClientProvider>
    </NiceModal.Provider>
  );
}

export default App;
