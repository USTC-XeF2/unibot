import { useQueries } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import {
  Bell,
  BellOff,
  Check,
  ChevronDown,
  ChevronRight,
  FolderOpen,
  Pencil,
  Pin,
  Plus,
  Search,
  Trash2,
  UserPlus,
  Users,
} from "lucide-react";
import {
  type FormEvent,
  type ReactNode,
  useEffect,
  useMemo,
  useState,
} from "react";
import { toast } from "sonner";
import AddFriendDialog from "@/components/chat/add-friend-dialog";
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { COMMANDS } from "@/lib/commands";
import { segmentsToNodes } from "@/lib/message-content";
import { confirmDialog, promptDialog } from "@/lib/modal";
import {
  useCreateFriendCategoryMutation,
  useCreateGroupCategoryMutation,
  useDeleteFriendCategoryMutation,
  useDeleteFriendMutation,
  useDeleteGroupCategoryMutation,
  useDissolveGroupMutation,
  useLeaveGroupMutation,
  useRenameFriendCategoryMutation,
  useRenameGroupCategoryMutation,
  useRenameGroupMutation,
  useSetConversationMutedMutation,
  useSetConversationPinnedMutation,
  useSetFriendCategoryMutation,
  useSetGroupCategoryMutation,
  useSetGroupWholeMuteMutation,
} from "@/lib/mutations";
import {
  messageHistoryQueryOptions,
  useConversationStatesQuery,
  useFriendCategoriesQuery,
  useFriendRequestsQuery,
  useFriendshipsQuery,
  useGroupCategoriesQuery,
  useGroupRequestsQuery,
  useUserGroupsQuery,
  useUsersQuery,
} from "@/lib/query";
import { formatConversationPreviewTime } from "@/lib/time-format";
import { resolveUserDisplayName } from "@/lib/utils";
import { useAuthStore } from "@/store/use-auth-store";
import type { ChatMessage, MessageSource } from "@/types/chat";
import type { GroupMemberProfile, GroupRole } from "@/types/group";
import type { UserProfile } from "@/types/user";

type ConversationItem = {
  key: string;
  source: MessageSource;
  title: string;
  avatarText: string;
  avatarUrl?: string;
  isPinned: boolean;
  isMuted: boolean;
  categoryId: string | null;
};

type ConversationSnapshot = {
  lastMessage: ReactNode;
  lastAt: number;
};

type ConversationListView = "messages" | "friends" | "groups";

type ConversationSection = {
  key: string;
  title: string;
  items: ConversationItem[];
};

type CategoryDialogState =
  | {
      mode: "friend";
      peerUserId?: string;
      selectedCategoryId?: string;
    }
  | {
      mode: "group";
      groupId?: string;
      selectedCategoryId?: string;
    };

type CategoryOption = {
  id: string;
  name: string;
  sortOrder: number;
};

type ConversationListProps = {
  onSelectedConversationChange: (conversation: MessageSource | null) => void;
};

const DEFAULT_FRIEND_SECTION_KEY = "friends:default";
const DEFAULT_GROUP_SECTION_KEY = "groups:default";
const DEFAULT_FRIEND_CATEGORY_NAME = "我的好友";
const DEFAULT_GROUP_CATEGORY_NAME = "我的群聊";
const LEGACY_DEFAULT_CATEGORY_NAME = "默认分组";

function buildCategoryOptions<
  T extends { category_id: string; name: string; sort_order: number },
>(categories: T[], defaultCategoryId: string, defaultName: string) {
  return [...categories]
    .sort((a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name))
    .map((category) => ({
      id: category.category_id,
      name:
        category.category_id === defaultCategoryId &&
        category.name === LEGACY_DEFAULT_CATEGORY_NAME
          ? defaultName
          : category.name,
      sortOrder: category.sort_order,
    }));
}

function buildConversationPreview(
  latestMessage: ChatMessage | null,
  users: UserProfile[],
  currentUserId: string,
) {
  if (!latestMessage) {
    return "";
  }

  if (latestMessage.recall.recalled) {
    const recalledByUserId =
      latestMessage.recall.recalled_by_user_id ?? latestMessage.sender_user_id;
    if (recalledByUserId === currentUserId) {
      return "你撤回了一条消息";
    }
    const recalledByName = resolveUserDisplayName(
      recalledByUserId,
      users.find((user) => user.user_id === recalledByUserId)?.nickname,
    );
    return `${recalledByName}撤回了一条消息`;
  }

  return segmentsToNodes(latestMessage.content, users);
}

