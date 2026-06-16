import { QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import ConversationList from "@/components/chat/conversation-list";
import { COMMANDS } from "@/lib/commands";
import { useAuthStore } from "@/store/use-auth-store";
import { createTestQueryClient } from "@/test/utils";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

function renderConversationList() {
  const queryClient = createTestQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      <ConversationList onSelectedConversationChange={vi.fn()} />
    </QueryClientProvider>,
  );
}

describe("ConversationList", () => {
  const mockedInvoke = invoke as Mock;

  beforeEach(() => {
    useAuthStore.getState().setCurrentUserId("10001");
    mockedInvoke.mockReset();
    mockedInvoke.mockImplementation((cmd, args) => {
      if (cmd === COMMANDS.listUsers) {
        return Promise.resolve([
          {
            user_id: "10001",
            nickname: "Me",
            avatar: "",
            signature: "",
            account_status: "active",
          },
          {
            user_id: "10002",
            nickname: "Alice",
            avatar: "",
            signature: "",
            account_status: "active",
          },
        ]);
      }
      if (cmd === COMMANDS.listFriends) {
        return Promise.resolve(["10002"]);
      }
      if (cmd === COMMANDS.listFriendships) {
        return Promise.resolve([
          {
            friend_user_id: "10002",
            category_id: null,
          },
        ]);
      }
      if (cmd === COMMANDS.listFriendCategories) {
        return Promise.resolve([
          {
            category_id: "10001:friend:default",
            owner_user_id: "10001",
            name: "我的好友",
            sort_order: 0,
            created_at: 1,
            updated_at: 1,
          },
        ]);
      }
      if (cmd === COMMANDS.listUserGroups) {
        return Promise.resolve([
          {
            group_id: "20001",
            group_name: "456",
            owner_user_id: "10001",
            member_count: 3,
            max_member_count: 500,
            group_status: "active",
            category_id: null,
          },
        ]);
      }
      if (cmd === COMMANDS.listConversationStates) {
        return Promise.resolve([
          {
            conversation_scene: "group",
            peer_user_id: null,
            group_id: "20001",
            is_pinned: true,
            is_muted: false,
          },
        ]);
      }
      if (cmd === COMMANDS.listGroupCategories) {
        return Promise.resolve([
          {
            category_id: "10001:group:default",
            owner_user_id: "10001",
            name: "我的群聊",
            sort_order: 0,
            created_at: 1,
            updated_at: 1,
          },
          {
            category_id: "cat-1",
            owner_user_id: "10001",
            name: "项目组",
            sort_order: 1,
            created_at: 1,
            updated_at: 1,
          },
        ]);
      }
      if (
        cmd === COMMANDS.listFriendRequests ||
        cmd === COMMANDS.listGroupRequests
      ) {
        return Promise.resolve([]);
      }
      if (cmd === COMMANDS.listMessageHistory) {
        const isGroup = args.source.scene === "group";
        return Promise.resolve([
          {
            id: isGroup ? "m-group" : "m-private",
            sender_user_id: isGroup ? "10001" : "10002",
            source: args.source,
            content: [
              {
                type: "Text",
                data: { text: isGroup ? "group latest" : "hello" },
              },
            ],
            quoted_message_id: null,
            recall: { recalled: false },
            created_at: isGroup ? 200 : 100,
            bot_id: null,
          },
        ]);
      }
      return Promise.reject(new Error(`unexpected invoke: ${String(cmd)}`));
    });
  });

  it("splits messages, friends, and groups into primary views", async () => {
    renderConversationList();

    expect(await screen.findByRole("button", { name: "消息" })).toBeVisible();
    expect(screen.getByRole("button", { name: "好友" })).toBeVisible();
    expect(screen.getByRole("button", { name: "群聊" })).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "好友" }));
    expect(await screen.findByText("我的好友")).toBeVisible();
    expect(screen.getByText("Alice")).toBeVisible();
    expect(screen.queryByText("未分组")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "群聊" }));
    expect(await screen.findByText("我的群聊")).toBeVisible();
    expect(screen.getByRole("button", { name: "新建分组" })).toBeVisible();
    expect(screen.getByText("456 (3)")).toBeVisible();
    expect(screen.queryByText("未分组")).not.toBeInTheDocument();
    // 空分组（无群聊成员）不展示，但仍可在分组管理弹窗中维护
    expect(screen.queryByText("项目组")).not.toBeInTheDocument();
  });

  it("hides empty categories in the list but keeps them in the manage dialog", async () => {
    renderConversationList();

    fireEvent.click(await screen.findByRole("button", { name: "群聊" }));
    expect(await screen.findByText("我的群聊")).toBeVisible();
    expect(screen.queryByText("项目组")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "新建分组" }));
    expect(
      await screen.findByRole("button", { name: "重命名项目组" }),
    ).toBeEnabled();
  });

  it("opens grouping from the context menu instead of listing every group", async () => {
    renderConversationList();

    const groupButton = await screen.findByRole("button", {
      name: /456 \(3\)/,
    });
    fireEvent.contextMenu(groupButton);

    expect(await screen.findByText("分组")).toBeVisible();
    expect(screen.queryByText(/^移动到:/)).not.toBeInTheDocument();
  });

  it("groups pinned conversations into a single highlighted block", async () => {
    renderConversationList();

    const pinnedButton = await screen.findByRole("button", {
      name: /456 \(3\)/,
    });
    await waitFor(() => {
      // 置顶项被包裹进统一加深的整块容器，而非单条加深
      expect(pinnedButton.parentElement).toHaveClass("bg-foreground/5");
    });
  });

  it("allows default categories to be renamed and deleted from grouping dialogs", async () => {
    renderConversationList();

    fireEvent.click(await screen.findByRole("button", { name: "好友" }));
    fireEvent.click(screen.getByRole("button", { name: "新建分组" }));
    expect(
      await screen.findByRole("button", { name: "重命名我的好友" }),
    ).toBeEnabled();
    expect(screen.getByRole("button", { name: "删除我的好友" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    fireEvent.click(screen.getByRole("button", { name: "群聊" }));
    fireEvent.click(screen.getByRole("button", { name: "新建分组" }));
    expect(
      await screen.findByRole("button", { name: "重命名我的群聊" }),
    ).toBeEnabled();
    expect(screen.getByRole("button", { name: "删除我的群聊" })).toBeEnabled();
  });
});
