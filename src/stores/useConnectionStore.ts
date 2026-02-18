import { create } from 'zustand';

interface ConnectionState {
  status: 'DISCONNECTED' | 'CONNECTING' | 'CONNECTED' | 'RECONNECTING' | 'ERROR';
  error: string | null;
  serverAddress: string;
  setStatus: (status: ConnectionState['status']) => void;
  setError: (error: string | null) => void;
  setServerAddress: (address: string) => void;
}

export const useConnectionStore = create<ConnectionState>((set) => ({
  status: 'DISCONNECTED',
  error: null,
  serverAddress: '',
  setStatus: (status) => set({ status }),
  setError: (error) => set({ error }),
  setServerAddress: (serverAddress) => set({ serverAddress }),
}));