function buildCategorySections(
  categories: CategoryOption[],
  conversations: ConversationItem[],
  options: {
    keyPrefix: string;
    defaultCategoryId: string;
    defaultSectionKey: string;
    defaultSectionTitle: string;
  },
): ConversationSection[] {
  const sections = categories.map((category) => ({
    key: `${options.keyPrefix}:${category.id}`,
    title: category.name,
    items: conversations.filter(
      (conversation) =>
        conversation.categoryId === category.id ||
        (category.id === options.defaultCategoryId &&
          conversation.categoryId === null),
    ),
  }));

  const hasDefaultCategory = categories.some(
    (category) => category.id === options.defaultCategoryId,
  );
  if (hasDefaultCategory) {
    return sections;
  }

  return [
    ...sections,
    {
      key: options.defaultSectionKey,
      title: options.defaultSectionTitle,
      items: conversations.filter(
        (conversation) => conversation.categoryId === null,
      ),
    },
  ];
}

type CategoryManageDialogProps = {
  open: boolean;
  title: string;
  categories: CategoryOption[];
  selectedCategoryId?: string;
  onOpenChange: (open: boolean) => void;
  onCreate: (name: string) => Promise<void>;
  onRename: (categoryId: string, name: string) => Promise<void>;
  onDelete: (categoryId: string) => Promise<void>;
  onSelect?: (categoryId: string) => Promise<void>;
};

