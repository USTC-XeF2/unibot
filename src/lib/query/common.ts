export function isValidUserId(userId: string | null | undefined): boolean {
  return typeof userId === "string" && userId.trim().length > 0;
}

export function stableGroupIdsKey(groupIds: string[]): string {
  return [...groupIds].sort().join(",");
}