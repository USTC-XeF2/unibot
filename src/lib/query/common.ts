export function isValidUserId(userId: string | null | undefined): boolean {
  return typeof userId === "string" && userId.trim().length > 0;
}

const GROUP_ID_RE = /^[A-Za-z0-9_-]+$/;

export function isValidGroupId(groupId: string | null | undefined): boolean {
  return typeof groupId === "string" && GROUP_ID_RE.test(groupId);
}

export function stableGroupIdsKey(groupIds: string[]): string {
  return [...groupIds].sort().join(",");
}
