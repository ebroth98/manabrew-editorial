import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { ThemeProvider } from "next-themes";
import { queryClient } from "@/api/queryClient";
import { router } from "@/router";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useTheme } from "@/hooks/useTheme";
import { useGameDevStore } from "@/stores/useGameDevStore";
import { lazy, Suspense, useEffect } from "react";
import { toast } from "sonner";
import { getPlatformType } from "@/platform";
import { initApp } from "@/lib/appInit";

void initApp();
const DevToolsPanel = import.meta.env.DEV
  ? lazy(() => import("@/components/dev/DevToolsPanel").then((m) => ({ default: m.DevToolsPanel })))
  : () => null;

function ThemeApplicator({ children }: { children: React.ReactNode }) {
  useTheme();
  return <>{children}</>;
}

function PlatformRuntimeChecks() {
  useEffect(() => {
    if (getPlatformType() !== "web") return;

    const isolated = window.crossOriginIsolated;
    const hasSharedArrayBuffer = typeof window.SharedArrayBuffer !== "undefined";

    if (!isolated || !hasSharedArrayBuffer) {
      console.error(
        "[WebRuntime] Browser deployment is missing cross-origin isolation. SharedArrayBuffer game flow will fail.",
        {
          crossOriginIsolated: isolated,
          hasSharedArrayBuffer,
        },
      );
      toast.error(
        "Web deployment is missing required isolation headers. Ask infra to enable COOP/COEP through the Twingate/SSO path.",
        { duration: 12000 },
      );
      return;
    }

    console.info("[WebRuntime] Cross-origin isolation is enabled.");
  }, []);

  return null;
}

function App() {
  const devToolsEnabled = useGameDevStore((s) => s.devToolsEnabled);

  return (
    <ThemeProvider attribute="class" defaultTheme="dark" enableSystem>
      <ThemeApplicator>
        <QueryClientProvider client={queryClient}>
          <TooltipProvider delayDuration={120} skipDelayDuration={300}>
            <PlatformRuntimeChecks />
            <RouterProvider router={router} />
            <Toaster />
            {import.meta.env.DEV && devToolsEnabled && (
              <Suspense>
                <DevToolsPanel />
              </Suspense>
            )}
          </TooltipProvider>
        </QueryClientProvider>
      </ThemeApplicator>
    </ThemeProvider>
  );
}

export default App;
