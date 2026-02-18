import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";

export function AppShell() {
  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar className="hidden md:block" />
      <div className="flex-1 flex flex-col h-full overflow-hidden">
        <header className="h-14 border-b flex items-center px-4 shrink-0 bg-card">
          {/* Header/Breadcrumbs/User Menu could go here */}
          <h1 className="text-xl font-semibold">XMage Client</h1>
        </header>
        <main className="flex-1 overflow-auto p-4">
          <Outlet />
        </main>
        <footer className="h-8 border-t flex items-center px-4 shrink-0 bg-muted text-xs text-muted-foreground">
          {/* Status Bar */}
          <span className="mr-4">Connected: Localhost</span>
          <span>Version: 0.1.0</span>
        </footer>
      </div>
    </div>
  );
}
