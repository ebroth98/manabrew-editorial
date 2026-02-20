import { useState } from "react";
import { usePanelRef } from "react-resizable-panels";
import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { useConnectionStore } from "@/stores/useConnectionStore";
import { useAuthStore } from "@/stores/useAuthStore";
import { useTheme } from "next-themes";
import { Button } from "@/components/ui/button";
import { Sun, Moon, Menu } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";

const statusColors: Record<string, string> = {
  CONNECTED: "text-green-500",
  CONNECTING: "text-yellow-500",
  RECONNECTING: "text-yellow-500",
  DISCONNECTED: "text-muted-foreground",
  ERROR: "text-red-500",
};

export function AppShell() {
  const { status, serverAddress } = useConnectionStore();
  const { user } = useAuthStore();
  const { theme, setTheme } = useTheme();
  const sidebarRef = usePanelRef();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  function toggleSidebar() {
    const panel = sidebarRef.current;
    if (!panel) return;
    if (panel.isCollapsed()) {
      panel.expand();
      setSidebarCollapsed(false);
    } else {
      panel.collapse();
      setSidebarCollapsed(true);
    }
  }

  return (
    <div className="h-screen flex flex-col overflow-hidden">
      <header className="h-14 border-b flex items-center justify-between px-4 shrink-0 bg-card">
        <div className="flex items-center gap-2">
          {/* Hamburger — only on desktop where the sidebar lives */}
          <Button
            size="icon"
            variant="ghost"
            className="hidden md:flex h-8 w-8 shrink-0"
            onClick={toggleSidebar}
            title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            <Menu className="h-4 w-4" />
          </Button>
          <h1 className="text-xl font-semibold">Bardidina Magica Client</h1>
        </div>
        <div className="flex items-center gap-3">
          {user && (
            <span className="text-sm text-muted-foreground">
              {user.username}
            </span>
          )}
          <Button
            size="icon"
            variant="ghost"
            className="h-8 w-8"
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
            title="Toggle theme"
          >
            <Sun className="h-4 w-4 rotate-0 scale-100 transition-transform dark:-rotate-90 dark:scale-0" />
            <Moon className="absolute h-4 w-4 rotate-90 scale-0 transition-transform dark:rotate-0 dark:scale-100" />
          </Button>
        </div>
      </header>

      <ResizablePanelGroup orientation="horizontal" className="flex-1 min-h-0">
        <ResizablePanel
          panelRef={sidebarRef}
          defaultSize={100}
          minSize={14}
          maxSize={300}
          collapsible
          collapsedSize={0}
          className="hidden md:block"
        >
          <Sidebar />
        </ResizablePanel>
        <ResizableHandle withHandle className="hidden md:flex" />
        <ResizablePanel minSize={40}>
          <main className="h-full overflow-auto p-4">
            <Outlet />
          </main>
        </ResizablePanel>
      </ResizablePanelGroup>

      <footer className="h-8 border-t flex items-center px-4 shrink-0 bg-muted text-xs text-muted-foreground gap-4">
        <span
          className={cn(
            "flex items-center gap-1",
            statusColors[status] ?? "text-muted-foreground",
          )}
        >
          <span className="w-1.5 h-1.5 rounded-full bg-current inline-block" />
          {status}
        </span>
        {serverAddress && <span>{serverAddress}</span>}
        <span className="ml-auto">v0.1.0</span>
      </footer>
    </div>
  );
}
