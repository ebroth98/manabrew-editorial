import { createBrowserRouter, Navigate } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";
import Login from "@/views/Login";
import Lobby from "@/views/Lobby";
import DeckEditor from "@/views/DeckEditor";
import MyDecks from "@/views/MyDecks";
import Game from "@/views/Game";
import Draft from "@/views/Draft";

export const router = createBrowserRouter([
  {
    path: "/",
    element: <Navigate to="/login" replace />,
  },
  {
    path: "/login",
    element: <Login />,
  },
  {
    path: "/",
    element: <AppShell />,
    children: [
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
    ],
  },
]);
