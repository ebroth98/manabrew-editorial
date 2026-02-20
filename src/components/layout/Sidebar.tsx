import { NavLink, useNavigate } from "react-router-dom";
import { cn } from "@/lib/utils";
import {
  Home,
  Gamepad2,
  Layers,
  BookMarked,
  Settings,
  LogOut,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useAuthStore } from "@/stores/useAuthStore";
import { useConnectionStore } from "@/stores/useConnectionStore";
import { wsClient } from "@/api/websocket";

interface SidebarProps extends React.HTMLAttributes<HTMLDivElement> {}

export function Sidebar({ className }: SidebarProps) {
  const navigate = useNavigate();
  const { logout } = useAuthStore();
  const { setStatus, setServerAddress } = useConnectionStore();

  function handleDisconnect() {
    wsClient.disconnect();
    setStatus("DISCONNECTED");
    setServerAddress("");
    logout();
    navigate("/login");
  }

  return (
    <div
      className={cn(
        "pb-12 w-full h-full bg-background flex flex-col border-r",
        className,
      )}
    >
      <div className="flex-1 space-y-4 py-4">
        <div className="px-3 py-2">
          <h2 className="mb-2 px-4 text-lg font-semibold tracking-tight">
            Bardidina Magica
          </h2>
          <div className="space-y-1">
            <NavLink to="/lobby">
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start"
                >
                  <Home className="mr-2 h-4 w-4" />
                  Lobby
                </Button>
              )}
            </NavLink>
            <NavLink to="/deck-editor">
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start"
                >
                  <Layers className="mr-2 h-4 w-4" />
                  Deck Editor
                </Button>
              )}
            </NavLink>
            <NavLink to="/my-decks">
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start"
                >
                  <BookMarked className="mr-2 h-4 w-4" />
                  My Decks
                </Button>
              )}
            </NavLink>
            <NavLink to="/matches">
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start"
                >
                  <Gamepad2 className="mr-2 h-4 w-4" />
                  Active Matches
                </Button>
              )}
            </NavLink>
          </div>
        </div>
        <div className="px-3 py-2">
          <h2 className="mb-2 px-4 text-lg font-semibold tracking-tight">
            Settings
          </h2>
          <div className="space-y-1">
            <NavLink to="/settings">
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start"
                >
                  <Settings className="mr-2 h-4 w-4" />
                  Preferences
                </Button>
              )}
            </NavLink>
          </div>
        </div>
      </div>
      <div className="px-3 py-4 border-t">
        <Button
          variant="ghost"
          className="w-full justify-start text-red-500 hover:text-red-600 hover:bg-red-100"
          onClick={handleDisconnect}
        >
          <LogOut className="mr-2 h-4 w-4" />
          Disconnect
        </Button>
      </div>
    </div>
  );
}
