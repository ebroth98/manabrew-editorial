import { useCallback, useEffect, useState } from "react";
import { usePanelRef } from "react-resizable-panels";
import { Outlet, useLocation } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { useServerStore } from "@/stores/useServerStore";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { useDragToggle } from "@/hooks/useDragToggle";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";

export function AppShell() {
  const sidebarRef = usePanelRef();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const setupListeners = useServerStore((s) => s.setupListeners);
  const location = useLocation();

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

  const expandSidebar = useCallback(() => {
    const panel = sidebarRef.current;
    if (panel?.isCollapsed()) {
      panel.expand();
      setSidebarCollapsed(false);
    }
  }, [sidebarRef]);

  const collapseSidebar = useCallback(() => {
    const panel = sidebarRef.current;
    if (panel && !panel.isCollapsed()) {
      panel.collapse();
      setSidebarCollapsed(true);
    }
  }, [sidebarRef]);

  const onDragMouseDown = useDragToggle(
    expandSidebar,
    collapseSidebar,
    "right",
  );

  useEffect(() => {
    if (location.pathname.startsWith("/game")) {
      collapseSidebar();
    }
  }, [location.pathname, collapseSidebar]);

  return (
    <div className="h-screen overflow-hidden">
      <ResizablePanelGroup
        orientation="horizontal"
        className="relative h-full min-h-0"
      >
        <ResizablePanel
          panelRef={sidebarRef}
          defaultSize={260}
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
                "h-24 w-4 rounded-r-md rounded-l-none border border-l-0 border-border bg-card/90 px-0",
                "translate-x-[-9px] group-hover:translate-x-0 group-hover:w-6 group-hover:h-28 transition-all duration-150",
                "hover:bg-card",
              )}
              onClick={toggleSidebar}
              onMouseDown={onDragMouseDown}
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
    </div>
  );
}
