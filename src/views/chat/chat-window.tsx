import { useQueries } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Check, Plus, Search, UserPlus, Users } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useParams } from "react-router";
import AddFriendDialog from "@/components/chat/add-friend-dialog";
import ChatMainPanel from "@/components/chat/chat-main-panel";
import CreateGroupDialog from "@/components/chat/create-group-dialog";
import RequestManageDialog from "@/components/chat/request-manage-dialog";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import {
  invalidateMessageHistoryQueries,
  invalidateMessageHistoryQuery,
  messageHistoryQueryOptions,
  sourceFromInternalEvent,
} from "@/lib/chat-query";
import {
  invalidateFriendsQuery,
  useFriendsQuery,
} from "@/lib/friendships-query";
import { invalidateGroupsQuery, useUserGroupsQuery } from "@/lib/groups-query";
import { confirmDialog, promptDialog } from "@/lib/modal";
import {
  invalidateFriendRequestsQuery,
  invalidateManageableGroupRequestsQueries,
  useFriendRequestsQuery,
  useManageableGroupRequestsQuery,
} from "@/lib/request-query";
import { invalidateUsersQuery, useUsersQuery } from "@/lib/users-query";
import type {
  ChatMessage,
  InternalEventPayload,
  MessageSource,
} from "@/types/chat";
import type { GroupMemberProfile, GroupRole } from "@/types/group";

type ConversationItem = {
  key: string;
  type: "private" | "group";
  source: MessageSource;
  title: string;
  avatarText: string;
  avatarUrl?: string;
};

type ConversationSnapshot = {
  lastMessage: string;
  lastAt: number;
};

function extractMessageText(message: ChatMessage | null): string {
  if (!message) {
    return "";
  }

  const parts = message.content
    .map((segment) => {
      if (segment.type === "Text") {
        return segment.data?.text ?? "";
      }
      return `[${segment.type}]`;
    })
    .filter(Boolean);

  return parts.join("") || "[空消息]";
}

function formatMessageTime(ts: number): string {
  if (!ts) {
    return "";
  }
  const date = new Date(ts * 1000);
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();

  if (sameDay) {
    return date.toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  return date.toLocaleDateString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
  });
}

