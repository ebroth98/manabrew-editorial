import { NavLink } from "react-router-dom";
import { cn } from "@/lib/utils";
import { useGameStore } from "@/stores/useGameStore";
import { Home, Gamepad2, Layers, Settings, Swords, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { OpenMagicLogo } from "./OpenMagicLogo";

interface SidebarProps extends React.HTMLAttributes<HTMLDivElement> {
  onNavigate?: () => void;
}

export function Sidebar({ className, onNavigate }: SidebarProps) {
  const isGameActive = useGameStore((s) => s.isGameActive);

  return (
    <div
      className={cn(
        "pb-12 w-full h-full bg-background flex flex-col border-r overflow-hidden",
        className,
      )}
    >
      <div className="flex-1 space-y-4 py-4">
        <div className="px-3 py-2">
          <div className="mb-2 px-4 flex items-center gap-2 overflow-hidden">
            <OpenMagicLogo size={48} className="rounded-xl shrink-0" />
            <h2 className="text-lg font-semibold tracking-tight truncate">OpenMagic</h2>
          </div>
          <div className="space-y-1">
            <NavLink to="/play" onClick={onNavigate}>
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start whitespace-nowrap"
                >
                  <Swords className="mr-2 h-4 w-4 shrink-0" />
                  Play
                </Button>
              )}
            </NavLink>
            <NavLink to="/lobby" onClick={onNavigate}>
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start whitespace-nowrap"
                >
                  <Home className="mr-2 h-4 w-4 shrink-0" />
                  Lobby
                </Button>
              )}
            </NavLink>
            <NavLink to="/search" onClick={onNavigate}>
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start whitespace-nowrap"
                >
                  <Search className="mr-2 h-4 w-4 shrink-0" />
                  Card Search
                </Button>
              )}
            </NavLink>
            <NavLink to="/deck-editor" onClick={onNavigate}>
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start whitespace-nowrap"
                >
                  <Layers className="mr-2 h-4 w-4 shrink-0" />
                  My Decks
                </Button>
              )}
            </NavLink>
            <NavLink to="/matches" onClick={onNavigate}>
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start whitespace-nowrap"
                >
                  <Gamepad2 className="mr-2 h-4 w-4 shrink-0" />
                  Active Matches
                </Button>
              )}
            </NavLink>
          </div>
        </div>
        <div className="px-3 py-2">
          <h2 className="mb-2 px-4 text-lg font-semibold tracking-tight">Settings</h2>
          <div className="space-y-1">
            {isGameActive ? (
              <Button
                variant="ghost"
                className="w-full justify-start"
                disabled
                title="Preferences are unavailable during an active game"
              >
                <Settings className="mr-2 h-4 w-4" />
                Preferences
              </Button>
            ) : (
              <NavLink to="/settings" onClick={onNavigate}>
                {({ isActive }) => (
                  <Button
                    variant={isActive ? "secondary" : "ghost"}
                    className="w-full justify-start whitespace-nowrap"
                  >
                    <Settings className="mr-2 h-4 w-4 shrink-0" />
                    Preferences
                  </Button>
                )}
              </NavLink>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
