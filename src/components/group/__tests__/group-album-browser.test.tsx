import { QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { toast } from "sonner";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { TooltipProvider } from "@/components/ui/tooltip";
import { createTestQueryClient } from "@/test/utils";
import GroupAlbumBrowser from "../group-album-browser";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string) => path,
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

  return render(<GroupAlbumBrowser userId="12345" groupId="group_1" />, {
    wrapper: Wrapper,
  });
}

describe("GroupAlbumBrowser error handling", () => {
  const mockedInvoke = invoke as Mock;
  const mockedOpen = open as Mock;

  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedOpen.mockReset();
    vi.mocked(toast.error).mockReset();
  });

  it("renders album query failure instead of an empty grid", async () => {
    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_albums") {
        return Promise.reject(new Error("无权访问"));
      }
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });

    renderBrowser();

    const retryButton = await screen.findByRole("button", { name: "重试" });
    expect(screen.getByText(/加载相册失败/)).toBeInTheDocument();

    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_albums") return Promise.resolve([]);
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });
    fireEvent.click(retryButton);

    await waitFor(() => {
      expect(screen.queryByText(/加载相册失败/)).not.toBeInTheDocument();
    });
  });

  it("shows photo upload failure and handles the rejected promise", async () => {
    const rejection = vi.fn();
    process.on("unhandledRejection", rejection);

    mockedOpen.mockResolvedValue("/tmp/photo.png");
    mockedInvoke.mockImplementation((cmd) => {
      if (cmd === "list_group_albums") {
        return Promise.resolve([
          {
            album_id: "album-1",
            group_id: "group_1",
            name: "Vacation",
            cover_url: null,
            photo_count: 0,
            created_at: 1,
            updated_at: 1,
          },
        ]);
      }
      if (cmd === "list_group_photos") return Promise.resolve([]);
      if (cmd === "upload_group_photo") {
        return Promise.reject(new Error("磁盘已满"));
      }
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });

    renderBrowser();
    fireEvent.click(await screen.findByText("Vacation"));
    fireEvent.click(await screen.findByText("上传照片"));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        expect.stringContaining("上传照片失败"),
      );
    });

    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(rejection).not.toHaveBeenCalled();
    process.off("unhandledRejection", rejection);
  });
});