function ChatWindowView() {
  const { userId } = useParams();
  const currentUserId = useMemo(() => Number(userId), [userId]);
  const usersQuery = useUsersQuery();
  const groupsQuery = useUserGroupsQuery(currentUserId);

  const users = usersQuery.data ?? [];
  const groups = groupsQuery.data ?? [];
  const friendsQuery = useFriendsQuery(currentUserId);
  const friendIds = friendsQuery.data ?? [];

  const conversations = useMemo<ConversationItem[]>(() => {
    if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
      return [];
    }

    const friendIdSet = new Set(friendIds);

    const privateConversations = users
      .filter(
        (user) =>
          user.user_id !== currentUserId && friendIdSet.has(user.user_id),
      )
      .map((user) => ({
        key: `private-${user.user_id}`,
        type: "private" as const,
        source: { scene: "private" as const, peer_user_id: user.user_id },
        title: user.nickname,
        avatarText: user.nickname.slice(0, 1).toUpperCase(),
        avatarUrl: user.avatar,
      }));

    const groupConversations = groups.map((group) => ({
      key: `group-${group.group_id}`,
      type: "group" as const,
      source: { scene: "group" as const, group_id: group.group_id },
      title: `${group.group_name} (${group.member_count})`,
      avatarText: "群",
    }));

    return [...privateConversations, ...groupConversations];
  }, [currentUserId, friendIds, groups, users]);

  const [searchText, setSearchText] = useState("");
  const [selectedConversationKey, setSelectedConversationKey] = useState<
    string | null
  >(null);
  const [addFriendOpen, setAddFriendOpen] = useState(false);
  const [createGroupOpen, setCreateGroupOpen] = useState(false);
  const [requestManageOpen, setRequestManageOpen] = useState(false);
  const [conversationActionError, setConversationActionError] = useState<
    string | null
  >(null);

  const resolveMyGroupRole = async (
    groupId: number,
  ): Promise<GroupRole | null> => {
    const members = await invoke<GroupMemberProfile[]>("list_group_members", {
      userId: currentUserId,
      groupId,
    });
    return (
      members.find((member) => member.user_id === currentUserId)?.role ?? null
    );
  };

  const handleDeleteFriend = async (peerUserId: number) => {
    const confirmed = await confirmDialog({
      title: "确认删除好友",
      description: "确认删除该好友？",
      confirmText: "删除",
    });
    if (!confirmed) {
      return;
    }
    try {
      await invoke("delete_friend", {
        userId: currentUserId,
        friendUserId: peerUserId,
      });
      setConversationActionError(null);
      await invalidateUsersQuery();
      await invalidateFriendRequestsQuery(currentUserId);
      await invalidateFriendsQuery(currentUserId);
      if (selectedConversationKey === `private-${peerUserId}`) {
        setSelectedConversationKey(null);
      }
    } catch (error) {
      setConversationActionError(error as string);
    }
  };

  const handleSetWholeMute = async (groupId: number) => {
    const input = await promptDialog({
      title: "设置全体禁言",
      description: "请输入全体禁言时长（秒，0为解除）",
      confirmText: "确定",
    });
    if (input === null) {
      return;
    }

    const duration = Number(input.trim());
    if (!Number.isInteger(duration) || duration < 0) {
      setConversationActionError("禁言时长必须为大于等于 0 的整数");
      return;
    }

    try {
      await invoke("set_group_whole_mute", {
        userId: currentUserId,
        groupId,
        durationSeconds: duration,
      });
      setConversationActionError(null);
    } catch (error) {
      setConversationActionError(error as string);
    }
  };

  const handleRenameGroup = async (groupId: number) => {
    const input = await promptDialog({
      title: "修改群昵称",
      description: "请输入新的群昵称",
      confirmText: "保存",
    });
    if (input === null) {
      return;
    }

    const name = input.trim();
    if (!name) {
      setConversationActionError("群昵称不能为空");
      return;
    }

    try {
      await invoke("rename_group", {
        userId: currentUserId,
        groupId,
        groupName: name,
      });
      setConversationActionError(null);
      await invalidateGroupsQuery();
    } catch (error) {
      setConversationActionError(error as string);
    }
  };

  const handleDissolveGroup = async (groupId: number) => {
    const confirmed = await confirmDialog({
      title: "确认解散群聊",
      description: "确认解散该群聊？该操作不可恢复。",
      confirmText: "解散",
    });
    if (!confirmed) {
      return;
    }
    try {
      await invoke("dissolve_group", {
        userId: currentUserId,
        groupId,
      });
      setConversationActionError(null);
      setSelectedConversationKey(null);
      await invalidateGroupsQuery();
    } catch (error) {
      setConversationActionError(error as string);
    }
  };

  const handleLeaveGroup = async (groupId: number) => {
    const confirmed = await confirmDialog({
      title: "确认退出群聊",
      description: "确认退出该群聊？",
      confirmText: "退出",
    });
    if (!confirmed) {
      return;
    }

    try {
      await invoke("leave_group", {
        userId: currentUserId,
        groupId,
      });
      setConversationActionError(null);
      setSelectedConversationKey(null);
      await invalidateGroupsQuery();
    } catch (error) {
      setConversationActionError(error as string);
    }
  };

  const friendRequestsQuery = useFriendRequestsQuery(currentUserId, true);
  const manageableGroupRequestsQuery = useManageableGroupRequestsQuery(
    currentUserId,
    groups,
    true,
  );
  const hasPendingRequests = useMemo(() => {
    const incomingPendingFriendRequests = (friendRequestsQuery.data ?? []).some(
      (request) =>
        request.state === "pending" && request.target_user_id === currentUserId,
    );
    const pendingManageableGroupRequests =
      (manageableGroupRequestsQuery.data ?? []).length > 0;

    return incomingPendingFriendRequests || pendingManageableGroupRequests;
  }, [
    friendRequestsQuery.data,
    manageableGroupRequestsQuery.data,
    currentUserId,
  ]);

  const snapshotQueries = useQueries({
    queries: conversations.map((conversation) =>
      messageHistoryQueryOptions(currentUserId, conversation.source, 1),
    ),
  });

  const snapshots = useMemo<Record<string, ConversationSnapshot>>(() => {
    return Object.fromEntries(
      conversations.map((conversation, index) => {
        const latestMessage = snapshotQueries[index]?.data?.[0] ?? null;
        return [
          conversation.key,
          {
            lastMessage: extractMessageText(latestMessage),
            lastAt: latestMessage?.created_at ?? 0,
          },
        ] as const;
      }),
    );
  }, [conversations, snapshotQueries]);

  const sortedConversations = useMemo(() => {
    return [...conversations].sort((a, b) => {
      const timeA = snapshots[a.key]?.lastAt ?? 0;
      const timeB = snapshots[b.key]?.lastAt ?? 0;
      if (timeA !== timeB) {
        return timeB - timeA;
      }
      return a.title.localeCompare(b.title, "zh-CN");
    });
  }, [conversations, snapshots]);

  const filteredConversations = useMemo(() => {
    const keyword = searchText.trim().toLowerCase();
    if (!keyword) {
      return sortedConversations;
    }

    return sortedConversations.filter((conversation) =>
      conversation.title.toLowerCase().includes(keyword),
    );
  }, [searchText, sortedConversations]);

  const selectedConversation = useMemo(
    () => conversations.find((item) => item.key === selectedConversationKey),
    [selectedConversationKey, conversations],
  );

  useEffect(() => {
    if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
      return;
    }

    let disposed = false;
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      unlisten = await listen<InternalEventPayload>("chat:event", (event) => {
        if (disposed) {
          return;
        }

        const payload = event.payload;
        if (!payload) {
          return;
        }

        const source = sourceFromInternalEvent(payload, currentUserId);
        if (source) {
          void invalidateMessageHistoryQuery(currentUserId, source);
          if (
            payload.kind === "group_member_joined" &&
            payload.target_user_id === currentUserId
          ) {
            void invalidateGroupsQuery();
          }
          if (
            payload.kind === "friend_request_created" ||
            payload.kind === "friend_request_handled" ||
            payload.kind === "group_request_created" ||
            payload.kind === "group_request_handled"
          ) {
            void invalidateFriendRequestsQuery(currentUserId);
            void invalidateManageableGroupRequestsQueries(currentUserId);
            if (payload.kind === "friend_request_handled") {
              void invalidateFriendsQuery(currentUserId);
            }
            if (
              payload.kind === "group_request_handled" &&
              payload.state === "accepted"
            ) {
              const shouldRefreshGroups =
                payload.initiator_user_id === currentUserId ||
                payload.target_user_id === currentUserId;
              if (shouldRefreshGroups) {
                void invalidateGroupsQuery();
              }
            }
          }
          return;
        }

        void invalidateMessageHistoryQueries(currentUserId);
      });
    };

    void setup();

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [currentUserId]);

  if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
    return (
      <main className="flex h-screen w-screen items-center justify-center bg-background p-6">
        <p className="text-muted-foreground text-sm">
          无效的用户 ID，无法打开聊天窗口。
        </p>
      </main>
    );
  }

  return (
    <main className="flex h-screen w-screen overflow-hidden bg-background">
      <aside className="flex w-70 shrink-0 flex-col border-r bg-muted/10">
        <header className="space-y-2 border-b p-3">
          <div className="flex items-center gap-2">
            <div className="relative min-w-0 flex-1">
              <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={searchText}
                onChange={(event) => setSearchText(event.target.value)}
                placeholder="搜索会话"
                className="pl-8"
              />
            </div>
            <div className="flex items-center gap-1">
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    type="button"
                    variant="outline"
                    size="icon-sm"
                    title="更多操作"
                    className="relative"
                  >
                    <Plus className="size-4" />
                    {hasPendingRequests ? (
                      <span className="absolute top-1 right-1 size-2 rounded-full bg-red-500" />
                    ) : null}
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-44 min-w-44">
                  <DropdownMenuItem
                    onSelect={(event) => {
                      event.preventDefault();
                      setAddFriendOpen(true);
                    }}
                  >
                    <UserPlus className="size-4" />
                    添加好友
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onSelect={(event) => {
                      event.preventDefault();
                      setCreateGroupOpen(true);
                    }}
                  >
                    <Users className="size-4" />
                    创建群聊
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onSelect={(event) => {
                      event.preventDefault();
                      setRequestManageOpen(true);
                    }}
                  >
                    <Check className="size-4" />
                    请求管理
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>
        </header>

        <div className="min-h-0 flex-1 overflow-auto p-2">
          {conversationActionError ? (
            <div className="mb-2 rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1 text-destructive text-xs">
              {conversationActionError}
            </div>
          ) : null}
          <div className="space-y-1">
            {filteredConversations.length === 0 ? (
              <div className="rounded-lg border border-dashed px-3 py-6 text-center text-muted-foreground text-xs">
                没有匹配的会话
              </div>
            ) : (
              filteredConversations.map((conversation) => {
                const snapshot = snapshots[conversation.key];
                const isActive = selectedConversation?.key === conversation.key;
                const privatePeerId =
                  conversation.source.scene === "private"
                    ? conversation.source.peer_user_id
                    : null;
                const groupId =
                  conversation.source.scene === "group"
                    ? conversation.source.group_id
                    : null;
                return (
                  <ContextMenu key={conversation.key}>
                    <ContextMenuTrigger asChild>
                      <button
                        type="button"
                        className={`w-full rounded-lg border px-2.5 py-2 text-left transition-colors ${
                          isActive
                            ? "border-primary/40 bg-primary/10"
                            : "border-transparent hover:border-border hover:bg-muted/40"
                        }`}
                        onClick={() =>
                          setSelectedConversationKey(conversation.key)
                        }
                      >
                        <div className="flex items-center gap-2">
                          <Avatar className="size-8">
                            <AvatarImage src={conversation.avatarUrl} />
                            <AvatarFallback>
                              {conversation.avatarText}
                            </AvatarFallback>
                          </Avatar>
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center justify-between gap-2">
                              <p className="truncate font-medium text-sm">
                                {conversation.title}
                              </p>
                              <span className="shrink-0 text-[11px] text-muted-foreground">
                                {formatMessageTime(snapshot?.lastAt ?? 0)}
                              </span>
                            </div>
                            <p className="truncate text-muted-foreground text-xs">
                              {snapshot?.lastMessage ?? ""}
                            </p>
                          </div>
                        </div>
                      </button>
                    </ContextMenuTrigger>
                    <ContextMenuContent>
                      {privatePeerId !== null ? (
                        <ContextMenuItem
                          variant="destructive"
                          onSelect={() =>
                            void handleDeleteFriend(privatePeerId)
                          }
                        >
                          删除好友
                        </ContextMenuItem>
                      ) : null}

                      {groupId !== null ? (
                        <ContextMenuItem
                          onSelect={async () => {
                            const role = await resolveMyGroupRole(groupId);
                            if (role !== "owner" && role !== "admin") {
                              setConversationActionError(
                                "仅群主或管理员可设置全体禁言",
                              );
                              return;
                            }
                            await handleSetWholeMute(groupId);
                          }}
                        >
                          设置全体禁言
                        </ContextMenuItem>
                      ) : null}
                      {groupId !== null ? (
                        <ContextMenuItem
                          onSelect={async () => {
                            const role = await resolveMyGroupRole(groupId);
                            if (role !== "owner" && role !== "admin") {
                              setConversationActionError(
                                "仅群主或管理员可修改群昵称",
                              );
                              return;
                            }
                            await handleRenameGroup(groupId);
                          }}
                        >
                          修改群昵称
                        </ContextMenuItem>
                      ) : null}
                      {groupId !== null ? <ContextMenuSeparator /> : null}
                      {groupId !== null ? (
                        <ContextMenuItem
                          variant="destructive"
                          onSelect={async () => {
                            const role = await resolveMyGroupRole(groupId);
                            if (role === "owner") {
                              await handleDissolveGroup(groupId);
                              return;
                            }
                            await handleLeaveGroup(groupId);
                          }}
                        >
                          {groups.find((item) => item.group_id === groupId)
                            ?.owner_user_id === currentUserId
                            ? "解散群聊"
                            : "退出群聊"}
                        </ContextMenuItem>
                      ) : null}
                    </ContextMenuContent>
                  </ContextMenu>
                );
              })
            )}
          </div>
        </div>
      </aside>

      {selectedConversation ? (
        <ChatMainPanel
          selectedConversation={selectedConversation}
          currentUserId={currentUserId}
          users={users}
        />
      ) : (
        <section className="flex min-w-0 flex-1 items-center justify-center">
          <p className="text-muted-foreground text-sm">
            请选择一个会话开始聊天
          </p>
        </section>
      )}

      <AddFriendDialog
        open={addFriendOpen}
        onOpenChange={setAddFriendOpen}
        currentUserId={currentUserId}
        users={users}
      />
      <CreateGroupDialog
        open={createGroupOpen}
        onOpenChange={setCreateGroupOpen}
        currentUserId={currentUserId}
        users={users}
        groups={groups}
      />
      <RequestManageDialog
        open={requestManageOpen}
        onOpenChange={setRequestManageOpen}
        currentUserId={currentUserId}
        users={users}
        groups={groups}
      />
    </main>
  );
}

export default ChatWindowView;
