export function isValidUserId(userId: number): boolean {
  return Number.isInteger(userId) && userId > 10000 && userId <= 4294967295;
}

export function stableGroupIdsKey(groupIds: number[]): string {
  return [...groupIds].sort((a, b) => a - b).join(",");
}
