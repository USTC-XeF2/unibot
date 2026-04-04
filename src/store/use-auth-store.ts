import { create } from "zustand";

type AuthState = {
  currentUserId: number | null;
  setCurrentUserId: (userId: number | null) => void;
};

export const useAuthStore = create<AuthState>((set) => ({
  currentUserId: null,
  setCurrentUserId: (userId) => set({ currentUserId: userId }),
}));
