import { useEffect, useState } from "react";
import { usePanelRef } from "react-resizable-panels";
import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { useServerStore } from "@/stores/useServerStore";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight } from "lucide-react";
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
  const { connected, connecting, error } = useServerStore();
  const { serverHost, serverPort } = usePreferencesStore();
  const sidebarRef = usePanelRef();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const setupListeners = useServerStore((s) => s.setupListeners);
  const status = connecting
    ? "CONNECTING"
    : connected
      ? "CONNECTED"
      : error
        ? "ERROR"
        : "DISCONNECTED";
  const serverAddress = `${serverHost}:${serverPort}`;

  // Register Tauri event listeners at app level so they're always active
  useEffect(() => {
    const cleanup = setupListeners();
    return cleanup;
  }, []);

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
      <ResizablePanelGroup orientation="horizontal" className="relative flex-1 min-h-0">
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
        <ResizablePanel minSize={40} className="relative">
          <div className="hidden md:block absolute left-0 top-1/2 -translate-y-1/2 z-30 group">
            <Button
              size="icon"
              variant="ghost"
              className={cn(
                "h-20 w-3 rounded-r-md rounded-l-none border border-l-0 border-border bg-card/90 px-0",
                "translate-x-[-9px] group-hover:translate-x-0 transition-transform duration-150",
                "hover:bg-card",
              )}
              onClick={toggleSidebar}
              title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            >
              {sidebarCollapsed ? (
                <ChevronRight className="h-3 w-3" />
              ) : (
                <ChevronLeft className="h-3 w-3" />
              )}
            </Button>
          </div>
          <main className="h-full overflow-auto p-4">
            <Outlet />
          </main>
        </ResizablePanel>
      </ResizablePanelGroup>

      <footer className="h-8 border-t flex items-center px-4 shrink-0 bg-secondary text-xs text-muted-foreground gap-4">
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
