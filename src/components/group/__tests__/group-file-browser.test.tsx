import { QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { createTestQueryClient } from "@/test/utils";
import type { GroupFolder } from "@/types/group";
import GroupFileBrowser from "../group-file-browser";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

function renderBrowser() {
  const queryClient = createTestQueryClient();

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  }

  return render(<GroupFileBrowser userId="12345" groupId="group_1" />, {
    wrapper: Wrapper,
  });
}

describe("GroupFileBrowser breadcrumb navigation", () => {
  const mockedInvoke = invoke as Mock;

  beforeEach(() => {
    mockedInvoke.mockReset();
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
});
