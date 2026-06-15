import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import {
  openGroupAlbumsWindow,
  openGroupFilesWindow,
} from "../group-content-window";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    error: vi.fn(),
  },
}));

describe("group content window helpers", () => {
  const mockedInvoke = invoke as Mock;

  beforeEach(() => {
    mockedInvoke.mockReset();
    vi.mocked(toast.error).mockReset();
  });

  it("invokes the files window command with user and group ids", async () => {
    mockedInvoke.mockResolvedValue(true);

    await openGroupFilesWindow("12345", "group_1");

    expect(mockedInvoke).toHaveBeenCalledWith("open_group_files_window", {
      userId: "12345",
      groupId: "group_1",
    });
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("shows a toast when opening the files window fails", async () => {
    mockedInvoke.mockRejectedValue(new Error("无权访问"));

    await openGroupFilesWindow("12345", "group_1");

    expect(toast.error).toHaveBeenCalledWith(
      expect.stringContaining("打开群文件失败"),
    );
  });

  it("shows a toast when opening the albums window fails", async () => {
    mockedInvoke.mockRejectedValue(new Error("无权访问"));

    await openGroupAlbumsWindow("12345", "group_1");

    expect(toast.error).toHaveBeenCalledWith(
      expect.stringContaining("打开群相册失败"),
    );
  });
});
