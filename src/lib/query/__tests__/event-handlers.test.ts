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
});