function CategoryManageDialog({
  open,
  title,
  categories,
  selectedCategoryId,
  onOpenChange,
  onCreate,
  onRename,
  onDelete,
  onSelect,
}: CategoryManageDialogProps) {
  const [newName, setNewName] = useState("");
  const [editingCategoryId, setEditingCategoryId] = useState<string | null>(
    null,
  );
  const [editingName, setEditingName] = useState("");
  const [pendingCategoryId, setPendingCategoryId] = useState<string | null>(
    null,
  );
  const [isCreating, setIsCreating] = useState(false);

  useEffect(() => {
    if (!open) {
      setNewName("");
      setEditingCategoryId(null);
      setEditingName("");
      setPendingCategoryId(null);
      setIsCreating(false);
    }
  }, [open]);

  const categoryNameExists = (name: string, exceptCategoryId?: string) =>
    categories.some(
      (category) =>
        category.name === name && category.id !== (exceptCategoryId ?? ""),
    );

  const submitCreate = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const name = newName.trim();
    if (!name) {
      toast.error("分组名称不能为空");
      return;
    }
    if (categoryNameExists(name)) {
      toast.error("分组名称不能重复");
      return;
    }

    setIsCreating(true);
    try {
      await onCreate(name);
      setNewName("");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setIsCreating(false);
    }
  };

  const submitRename = async (
    event: FormEvent<HTMLFormElement>,
    category: CategoryOption,
  ) => {
    event.preventDefault();
    const name = editingName.trim();
    if (!name) {
      toast.error("分组名称不能为空");
      return;
    }
    if (name === category.name) {
      setEditingCategoryId(null);
      setEditingName("");
      return;
    }
    if (categoryNameExists(name, category.id)) {
      toast.error("分组名称不能重复");
      return;
    }

    setPendingCategoryId(category.id);
    try {
      await onRename(category.id, name);
      setEditingCategoryId(null);
      setEditingName("");
    } catch (error) {
      toast.error(String(error));
    } finally {
      setPendingCategoryId(null);
    }
  };

  const selectCategory = async (categoryId: string) => {
    if (!onSelect) {
      return;
    }
    if (categoryId === selectedCategoryId) {
      onOpenChange(false);
      return;
    }

    setPendingCategoryId(categoryId);
    try {
      await onSelect(categoryId);
      onOpenChange(false);
    } catch (error) {
      toast.error(String(error));
    } finally {
      setPendingCategoryId(null);
    }
  };

  const deleteCategory = async (category: CategoryOption) => {
    const confirmed = await confirmDialog({
      title: "删除分组",
      description: `确认删除「${category.name}」？`,
      confirmText: "删除",
    });
    if (!confirmed) {
      return;
    }

    setPendingCategoryId(category.id);
    try {
      await onDelete(category.id);
      if (selectedCategoryId === category.id) {
        onOpenChange(false);
      }
    } catch (error) {
      toast.error(String(error));
    } finally {
      setPendingCategoryId(null);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription className="sr-only">{title}</DialogDescription>
        </DialogHeader>

        <form className="flex items-center gap-2" onSubmit={submitCreate}>
          <Input
            value={newName}
            onChange={(event) => setNewName(event.target.value)}
            placeholder="添加分组"
          />
          <Button
            type="submit"
            size="icon-sm"
            disabled={isCreating}
            aria-label="添加分组"
          >
            <Plus className="size-4" />
          </Button>
        </form>

        <div className="space-y-2">
          <div className="font-medium text-muted-foreground text-xs">
            已有分组
          </div>
          <div className="max-h-64 space-y-1 overflow-auto">
            {categories.map((category) => {
              const isSelected = category.id === selectedCategoryId;
              const isEditing = editingCategoryId === category.id;
              const isPending = pendingCategoryId === category.id;

              return (
                <div
                  key={category.id}
                  className="flex min-h-10 items-center gap-1 rounded-md border border-transparent bg-muted/30 px-1.5"
                >
                  {isEditing ? (
                    <form
                      className="flex min-w-0 flex-1 items-center gap-1"
                      onSubmit={(event) => submitRename(event, category)}
                    >
                      <Input
                        value={editingName}
                        autoFocus
                        onChange={(event) => setEditingName(event.target.value)}
                        className="h-8"
                      />
                      <Button
                        type="submit"
                        size="icon-sm"
                        disabled={isPending}
                        aria-label="保存分组名称"
                      >
                        <Check className="size-4" />
                      </Button>
                    </form>
                  ) : (
                    <>
                      <button
                        type="button"
                        className="flex h-9 min-w-0 flex-1 items-center justify-between gap-2 rounded px-2 text-left transition-colors enabled:hover:bg-background disabled:cursor-default"
                        disabled={!onSelect || isPending}
                        onClick={() => selectCategory(category.id)}
                      >
                        <span className="truncate">{category.name}</span>
                        {isSelected ? (
                          <Check className="size-4 shrink-0 text-primary" />
                        ) : null}
                      </button>
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-sm"
                        disabled={isPending}
                        aria-label={`重命名${category.name}`}
                        onClick={() => {
                          setEditingCategoryId(category.id);
                          setEditingName(category.name);
                        }}
                      >
                        <Pencil className="size-4" />
                      </Button>
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-sm"
                        disabled={isPending}
                        aria-label={`删除${category.name}`}
                        onClick={() => deleteCategory(category)}
                      >
                        <Trash2 className="size-4 text-destructive" />
                      </Button>
                    </>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function ConversationList({
  onSelectedConversationChange,
}: ConversationListProps) {
  const currentUserId = useAuthStore((state) => state.currentUserId ?? "");

  const deleteFriendMutation = useDeleteFriendMutation();
  const setGroupWholeMuteMutation = useSetGroupWholeMuteMutation();
  const renameGroupMutation = useRenameGroupMutation();
  const dissolveGroupMutation = useDissolveGroupMutation();
  const leaveGroupMutation = useLeaveGroupMutation();
  const setConversationPinnedMutation = useSetConversationPinnedMutation();
  const setConversationMutedMutation = useSetConversationMutedMutation();
  const createFriendCategoryMutation = useCreateFriendCategoryMutation();
  const createGroupCategoryMutation = useCreateGroupCategoryMutation();
  const deleteFriendCategoryMutation = useDeleteFriendCategoryMutation();
  const deleteGroupCategoryMutation = useDeleteGroupCategoryMutation();
  const renameFriendCategoryMutation = useRenameFriendCategoryMutation();
  const renameGroupCategoryMutation = useRenameGroupCategoryMutation();
  const setFriendCategoryMutation = useSetFriendCategoryMutation();
  const setGroupCategoryMutation = useSetGroupCategoryMutation();

  const [searchText, setSearchText] = useState("");
  const [activeView, setActiveView] =
    useState<ConversationListView>("messages");
  const [collapsedSections, setCollapsedSections] = useState<
    Record<string, boolean>
  >({});
  const [selectedConversationKey, setSelectedConversationKey] = useState<
    string | null
  >(null);
  const [activeDialog, setActiveDialog] = useState<
    "add-friend" | "create-group" | "request-manage" | null
  >(null);
  const [categoryDialog, setCategoryDialog] =
    useState<CategoryDialogState | null>(null);

  const usersQuery = useUsersQuery();
  const groupsQuery = useUserGroupsQuery(currentUserId);
  const friendshipsQuery = useFriendshipsQuery(currentUserId);
  const conversationStatesQuery = useConversationStatesQuery(currentUserId);
  const friendCategoriesQuery = useFriendCategoriesQuery(currentUserId);
  const groupCategoriesQuery = useGroupCategoriesQuery(currentUserId);

  const users = usersQuery.data ?? [];
  const groups = groupsQuery.data ?? [];
  const friendships = friendshipsQuery.data ?? [];
  const conversationStates = conversationStatesQuery.data ?? [];
  const friendCategories = friendCategoriesQuery.data ?? [];
  const groupCategories = groupCategoriesQuery.data ?? [];
  const defaultFriendCategoryId = currentUserId
    ? `${currentUserId}:friend:default`
    : "";
  const defaultGroupCategoryId = currentUserId
    ? `${currentUserId}:group:default`
    : "";

  // Build state lookup map keyed by "{scene}:{id}"
  const stateMap = useMemo(() => {
    const map: Record<string, { isPinned: boolean; isMuted: boolean }> = {};
    for (const state of conversationStates) {
      const key =
        state.conversation_scene === "private" && state.peer_user_id
          ? `private:${state.peer_user_id}`
          : `group:${state.group_id}`;
      map[key] = { isPinned: state.is_pinned, isMuted: state.is_muted };
    }
    return map;
  }, [conversationStates]);

  // Build group-to-category lookup from list_user_groups category_id
  const groupToCategoryMap = useMemo(() => {
    const map: Record<string, string | null> = {};
    for (const group of groups) {
      map[group.group_id] = group.category_id ?? null;
    }
    return map;
  }, [groups]);

  const friendToCategoryMap = useMemo(() => {
    const map: Record<string, string | null> = {};
    for (const friendship of friendships) {
      map[friendship.friend_user_id] = friendship.category_id ?? null;
    }
    return map;
  }, [friendships]);

  const friendCategoryOptions = useMemo(
    () =>
      buildCategoryOptions(
        friendCategories,
        defaultFriendCategoryId,
        DEFAULT_FRIEND_CATEGORY_NAME,
      ),
    [friendCategories, defaultFriendCategoryId],
  );

  const groupCategoryOptions = useMemo(
    () =>
      buildCategoryOptions(
        groupCategories,
        defaultGroupCategoryId,
        DEFAULT_GROUP_CATEGORY_NAME,
      ),
    [groupCategories, defaultGroupCategoryId],
  );

  const conversations = useMemo<ConversationItem[]>(() => {
    if (!currentUserId) {
      return [];
    }

    const friendIdSet = new Set(friendships.map((item) => item.friend_user_id));

    const privateConversations = users
      .filter(
        (user) =>
          user.user_id !== currentUserId && friendIdSet.has(user.user_id),
      )
      .map((user) => {
        const key = `private:${user.user_id}`;
        const state = stateMap[key];
        return {
          key: `private-${user.user_id}`,
          source: { scene: "private" as const, peer_user_id: user.user_id },
          title: user.nickname,
          avatarText: user.nickname.slice(0, 1).toUpperCase(),
          avatarUrl: user.avatar,
          isPinned: state?.isPinned ?? false,
          isMuted: state?.isMuted ?? false,
          categoryId: friendToCategoryMap[user.user_id] ?? null,
        };
      });

    const groupConversations = groups.map((group) => {
      const key = `group:${group.group_id}`;
      const state = stateMap[key];
      return {
        key: `group-${group.group_id}`,
        source: { scene: "group" as const, group_id: group.group_id },
        title: `${group.group_name} (${group.member_count})`,
        avatarText: "群",
        isPinned: state?.isPinned ?? false,
        isMuted: state?.isMuted ?? false,
        categoryId: groupToCategoryMap[group.group_id] ?? null,
      };
    });

    return [...privateConversations, ...groupConversations];
  }, [
    currentUserId,
    friendToCategoryMap,
    friendships,
    groups,
    users,
    stateMap,
    groupToCategoryMap,
  ]);

  const resolveMyGroupRole = async (
    groupId: string,
  ): Promise<GroupRole | null> => {
    const members = await invoke<GroupMemberProfile[]>(
      COMMANDS.listGroupMembers,
      {
        userId: currentUserId,
        groupId,
      },
    );
    return (
      members.find((member) => member.user_id === currentUserId)?.role ?? null
    );
  };

  const handleDeleteFriend = async (peerUserId: string) => {
    const confirmed = await confirmDialog({
      title: "确认删除好友",
      description: "确认删除该好友？",
      confirmText: "删除",
    });
    if (!confirmed) {
      return;
    }
    try {
      await deleteFriendMutation.mutateAsync({
        userId: currentUserId,
        friendUserId: peerUserId,
      });
      if (selectedConversationKey === `private-${peerUserId}`) {
        setSelectedConversationKey(null);
      }
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleSetWholeMute = async (groupId: string) => {
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
      toast.error("禁言时长必须为大于等于 0 的整数");
      return;
    }

    try {
      await setGroupWholeMuteMutation.mutateAsync({
        userId: currentUserId,
        groupId,
        durationSeconds: duration,
      });
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleRenameGroup = async (groupId: string) => {
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
      toast.error("群昵称不能为空");
      return;
    }

    try {
      await renameGroupMutation.mutateAsync({
        userId: currentUserId,
        groupId,
        groupName: name,
      });
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleDissolveGroup = async (groupId: string) => {
    const confirmed = await confirmDialog({
      title: "确认解散群聊",
      description: "确认解散该群聊？该操作不可恢复。",
      confirmText: "解散",
    });
    if (!confirmed) {
      return;
    }
    try {
      await dissolveGroupMutation.mutateAsync({
        userId: currentUserId,
        groupId,
      });
      setSelectedConversationKey(null);
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleLeaveGroup = async (groupId: string) => {
    const confirmed = await confirmDialog({
      title: "确认退出群聊",
      description: "确认退出该群聊？",
      confirmText: "退出",
    });
    if (!confirmed) {
      return;
    }

    try {
      await leaveGroupMutation.mutateAsync({
        userId: currentUserId,
        groupId,
      });
      setSelectedConversationKey(null);
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleTogglePinned = async (
    source: MessageSource,
    currentPinned: boolean,
  ) => {
    try {
      await setConversationPinnedMutation.mutateAsync({
        userId: currentUserId,
        scene: source.scene,
        peerUserId: source.scene === "private" ? source.peer_user_id : null,
        groupId: source.scene === "group" ? source.group_id : null,
        isPinned: !currentPinned,
      });
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleToggleMuted = async (
    source: MessageSource,
    currentMuted: boolean,
  ) => {
    try {
      await setConversationMutedMutation.mutateAsync({
        userId: currentUserId,
        scene: source.scene,
        peerUserId: source.scene === "private" ? source.peer_user_id : null,
        groupId: source.scene === "group" ? source.group_id : null,
        isMuted: !currentMuted,
      });
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleSetGroupCategory = async (
    groupId: string,
    categoryId: string,
  ) => {
    await setGroupCategoryMutation.mutateAsync({
      userId: currentUserId,
      groupId,
      categoryId,
    });
  };

  const handleSetFriendCategory = async (
    friendUserId: string,
    categoryId: string,
  ) => {
    await setFriendCategoryMutation.mutateAsync({
      userId: currentUserId,
      friendUserId,
      categoryId,
    });
  };

  const handleCreateFriendCategory = async (name: string) => {
    await createFriendCategoryMutation.mutateAsync({
      userId: currentUserId,
      name,
    });
  };

  const handleCreateGroupCategory = async (name: string) => {
    await createGroupCategoryMutation.mutateAsync({
      userId: currentUserId,
      name,
    });
  };

  const handleRenameFriendCategory = async (
    categoryId: string,
    name: string,
  ) => {
    await renameFriendCategoryMutation.mutateAsync({
      userId: currentUserId,
      categoryId,
      name,
    });
  };

  const handleRenameGroupCategory = async (
    categoryId: string,
    name: string,
  ) => {
    await renameGroupCategoryMutation.mutateAsync({
      userId: currentUserId,
      categoryId,
      name,
    });
  };

  const handleDeleteFriendCategory = async (categoryId: string) => {
    await deleteFriendCategoryMutation.mutateAsync({
      userId: currentUserId,
      categoryId,
    });
  };

  const handleDeleteGroupCategory = async (categoryId: string) => {
    await deleteGroupCategoryMutation.mutateAsync({
      userId: currentUserId,
      categoryId,
    });
  };

  const friendRequestsQuery = useFriendRequestsQuery(currentUserId, true);
  const groupRequestsQuery = useGroupRequestsQuery(currentUserId, true);
  const hasPendingRequests = useMemo(() => {
    const incomingPendingFriendRequests = (friendRequestsQuery.data ?? []).some(
      (request) =>
        request.state === "pending" && request.target_user_id === currentUserId,
    );
    const pendingGroupRequests = (groupRequestsQuery.data ?? []).length > 0;

    return incomingPendingFriendRequests || pendingGroupRequests;
  }, [friendRequestsQuery.data, groupRequestsQuery.data, currentUserId]);

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
            lastMessage: buildConversationPreview(
              latestMessage,
              users,
              currentUserId,
            ),
            lastAt: latestMessage?.created_at ?? 0,
          },
        ] as const;
      }),
    );
  }, [currentUserId, users, conversations, snapshotQueries]);

  const sortedConversations = useMemo(() => {
    return [...conversations].sort((a, b) => {
      if (a.isPinned !== b.isPinned) {
        return a.isPinned ? -1 : 1;
      }
      const timeA = snapshots[a.key]?.lastAt ?? 0;
      const timeB = snapshots[b.key]?.lastAt ?? 0;
      if (timeA !== timeB) {
        return timeB - timeA;
      }
      return a.title.localeCompare(b.title, "zh-CN");
    });
  }, [conversations, snapshots]);

  const visibleConversations = useMemo(() => {
    const keyword = searchText.trim().toLowerCase();
    if (!keyword) {
      return sortedConversations;
    }
    return sortedConversations.filter((conversation) =>
      conversation.title.toLowerCase().includes(keyword),
    );
  }, [sortedConversations, searchText]);

  const friendConversations = useMemo(
    () =>
      visibleConversations.filter(
        (conversation) => conversation.source.scene === "private",
      ),
    [visibleConversations],
  );

  const groupConversations = useMemo(
    () =>
      visibleConversations.filter(
        (conversation) => conversation.source.scene === "group",
      ),
    [visibleConversations],
  );

  const friendSections = useMemo<ConversationSection[]>(() => {
    return buildCategorySections(friendCategoryOptions, friendConversations, {
      keyPrefix: "friends",
      defaultCategoryId: defaultFriendCategoryId,
      defaultSectionKey: DEFAULT_FRIEND_SECTION_KEY,
      defaultSectionTitle: DEFAULT_FRIEND_CATEGORY_NAME,
    }).filter((section) => section.items.length > 0);
  }, [defaultFriendCategoryId, friendCategoryOptions, friendConversations]);

  const groupSections = useMemo<ConversationSection[]>(() => {
    return buildCategorySections(groupCategoryOptions, groupConversations, {
      keyPrefix: "groups",
      defaultCategoryId: defaultGroupCategoryId,
      defaultSectionKey: DEFAULT_GROUP_SECTION_KEY,
      defaultSectionTitle: DEFAULT_GROUP_CATEGORY_NAME,
    }).filter((section) => section.items.length > 0);
  }, [defaultGroupCategoryId, groupCategoryOptions, groupConversations]);

  const toggleSection = (key: string) => {
    setCollapsedSections((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const selectedConversation = useMemo(
    () => conversations.find((item) => item.key === selectedConversationKey),
    [selectedConversationKey, conversations],
  );

  useEffect(() => {
    onSelectedConversationChange(
      selectedConversation ? selectedConversation.source : null,
    );
  }, [onSelectedConversationChange, selectedConversation]);

  useEffect(() => {
    if (!selectedConversationKey) {
      return;
    }
    const exists = conversations.some(
      (conversation) => conversation.key === selectedConversationKey,
    );
    if (!exists) {
      setSelectedConversationKey(null);
    }
  }, [conversations, selectedConversationKey]);

  const renderConversationItem = (conversation: ConversationItem) => {
    const snapshot = snapshots[conversation.key];
    const privatePeerId =
      conversation.source.scene === "private"
        ? conversation.source.peer_user_id
        : null;
    const groupId =
      conversation.source.scene === "group"
        ? conversation.source.group_id
        : null;
    const itemClassName = `w-full rounded-md px-2.5 py-2 text-left transition-colors ${
      selectedConversation?.key === conversation.key
        ? "bg-foreground/25"
        : "hover:bg-foreground/5"
    }`;

    return (
      <ContextMenu key={conversation.key}>
        <ContextMenuTrigger asChild>
          <button
            type="button"
            className={itemClassName}
            onClick={() => setSelectedConversationKey(conversation.key)}
          >
            <div className="flex items-center gap-2">
              <Avatar className="size-8">
                <AvatarImage src={conversation.avatarUrl} />
                <AvatarFallback>{conversation.avatarText}</AvatarFallback>
              </Avatar>
              <div className="min-w-0 flex-1">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex min-w-0 items-center gap-1">
                    <p className="truncate font-medium text-sm">
                      {conversation.title}
                    </p>
                    {conversation.isMuted && (
                      <BellOff className="size-3 shrink-0 text-muted-foreground" />
                    )}
                  </div>
                  <span className="shrink-0 text-[11px] text-muted-foreground">
                    {formatConversationPreviewTime(snapshot?.lastAt ?? 0)}
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
          <ContextMenuItem
            onSelect={() =>
              handleTogglePinned(conversation.source, conversation.isPinned)
            }
          >
            {conversation.isPinned ? (
              <>
                <Pin className="mr-1.5 size-3.5 rotate-45" /> 取消置顶
              </>
            ) : (
              <>
                <Pin className="mr-1.5 size-3.5" /> 置顶会话
              </>
            )}
          </ContextMenuItem>

          <ContextMenuItem
            onSelect={() =>
              handleToggleMuted(conversation.source, conversation.isMuted)
            }
          >
            {conversation.isMuted ? (
              <>
                <Bell className="mr-1.5 size-3.5" /> 开启通知
              </>
            ) : (
              <>
                <BellOff className="mr-1.5 size-3.5" /> 免打扰
              </>
            )}
          </ContextMenuItem>

          <ContextMenuSeparator />

          {privatePeerId !== null ? (
            <>
              <ContextMenuItem
                onSelect={() =>
                  setCategoryDialog({
                    mode: "friend",
                    peerUserId: privatePeerId,
                    selectedCategoryId: conversation.categoryId ?? undefined,
                  })
                }
              >
                <FolderOpen className="mr-1.5 size-3.5" /> 分组
              </ContextMenuItem>

              <ContextMenuSeparator />
              <ContextMenuItem
                variant="destructive"
                onSelect={() => handleDeleteFriend(privatePeerId)}
              >
                删除好友
              </ContextMenuItem>
            </>
          ) : null}

          {groupId !== null ? (
            <>
              <ContextMenuItem
                onSelect={async () => {
                  const role = await resolveMyGroupRole(groupId);
                  if (role !== "owner" && role !== "admin") {
                    toast.error("仅群主或管理员可设置全体禁言");
                    return;
                  }
                  await handleSetWholeMute(groupId);
                }}
              >
                设置全体禁言
              </ContextMenuItem>
              <ContextMenuItem
                onSelect={async () => {
                  const role = await resolveMyGroupRole(groupId);
                  if (role !== "owner" && role !== "admin") {
                    toast.error("仅群主或管理员可修改群昵称");
                    return;
                  }
                  await handleRenameGroup(groupId);
                }}
              >
                修改群昵称
              </ContextMenuItem>

              <ContextMenuSeparator />
              <ContextMenuItem
                onSelect={() =>
                  setCategoryDialog({
                    mode: "group",
                    groupId,
                    selectedCategoryId: conversation.categoryId ?? undefined,
                  })
                }
              >
                <FolderOpen className="mr-1.5 size-3.5" /> 分组
              </ContextMenuItem>

              <ContextMenuSeparator />
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
            </>
          ) : null}
        </ContextMenuContent>
      </ContextMenu>
    );
  };

  const renderConversationGroup = (items: ConversationItem[]) => {
    const pinned = items.filter((conversation) => conversation.isPinned);
    const rest = items.filter((conversation) => !conversation.isPinned);
    return (
      <div className="space-y-1">
        {pinned.length > 0 ? (
          <div className="space-y-1 rounded-md bg-foreground/5 p-1">
            {pinned.map((conversation) => renderConversationItem(conversation))}
          </div>
        ) : null}
        {rest.map((conversation) => renderConversationItem(conversation))}
      </div>
    );
  };

  const renderSections = (sections: ConversationSection[]) => (
    <div className="space-y-2">
      {sections.map((section) => {
        const collapsed = collapsedSections[section.key] ?? false;
        const ChevronIcon = collapsed ? ChevronRight : ChevronDown;
        return (
          <section key={section.key} className="space-y-1">
            <button
              type="button"
              className="flex h-8 w-full items-center justify-between px-1.5 text-muted-foreground text-xs transition-colors hover:text-foreground"
              onClick={() => toggleSection(section.key)}
            >
              <span className="flex min-w-0 items-center gap-1.5">
                <ChevronIcon className="size-3.5 shrink-0" />
                <span className="truncate font-medium">{section.title}</span>
              </span>
              <span className="shrink-0">{section.items.length}</span>
            </button>
            {!collapsed ? renderConversationGroup(section.items) : null}
          </section>
        );
      })}
    </div>
  );

  return (
    <aside className="flex h-full flex-col bg-sidebar">
      <header className="flex items-center gap-2 border-b p-3">
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
            <DropdownMenuContent align="start">
              <DropdownMenuItem onSelect={() => setActiveDialog("add-friend")}>
                <UserPlus className="size-4" />
                添加好友
              </DropdownMenuItem>
              <DropdownMenuItem
                onSelect={() => setActiveDialog("create-group")}
              >
                <Users className="size-4" />
                创建群聊
              </DropdownMenuItem>
              <DropdownMenuItem
                onSelect={() => setActiveDialog("request-manage")}
              >
                <Check className="size-4" />
                请求管理
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </header>

      <div className="grid grid-cols-3 gap-1 border-b p-2">
        {(
          [
            ["messages", "消息"],
            ["friends", "好友"],
            ["groups", "群聊"],
          ] as const
        ).map(([view, label]) => (
          <button
            key={view}
            type="button"
            className={`h-7 rounded-md px-2 text-sm transition-colors ${
              activeView === view
                ? "bg-primary/15 font-medium text-primary"
                : "text-muted-foreground hover:bg-muted hover:text-foreground"
            }`}
            onClick={() => setActiveView(view)}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-auto p-2">
        {activeView === "messages" ? (
          renderConversationGroup(visibleConversations)
        ) : activeView === "friends" ? (
          <div className="space-y-2">
            <div className="flex items-center justify-between px-1">
              <span className="font-medium text-muted-foreground text-xs">
                好友分组
              </span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 px-2"
                onClick={() => setCategoryDialog({ mode: "friend" })}
              >
                <Plus className="size-3.5" />
                新建分组
              </Button>
            </div>
            {renderSections(friendSections)}
          </div>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center justify-between px-1">
              <span className="font-medium text-muted-foreground text-xs">
                群聊分组
              </span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-7 px-2"
                onClick={() => setCategoryDialog({ mode: "group" })}
              >
                <Plus className="size-3.5" />
                新建分组
              </Button>
            </div>
            {renderSections(groupSections)}
          </div>
        )}
      </div>

      <AddFriendDialog
        open={activeDialog === "add-friend"}
        onOpenChange={(open) => setActiveDialog(open ? "add-friend" : null)}
        users={users}
      />
      <CreateGroupDialog
        open={activeDialog === "create-group"}
        onOpenChange={(open) => setActiveDialog(open ? "create-group" : null)}
        users={users}
        groups={groups}
      />
      <RequestManageDialog
        open={activeDialog === "request-manage"}
        onOpenChange={(open) => setActiveDialog(open ? "request-manage" : null)}
        users={users}
        groups={groups}
      />
      {categoryDialog?.mode === "friend" ? (
        <CategoryManageDialog
          open
          title="好友分组"
          categories={friendCategoryOptions}
          selectedCategoryId={
            categoryDialog.peerUserId
              ? categoryDialog.selectedCategoryId
              : undefined
          }
          onOpenChange={(open) => {
            if (!open) {
              setCategoryDialog(null);
            }
          }}
          onCreate={handleCreateFriendCategory}
          onRename={handleRenameFriendCategory}
          onDelete={handleDeleteFriendCategory}
          onSelect={
            categoryDialog.peerUserId
              ? (categoryId) =>
                  handleSetFriendCategory(
                    categoryDialog.peerUserId ?? "",
                    categoryId,
                  )
              : undefined
          }
        />
      ) : null}
      {categoryDialog?.mode === "group" ? (
        <CategoryManageDialog
          open
          title="群聊分组"
          categories={groupCategoryOptions}
          selectedCategoryId={
            categoryDialog.groupId
              ? categoryDialog.selectedCategoryId
              : undefined
          }
          onOpenChange={(open) => {
            if (!open) {
              setCategoryDialog(null);
            }
          }}
          onCreate={handleCreateGroupCategory}
          onRename={handleRenameGroupCategory}
          onDelete={handleDeleteGroupCategory}
          onSelect={
            categoryDialog.groupId
              ? (categoryId) =>
                  handleSetGroupCategory(
                    categoryDialog.groupId ?? "",
                    categoryId,
                  )
              : undefined
          }
        />
      ) : null}
    </aside>
  );
}

export default ConversationList;
