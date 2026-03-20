import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { ThemeProvider } from "next-themes";
import { queryClient } from "@/api/queryClient";
import { router } from "@/router";
import { Toaster } from "@/components/ui/sonner";
import { useAppTheme } from "@/hooks/useAppTheme";

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
        </QueryClientProvider>
      </ThemeApplicator>
    </ThemeProvider>
  );
}

export default App;
