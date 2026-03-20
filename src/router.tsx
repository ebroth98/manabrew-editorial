import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import Lobby from "@/views/Lobby";
import DeckEditor from "@/views/DeckEditor";
import MyDecks from "@/views/MyDecks";
import Game from "@/views/Game";
import Play from "@/views/Play";
import Draft from "@/views/Draft";
import Settings from "@/views/Settings";
import Search from "@/views/Search";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppShell />,
    errorElement: (
      <div className="flex items-center justify-center h-screen text-red-400">
        <div className="text-center">
          <h1 className="text-2xl font-bold">Page Not Found</h1>
          <p className="mt-2 text-muted-foreground">The page you're looking for doesn't exist.</p>
          <a href="/" className="mt-4 inline-block text-blue-400 hover:underline">Go Home</a>
        </div>
      </div>
    ),
    children: [
      {
        index: true,
        element: <Navigate to="/lobby" replace />,
      },
      {
        path: "play",
        element: (
          <ErrorBoundary context="Play">
            <Play />
          </ErrorBoundary>
        ),
      },
      {
        path: "lobby",
        element: (
          <ErrorBoundary context="Lobby">
            <Lobby />
          </ErrorBoundary>
        ),
      },
      {
        path: "search",
        element: (
          <ErrorBoundary context="Search">
            <Search />
          </ErrorBoundary>
        ),
      },
      {
        path: "deck-editor",
        element: (
          <ErrorBoundary context="Deck Editor">
            <DeckEditor />
          </ErrorBoundary>
        ),
      },
      {
        path: "my-decks",
        element: (
          <ErrorBoundary context="My Decks">
            <MyDecks />
          </ErrorBoundary>
        ),
      },
      {
        path: "game/:gameId",
        element: (
          <ErrorBoundary context="Game">
            <Game />
          </ErrorBoundary>
        ),
      },
      {
        path: "draft/:draftId",
        element: (
          <ErrorBoundary context="Draft">
            <Draft />
          </ErrorBoundary>
        ),
      },
      {
        path: "matches",
        element: (
          <div className="flex flex-col items-center justify-center h-full text-center gap-3">
            <div className="text-4xl opacity-20">🚧</div>
            <h2 className="text-lg font-semibold">Active Matches</h2>
            <p className="text-sm text-muted-foreground">This feature is currently under development.</p>
          </div>
        ),
      },
      {
        path: "settings",
        element: (
          <ErrorBoundary context="Settings">
            <Settings />
          </ErrorBoundary>
        ),
      },
    ],
  },
]);
