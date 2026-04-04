import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import faceGroups from "@/data/faces.json";

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

export const faceById = new Map(
  faceGroups.flatMap((group) => group.faces).map((face) => [face.id, face]),
);
