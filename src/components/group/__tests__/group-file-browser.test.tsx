import { QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { toast } from "sonner";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { TooltipProvider } from "@/components/ui/tooltip";
import { createTestQueryClient } from "@/test/utils";
import type { GroupFile, GroupFolder } from "@/types/group";
import GroupFileBrowser from "../group-file-browser";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

function renderBrowser() {
  const queryClient = createTestQueryClient();

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>{children}</TooltipProvider>
      </QueryClientProvider>
    );
  }

  return render(<GroupFileBrowser userId="12345" groupId="group_1" />, {
    wrapper: Wrapper,
  });
}

describe("GroupFileBrowser breadcrumb navigation", () => {
  const mockedInvoke = invoke as Mock;
  const mockedSave = save as Mock;
  const mockedOpen = open as Mock;

  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedSave.mockReset();
    mockedOpen.mockReset();
    vi.mocked(toast.success).mockReset();
    vi.mocked(toast.error).mockReset();
  });

  it("shows root breadcrumb and disabled back button initially", async () => {
    mockedInvoke.mockResolvedValue([]);
    renderBrowser();

    await waitFor(() => {
      expect(screen.getByText("全部")).toBeInTheDocument();
    });

    const backButton = screen.getByText("返回上一级").closest("button");
    expect(backButton).toBeDisabled();
  });

  it("updates breadcrumb when entering a folder", async () => {
    const folder: GroupFolder = {
      folder_id: "folder-1",
      group_id: "group_1",
      parent_folder_id: null,
      folder_name: "Documents",
      creator_user_id: "12345",
      created_at: 1,
      updated_at: 1,
      file_count: 0,
    };

    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_folders") {
        return Promise.resolve([folder]);
      }
      if (cmd === "list_group_files") {
        return Promise.resolve([]);
      }
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });

    renderBrowser();

    await waitFor(() => {
      expect(screen.getByText("Documents")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Documents"));

    await waitFor(() => {
      expect(screen.getByText(/Documents/).closest("span")).toBeInTheDocument();
    });

    const backButton = screen.getByText("返回上一级").closest("button");
    expect(backButton).not.toBeDisabled();
  });

  it("renames in place: carries folder_id and commits once on Enter", async () => {
    const folder: GroupFolder = {
      folder_id: "real-id-123",
      group_id: "group_1",
      parent_folder_id: null,
      folder_name: "Old",
      creator_user_id: "12345",
      created_at: 1,
      updated_at: 1,
      file_count: 0,
    };

    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_folders") return Promise.resolve([folder]);
      if (cmd === "list_group_files") return Promise.resolve([]);
      if (cmd === "upsert_group_folder") return Promise.resolve(folder);
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });

    renderBrowser();

    await waitFor(() => {
      expect(screen.getByText("Old")).toBeInTheDocument();
    });

    const renameButton = screen
      .getAllByRole("button")
      .find((b) => b.querySelector(".lucide-pencil"));
    expect(renameButton).toBeDefined();
    fireEvent.click(renameButton as HTMLElement);

    const input = await screen.findByDisplayValue("Old");
    fireEvent.change(input, { target: { value: "New" } });
    fireEvent.keyDown(input, { key: "Enter" });
    // The blur that follows Enter must not trigger a second upsert.
    fireEvent.blur(input);

    await waitFor(() => {
      const upserts = mockedInvoke.mock.calls.filter(
        (c) => c[0] === "upsert_group_folder",
      );
      expect(upserts).toHaveLength(1);
      expect(upserts[0][1].input.folder_id).toBe("real-id-123");
      expect(upserts[0][1].input.folder_name).toBe("New");
    });
  });

  it("does not persist a folder until a draft name is committed", async () => {
    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_folders") return Promise.resolve([]);
      if (cmd === "list_group_files") return Promise.resolve([]);
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });

    renderBrowser();

    await waitFor(() => {
      expect(screen.getByText("全部")).toBeInTheDocument();
    });

    // No create call should fire just from rendering; the draft row only
    // persists on explicit commit, never on open/cancel.
    const upserts = mockedInvoke.mock.calls.filter(
      (c) => c[0] === "upsert_group_folder",
    );
    expect(upserts).toHaveLength(0);
  });

  it("does not invoke download when save is cancelled", async () => {
    const file: GroupFile = {
      file_id: "file-1",
      group_id: "group_1",
      parent_folder_id: null,
      file_name: "report.txt",
      file_size: 7,
      file_hash: null,
      uploader_user_id: "12345",
      uploaded_at: 1,
      expire_at: null,
      download_count: 0,
      file_path: "groups/group_1/files/report.txt",
    };
    mockedSave.mockResolvedValue(null);
    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_folders") return Promise.resolve([]);
      if (cmd === "list_group_files") return Promise.resolve([file]);
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });

    renderBrowser();
    await screen.findByText("report.txt");
    const downloadButton = screen
      .getAllByRole("button")
      .find((button) => button.querySelector(".lucide-download"));
    fireEvent.click(downloadButton as HTMLElement);

    await waitFor(() => {
      expect(mockedSave).toHaveBeenCalledWith({
        defaultPath: "report.txt",
      });
    });
    expect(
      mockedInvoke.mock.calls.some((call) => call[0] === "download_group_file"),
    ).toBe(false);
  });

  it("passes the selected destination and reports success", async () => {
    const file: GroupFile = {
      file_id: "file-1",
      group_id: "group_1",
      parent_folder_id: null,
      file_name: "report.txt",
      file_size: 7,
      file_hash: null,
      uploader_user_id: "12345",
      uploaded_at: 1,
      expire_at: null,
      download_count: 0,
      file_path: "groups/group_1/files/report.txt",
    };
    mockedSave.mockResolvedValue("/downloads/report.txt");
    mockedInvoke.mockImplementation((cmd, args) => {
      if (cmd === "list_group_folders") return Promise.resolve([]);
      if (cmd === "list_group_files") return Promise.resolve([file]);
      if (cmd === "download_group_file") {
        expect(args).toEqual({
          userId: "12345",
          groupId: "group_1",
          fileId: "file-1",
          destinationPath: "/downloads/report.txt",
        });
        return Promise.resolve("/downloads/report.txt");
      }
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });

    renderBrowser();
    await screen.findByText("report.txt");
    const downloadButton = screen
      .getAllByRole("button")
      .find((button) => button.querySelector(".lucide-download"));
    fireEvent.click(downloadButton as HTMLElement);

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("download_group_file", {
        userId: "12345",
        groupId: "group_1",
        fileId: "file-1",
        destinationPath: "/downloads/report.txt",
      });
    });
    expect(toast.success).toHaveBeenCalledWith(
      "文件已下载: /downloads/report.txt",
    );
  });

  it("renders a retryable error instead of an empty file list", async () => {
    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_folders") return Promise.resolve([]);
      if (cmd === "list_group_files") {
        return Promise.reject(new Error("权限不足"));
      }
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });

    renderBrowser();

    const retryButton = await screen.findByRole("button", { name: "重试" });
    expect(screen.getByText(/加载群文件失败/)).toBeInTheDocument();

    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_folders") return Promise.resolve([]);
      if (cmd === "list_group_files") return Promise.resolve([]);
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });
    fireEvent.click(retryButton);

    await waitFor(() => {
      expect(screen.queryByText(/加载群文件失败/)).not.toBeInTheDocument();
    });
  });

  it("shows upload failure and handles the rejected promise", async () => {
    const rejection = vi.fn();
    process.on("unhandledRejection", rejection);

    mockedOpen.mockResolvedValue("/tmp/source.txt");
    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_folders") return Promise.resolve([]);
      if (cmd === "list_group_files") return Promise.resolve([]);
      if (cmd === "upload_group_file") {
        return Promise.reject(new Error("磁盘已满"));
      }
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });

    renderBrowser();
    await waitFor(() => expect(screen.getByText("全部")).toBeInTheDocument());
    fireEvent.click(screen.getByText("上传"));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        expect.stringContaining("上传文件失败"),
      );
    });

    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(rejection).not.toHaveBeenCalled();
    process.off("unhandledRejection", rejection);
  });
});
