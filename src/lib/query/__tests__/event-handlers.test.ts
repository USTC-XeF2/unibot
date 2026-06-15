import { beforeEach, describe, expect, it, vi } from "vitest";
import { queryClient } from "@/lib/query-client";
import { handleQueryInvalidation } from "../event-handlers";

vi.mock("@/lib/query/chat", () => ({
  refetchMessageHistoryQueries: vi.fn(),
  refetchMessageHistoryQuery: vi.fn(),
  refetchPokeHistoryQueries: vi.fn(),
  refetchPokeHistoryQuery: vi.fn(),
  sourceFromInternalEvent: vi.fn(() => null),
}));

describe("handleQueryInvalidation", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("invalidates all file-folder query variants for group file events", () => {
    const invalidateSpy = vi
      .spyOn(queryClient, "invalidateQueries")
      .mockResolvedValue(undefined);

    handleQueryInvalidation("10001", {
      kind: "group_file_upserted",
      file_id: "file-1",
      group_id: "20001",
      uploader_user_id: "10002",
      time: 100,
    });

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["groups", "files", "10001", "20001"],
    });
    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["groups", "folders", "10001", "20001"],
    });
  });

  it("removes group content queries for the target user on member-left", () => {
    const removeSpy = vi
      .spyOn(queryClient, "removeQueries")
      .mockReturnValue(undefined);
    vi.spyOn(queryClient, "invalidateQueries").mockResolvedValue(undefined);

    handleQueryInvalidation("10002", {
      kind: "group_member_left",
      group_id: "20001",
      operator_user_id: "10001",
      target_user_id: "10002",
      time: 100,
    });

    expect(removeSpy).toHaveBeenCalledWith({
      queryKey: ["groups", "files", "10002", "20001"],
    });
    expect(removeSpy).toHaveBeenCalledWith({
      queryKey: ["groups", "albums", "10002", "20001"],
    });
    expect(removeSpy).toHaveBeenCalledWith({
      queryKey: ["groups", "essence", "10002", "20001"],
    });
  });

  it("ignores member-left for other users", () => {
    const removeSpy = vi
      .spyOn(queryClient, "removeQueries")
      .mockReturnValue(undefined);
    vi.spyOn(queryClient, "invalidateQueries").mockResolvedValue(undefined);

    handleQueryInvalidation("10001", {
      kind: "group_member_left",
      group_id: "20001",
      operator_user_id: "10001",
      target_user_id: "10002",
      time: 100,
    });

    expect(removeSpy).not.toHaveBeenCalled();
  });
});
