import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";
import Lobby from "@/views/Lobby";
import DeckEditor from "@/views/DeckEditor";
import MyDecks from "@/views/MyDecks";
import Game from "@/views/Game";
import Play from "@/views/Play";
import Draft from "@/views/Draft";
import Settings from "@/views/Settings";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      {
        index: true,
        element: <Navigate to="/lobby" replace />,
      },
      {
        path: "play",
        element: <Play />,
      },
      {
        path: "lobby",
        element: <Lobby />,
      },
      {
        path: "deck-editor",
        element: <DeckEditor />,
      },
      {
        path: "my-decks",
        element: <MyDecks />,
      },
      {
        path: "game/:gameId",
        element: <Game />,
      },
      {
        path: "draft/:draftId",
        element: <Draft />,
      },
      {
        path: "settings",
        element: <Settings />,
      },
    ],
  },
]);
