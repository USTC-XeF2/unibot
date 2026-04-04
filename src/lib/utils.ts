import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function resolveUserDisplayName(
  userId: number,
  nickname?: string,
  cards?: Record<number, { card: string }>,
): string {
  return cards?.[userId]?.card?.trim() || nickname?.trim() || `用户 ${userId}`;
}
