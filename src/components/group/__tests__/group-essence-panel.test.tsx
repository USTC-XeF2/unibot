import { QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { confirmDialog } from "@/lib/modal";
import { createTestQueryClient } from "@/test/utils";
import GroupEssencePanel from "../group-essence-panel";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@/lib/modal", () => ({
  confirmDialog: vi.fn(),
}));

function renderPanel() {
  const queryClient = createTestQueryClient();

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  }

  return render(
    <GroupEssencePanel userId="10001" groupId="20001" canManage />,
    { wrapper: Wrapper },
  );
}

describe("GroupEssencePanel", () => {
  const mockedInvoke = invoke as Mock;
  const mockedConfirm = confirmDialog as Mock;

  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedConfirm.mockReset();
    mockedConfirm.mockResolvedValue(true);
  });

  it("unsets a deleted-source essence by essence id", async () => {
    mockedInvoke.mockImplementation((command) => {
      if (command === "list_group_essence_messages") {
        return Promise.resolve([
          {
            essence_id: "essence-1",
            group_id: "20001",
            message_id: "",
            sender_user_id: "10002",
            operator_user_id: "10001",
            is_set: true,
            content: [],
            created_at: 100,
          },
        ]);
      }
      if (command === "set_group_essence_message") {
        return Promise.resolve({
          essence_id: "essence-1",
          group_id: "20001",
          message_id: "",
          sender_user_id: "10002",
          operator_user_id: "10001",
          is_set: false,
          content: [],
          created_at: 100,
        });
      }
      return Promise.reject(new Error(`unexpected invoke: ${String(command)}`));
    });

    renderPanel();
    fireEvent.click(await screen.findByRole("button", { name: "取消精华" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("set_group_essence_message", {
        userId: "10001",
        groupId: "20001",
        update: { type: "unset", essence_id: "essence-1" },
      });
    });
  });
});
