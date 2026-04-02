import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { ThemeProvider } from "next-themes";
import { queryClient } from "@/api/queryClient";
import { router } from "@/router";
import { Toaster } from "@/components/ui/sonner";
import { useAppTheme } from "@/hooks/useAppTheme";
import { lazy, Suspense } from "react";

const DevToolsPanel = import.meta.env.DEV
  ? lazy(() => import("@/components/dev/DevToolsPanel").then((m) => ({ default: m.DevToolsPanel })))
  : () => null;

function ThemeApplicator({ children }: { children: React.ReactNode }) {
  useAppTheme();
  return <>{children}</>;
}

function App() {
  return (
    <ThemeProvider attribute="class" defaultTheme="dark" enableSystem>
      <ThemeApplicator>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
          <Toaster />
          {import.meta.env.DEV && (
            <Suspense>
              <DevToolsPanel />
            </Suspense>
          )}
        </QueryClientProvider>
      </ThemeApplicator>
    </ThemeProvider>
  );
}

export default App;
