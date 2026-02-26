import { NavLink } from "react-router-dom";
import { cn } from "@/lib/utils";
import { useGameStore } from "@/stores/useGameStore";
import {
  Home,
  Gamepad2,
  Layers,
  BookMarked,
  Settings,
  Swords,
} from "lucide-react";
import { Button } from "@/components/ui/button";

interface SidebarProps extends React.HTMLAttributes<HTMLDivElement> {}

export function Sidebar({ className }: SidebarProps) {
  const isGameActive = useGameStore((s) => s.isGameActive);

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
            <NavLink to="/play">
              {({ isActive }) => (
                <Button
                  variant={isActive ? "secondary" : "ghost"}
                  className="w-full justify-start"
                >
                  <Swords className="mr-2 h-4 w-4" />
                  Play
                </Button>
              )}
            </NavLink>
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
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
