import { create } from 'zustand';
import { persist, devtools } from 'zustand/middleware';
import type { User } from '@/types/openmagic';

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  token: string | null;
  lastServer: string;
  lastUsername: string;
  login: (user: User, token: string) => void;
  logout: () => void;
  setLastConnection: (server: string, username: string) => void;
}

export const useAuthStore = create<AuthState>()(
  devtools(persist(
    (set) => ({
      user: null,
      isAuthenticated: false,
      token: null,
      lastServer: '',
      lastUsername: '',
      login: (user, token) => set({ user, isAuthenticated: true, token }),
      logout: () => set({ user: null, isAuthenticated: false, token: null }),
      setLastConnection: (lastServer, lastUsername) => set({ lastServer, lastUsername }),
    }),
    {
      name: 'openmagic-auth-storage',
      partialize: (state) => ({
        lastServer: state.lastServer,
        lastUsername: state.lastUsername,
        token: state.token, // Maybe don't persist token for security, but usually convenient
      }),
    }
  ), { name: "auth", enabled: import.meta.env.DEV })
);
