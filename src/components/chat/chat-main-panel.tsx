import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { File, Image, Send, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import ChatComposer, {
  type ChatComposerHandle,
  type MentionableUser,
} from "@/components/chat/chat-composer";
import ChatMessageItem, {
  type ChatContextAction,
} from "@/components/chat/chat-message-item";
import FacePicker from "@/components/chat/face-picker";
import { Button } from "@/components/ui/button";
import {
  invalidateMessageHistoryQuery,
  invalidatePokeHistoryQuery,
  sourceFromInternalEvent,
  useMessageHistoryQuery,
  usePokeHistoryQuery,
} from "@/lib/chat-query";
import type { FaceDefinition } from "@/lib/face-library";
import {
  invalidateGroupEventHistoryQuery,
  useGroupEventHistoryQuery,
} from "@/lib/groups-query";
import { messageSegmentsToPlainText } from "@/lib/message-content";
import { confirmDialog, promptDialog } from "@/lib/modal";
import type {
  ChatMessage,
  ChatPoke,
  GroupEvent,
  InternalEventPayload,
  MessageSegment,
  MessageSource,
} from "@/types/chat";
import type { GroupMemberProfile, GroupRole } from "@/types/group";
import type { UserProfile } from "@/types/user";

type ConversationSummary = {
  title: string;
  type: "private" | "group";
  source: MessageSource;
};

type ChatMainPanelProps = {
  selectedConversation: ConversationSummary;
  currentUserId: number;
  users: UserProfile[];
};

type ChatTimelineItem =
  | {
      kind: "message";
      key: string;
      createdAt: number;
      senderUserId: number;
      message: ChatMessage;
    }
  | {
      kind: "poke";
      key: string;
      createdAt: number;
      senderUserId: number;
      poke: ChatPoke;
    }
  | {
      kind: "group_event";
      key: string;
      createdAt: number;
      groupEvent: GroupEvent;
    };

function formatNoticeUntilTime(ts: number): string {
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
      second: "2-digit",
    });
  }

  return date.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function hasSendableSegments(segments: MessageSegment[]): boolean {
  return segments.some((segment) => {
    if (segment.type === "Text") {
      return segment.data.text.trim().length > 0;
    }

    return true;
  });
}

function ChatMainPanel({
  selectedConversation,
  currentUserId,
  users,
}: ChatMainPanelProps) {
  const messageListRef = useRef<HTMLDivElement | null>(null);
  const composerRef = useRef<ChatComposerHandle | null>(null);
  const shouldScrollToBottomRef = useRef(false);
  const stickToBottomRef = useRef(true);
  const lastConversationKeyRef = useRef<string | null>(null);
  const [sendLoading, setSendLoading] = useState(false);
  const [composerSegments, setComposerSegments] = useState<MessageSegment[]>(
    [],
  );
  const [quotedMessage, setQuotedMessage] = useState<{
    messageId: number;
    summary: string;
  } | null>(null);
  const [chatError, setChatError] = useState<string | null>(null);
  const [groupMemberCards, setGroupMemberCards] = useState<
    Record<number, string>
  >({});
  const [groupMembersById, setGroupMembersById] = useState<
    Record<number, GroupMemberProfile>
  >({});
  const [currentUnixTime, setCurrentUnixTime] = useState(
    Math.floor(Date.now() / 1000),
  );

  const messagesQuery = useMessageHistoryQuery(
    currentUserId,
    selectedConversation.source,
    100,
  );
  const pokesQuery = usePokeHistoryQuery(
    currentUserId,
    selectedConversation.source,
    100,
  );
  const groupEventsQuery = useGroupEventHistoryQuery(
    currentUserId,
    selectedConversation.source.scene === "group"
      ? selectedConversation.source.group_id
      : 0,
    100,
    selectedConversation.source.scene === "group",
  );
  const messages = useMemo<ChatMessage[]>(() => {
    const history = messagesQuery.data ?? [];
    return [...history].reverse();
  }, [messagesQuery.data]);
  const pokes = useMemo<ChatPoke[]>(() => {
    const history = pokesQuery.data ?? [];
    return [...history].reverse();
  }, [pokesQuery.data]);
  const groupEvents = useMemo<GroupEvent[]>(() => {
    const history = groupEventsQuery.data ?? [];
    return [...history].reverse();
  }, [groupEventsQuery.data]);
  const timeline = useMemo<ChatTimelineItem[]>(() => {
    const items: ChatTimelineItem[] = [
      ...messages.map((message) => ({
        kind: "message" as const,
        key: `message-${message.id}-${message.created_at}`,
        createdAt: message.created_at,
        senderUserId: message.sender_user_id,
        message,
      })),
      ...pokes.map((poke) => ({
        kind: "poke" as const,
        key: `poke-${poke.poke_id}-${poke.created_at}`,
        createdAt: poke.created_at,
        senderUserId: poke.sender_user_id,
        poke,
      })),
      ...groupEvents.map((groupEvent) => ({
        kind: "group_event" as const,
        key: `group-event-${groupEvent.event_id}-${groupEvent.created_at}`,
        createdAt: groupEvent.created_at,
        groupEvent,
      })),
    ];

    items.sort((a, b) => {
      if (a.createdAt !== b.createdAt) {
        return a.createdAt - b.createdAt;
      }
      return a.key.localeCompare(b.key);
    });

    return items;
  }, [groupEvents, messages, pokes]);
  const queryError = messagesQuery.error
    ? String(messagesQuery.error)
    : pokesQuery.error
      ? String(pokesQuery.error)
      : groupEventsQuery.error
        ? String(groupEventsQuery.error)
        : null;

  const conversationKey =
    selectedConversation.source.scene === "group"
      ? `group-${selectedConversation.source.group_id}`
      : `private-${selectedConversation.source.peer_user_id}`;
  const selectedSource = selectedConversation.source;

  if (lastConversationKeyRef.current !== conversationKey) {
    lastConversationKeyRef.current = conversationKey;
    shouldScrollToBottomRef.current = true;
  }

  const updateStickToBottomState = () => {
    const list = messageListRef.current;
    if (!list) {
      return;
    }

    const distance = list.scrollHeight - list.scrollTop - list.clientHeight;
    stickToBottomRef.current = distance <= 20;
  };

  useEffect(() => {
    if (timeline.length === 0) {
      return;
    }

    const list = messageListRef.current;
    if (!list) {
      return;
    }

    if (!shouldScrollToBottomRef.current && !stickToBottomRef.current) {
      return;
    }

    requestAnimationFrame(() => {
      list.scrollTop = list.scrollHeight;
      shouldScrollToBottomRef.current = false;
      stickToBottomRef.current = true;
    });
  }, [timeline.length]);

  useEffect(() => {
    if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
      return;
    }

    let disposed = false;
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      unlisten = await listen<InternalEventPayload>("chat:event", (event) => {
        if (disposed || !event.payload) {
          return;
        }
        const payload = event.payload;

        const source = sourceFromInternalEvent(payload, currentUserId);
        if (!source) {
          return;
        }

        if (source.scene === "group") {
          if (
            selectedSource.scene !== "group" ||
            source.group_id !== selectedSource.group_id
          ) {
            return;
          }
        } else if (
          selectedSource.scene !== "private" ||
          source.peer_user_id !== selectedSource.peer_user_id
        ) {
          return;
        }

        if (stickToBottomRef.current) {
          shouldScrollToBottomRef.current = true;
        }

        if (payload.kind === "group_member_muted") {
          setGroupMembersById((previous) => {
            const target = previous[payload.target_user_id];
            if (!target) {
              return previous;
            }
            return {
              ...previous,
              [payload.target_user_id]: {
                ...target,
                mute_until: payload.mute_until,
              },
            };
          });
        }

        if (
          selectedSource.scene === "group" &&
          ((payload.kind === "notice" &&
            payload.notice_type === "admin_change") ||
            payload.kind === "group_member_title_updated" ||
            payload.kind === "group_member_joined")
        ) {
          void (async () => {
            try {
              const members = await invoke<GroupMemberProfile[]>(
                "list_group_members",
                {
                  userId: currentUserId,
                  groupId: selectedSource.group_id,
                },
              );
              if (disposed) {
                return;
              }

              setGroupMembersById(
                Object.fromEntries(
                  members.map((member) => [member.user_id, member]),
                ),
              );
              setGroupMemberCards(
                Object.fromEntries(
                  members.map((member) => [
                    member.user_id,
                    member.card?.trim() ?? "",
                  ]),
                ),
              );
            } catch {
              // Ignore refresh errors; existing snapshot remains usable.
            }
          })();
        }

        invalidateMessageHistoryQuery(currentUserId, selectedSource);
        invalidatePokeHistoryQuery(currentUserId, selectedSource);
        if (selectedSource.scene === "group") {
          invalidateGroupEventHistoryQuery(
            currentUserId,
            selectedSource.group_id,
          );
        }
      });
    };

    setup();

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [currentUserId, selectedSource]);

  useEffect(() => {
    if (selectedConversation.source.scene !== "group") {
      setGroupMemberCards({});
      setGroupMembersById({});
      return;
    }

    const groupId = selectedConversation.source.group_id;

    let disposed = false;
    const fetchMembers = async () => {
      try {
        const members = await invoke<GroupMemberProfile[]>(
          "list_group_members",
          {
            userId: currentUserId,
            groupId,
          },
        );
        if (disposed) {
          return;
        }

        setGroupMembersById(
          Object.fromEntries(members.map((member) => [member.user_id, member])),
        );
        setGroupMemberCards(
          Object.fromEntries(
            members.map((member) => [
              member.user_id,
              member.card?.trim() ?? "",
            ]),
          ),
        );
      } catch {
        if (!disposed) {
          setGroupMembersById({});
          setGroupMemberCards({});
        }
      }
    };

    fetchMembers();
    return () => {
      disposed = true;
    };
  }, [currentUserId, selectedConversation]);

  useEffect(() => {
    if (!conversationKey) {
      return;
    }

    setQuotedMessage(null);
  }, [conversationKey]);

  const myGroupRole: GroupRole | null = useMemo(() => {
    if (selectedConversation.source.scene !== "group") {
      return null;
    }
    return groupMembersById[currentUserId]?.role ?? null;
  }, [currentUserId, groupMembersById, selectedConversation]);

  const currentUserMuteUntil = useMemo(() => {
    if (selectedConversation.source.scene !== "group") {
      return null;
    }
    const muteUntil = groupMembersById[currentUserId]?.mute_until;
    return typeof muteUntil === "number" ? muteUntil : null;
  }, [currentUserId, groupMembersById, selectedConversation]);

  useEffect(() => {
    if (!currentUserMuteUntil || currentUserMuteUntil <= currentUnixTime) {
      return;
    }

    const remainingMs = Math.max(
      0,
      (currentUserMuteUntil - currentUnixTime) * 1000 + 50,
    );
    const timer = window.setTimeout(() => {
      setCurrentUnixTime(Math.floor(Date.now() / 1000));
    }, remainingMs);

    return () => {
      window.clearTimeout(timer);
    };
  }, [currentUnixTime, currentUserMuteUntil]);

  const isCurrentUserMuted = useMemo(() => {
    if (selectedConversation.source.scene !== "group") {
      return false;
    }
    return !!currentUserMuteUntil && currentUserMuteUntil > currentUnixTime;
  }, [currentUnixTime, currentUserMuteUntil, selectedConversation]);

  const mentionableMembers = useMemo<MentionableUser[]>(() => {
    if (selectedSource.scene !== "group") return [];
    return Object.values(groupMembersById)
      .filter((m) => m.user_id !== currentUserId)
      .map((member) => {
        const user = users.find((u) => u.user_id === member.user_id);
        const name =
          groupMemberCards[member.user_id] ||
          user?.nickname ||
          `用户 ${member.user_id}`;
        return {
          id: member.user_id,
          name,
          avatar: user?.avatar,
        };
      });
  }, [
    selectedSource.scene,
    groupMembersById,
    users,
    groupMemberCards,
    currentUserId,
  ]);

  const handleSelectFace = (face: FaceDefinition) => {
    composerRef.current?.insertFace(face);
  };

  const handleSendMessage = async () => {
    if (sendLoading) {
      return;
    }

    if (isCurrentUserMuted) {
      return;
    }

    if (!hasSendableSegments(composerSegments)) {
      return;
    }

    setSendLoading(true);
    try {
      await invoke("send_message", {
        userId: currentUserId,
        source: selectedConversation.source,
        content: composerSegments,
        quoteMessageId: quotedMessage?.messageId ?? null,
      });
      setComposerSegments([]);
      setQuotedMessage(null);
      shouldScrollToBottomRef.current = true;
      await invalidateMessageHistoryQuery(
        currentUserId,
        selectedConversation.source,
      );
      setChatError(null);
    } catch (error) {
      setChatError(error as string);
    } finally {
      setSendLoading(false);
    }
  };

  const handleCopyMessage = async (text: string) => {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      }
      setChatError(null);
    } catch {
      setChatError("复制失败，请检查剪贴板权限");
    }
  };

  const handleQuoteMessage = (
    message: ChatMessage,
    senderDisplayName: string,
  ) => {
    const normalizedText = messageSegmentsToPlainText(message.content)
      .trim()
      .replace(/\s+/g, " ");
    const summary = `${senderDisplayName}: ${normalizedText || "[空消息]"}`;

    setQuotedMessage({
      messageId: message.id,
      summary,
    });
    requestAnimationFrame(() => {
      composerRef.current?.focus();

      if (
        selectedConversation.source.scene === "group" &&
        message.sender_user_id !== currentUserId
      ) {
        composerRef.current?.insertMention(message.sender_user_id);
      }
    });
  };

  const handleRecallMessage = async (messageId: number) => {
    try {
      await invoke("recall_message", {
        userId: currentUserId,
        messageId,
      });
      setChatError(null);
      await invalidateMessageHistoryQuery(
        currentUserId,
        selectedConversation.source,
      );
    } catch (error) {
      setChatError(error as string);
    }
  };

  const handlePokeUser = async (targetUserId: number) => {
    try {
      await invoke("poke_user", {
        userId: currentUserId,
        source: selectedConversation.source,
        targetUserId,
      });
      await invalidatePokeHistoryQuery(
        currentUserId,
        selectedConversation.source,
      );
      setChatError(null);
    } catch (error) {
      setChatError(error as string);
    }
  };

  const handleAtUser = (targetUserId: number) => {
    if (selectedConversation.source.scene !== "group") {
      return;
    }

    composerRef.current?.insertMention(targetUserId);
  };

  const refreshGroupMembers = async () => {
    if (selectedConversation.source.scene !== "group") {
      return;
    }
    const groupId = selectedConversation.source.group_id;
    const members = await invoke<GroupMemberProfile[]>("list_group_members", {
      userId: currentUserId,
      groupId,
    });
    setGroupMembersById(
      Object.fromEntries(members.map((member) => [member.user_id, member])),
    );
    setGroupMemberCards(
      Object.fromEntries(
        members.map((member) => [member.user_id, member.card?.trim() ?? ""]),
      ),
    );
  };

  const handleMuteMember = async (
    targetUserId: number,
    durationSeconds: number,
  ) => {
    if (selectedConversation.source.scene !== "group") {
      return;
    }
    try {
      await invoke("mute_group_member", {
        userId: currentUserId,
        groupId: selectedConversation.source.group_id,
        targetUserId,
        durationSeconds,
      });
      setChatError(null);
    } catch (error) {
      setChatError(error as string);
    }
  };

  const handleKickMember = async (targetUserId: number) => {
    if (selectedConversation.source.scene !== "group") {
      return;
    }
    const confirmed = await confirmDialog({
      title: "确认踢出成员",
      description: "确认踢出该成员？",
      confirmText: "踢出",
    });
    if (!confirmed) {
      return;
    }

    try {
      await invoke("kick_group_member", {
        userId: currentUserId,
        groupId: selectedConversation.source.group_id,
        targetUserId,
      });
      await refreshGroupMembers();
      setChatError(null);
    } catch (error) {
      setChatError(error as string);
    }
  };

  const handleToggleAdmin = async (
    targetUserId: number,
    makeAdmin: boolean,
  ) => {
    if (selectedConversation.source.scene !== "group") {
      return;
    }
    try {
      await invoke("set_group_member_role", {
        userId: currentUserId,
        groupId: selectedConversation.source.group_id,
        targetUserId,
        isAdmin: makeAdmin,
      });
      await refreshGroupMembers();
      setChatError(null);
    } catch (error) {
      setChatError(error as string);
    }
  };

  const handleSetTitle = async (targetUserId: number, title: string) => {
    if (selectedConversation.source.scene !== "group") {
      return;
    }
    try {
      await invoke("set_group_member_title", {
        userId: currentUserId,
        groupId: selectedConversation.source.group_id,
        targetUserId,
        title,
      });
      await refreshGroupMembers();
      setChatError(null);
    } catch (error) {
      setChatError(error as string);
    }
  };

  return (
    <section className="flex min-w-0 flex-1 flex-col">
      <header className="flex items-center border-b px-4 py-2">
        <p className="font-semibold text-sm">{selectedConversation.title}</p>
      </header>

      <div
        ref={messageListRef}
        onScroll={updateStickToBottomState}
        className="min-h-0 flex-1 space-y-2 overflow-auto bg-background/60 px-4 py-3"
      >
        {timeline.map((item) => {
          if (item.kind === "group_event") {
            const resolveName = (userId: number) => {
              const user = users.find((entry) => entry.user_id === userId);
              if (selectedConversation.source.scene === "group") {
                return (
                  groupMemberCards[userId] || user?.nickname || `用户${userId}`
                );
              }
              if (userId === currentUserId) {
                return "你";
              }
              return user?.nickname || `用户${userId}`;
            };

            const { payload } = item.groupEvent;
            let eventText = "";

            if (payload.type === "member_joined") {
              const joinedUserId = payload.joined_user_id;
              eventText = joinedUserId
                ? `${resolveName(joinedUserId)} 加入了群聊`
                : "有新成员加入了群聊";
            }

            if (payload.type === "member_muted") {
              const operatorUserId = payload.operator_user_id;
              const targetUserId = payload.target_user_id;
              const muteUntil = payload.mute_until;
              const operatorName = operatorUserId
                ? resolveName(operatorUserId)
                : "管理员";
              const targetName = targetUserId
                ? resolveName(targetUserId)
                : "成员";
              const muted = muteUntil !== null && muteUntil > item.createdAt;
              const durationSeconds = muteUntil
                ? Math.max(0, muteUntil - item.createdAt)
                : 0;
              const durationText =
                durationSeconds > 0
                  ? `${durationSeconds}秒`
                  : muteUntil
                    ? `至 ${formatNoticeUntilTime(muteUntil)}`
                    : "";
              eventText = muted
                ? `${targetName} 被 ${operatorName} 禁言 ${durationText}`.trim()
                : `${targetName} 被 ${operatorName} 取消禁言`;
            }

            if (payload.type === "essence_set") {
              const senderUserId = payload.sender_user_id;
              eventText = `${resolveName(senderUserId)} 的消息被设为了精华消息`;
            }

            if (!eventText) {
              return null;
            }

            return (
              <p
                key={item.key}
                className="my-1 text-center text-[11px] text-muted-foreground"
              >
                {eventText}
              </p>
            );
          }

          const senderUserId = item.senderUserId;
          const sender = users.find((user) => user.user_id === senderUserId);
          if (!sender) {
            return null;
          }

          if (item.kind === "poke") {
            const target = users.find(
              (user) => user.user_id === item.poke.target_user_id,
            );
            const senderDisplayName =
              selectedConversation.source.scene === "group"
                ? groupMemberCards[item.poke.sender_user_id] || sender.nickname
                : sender.nickname;
            const targetDisplayName =
              selectedConversation.source.scene === "group"
                ? groupMemberCards[item.poke.target_user_id] ||
                  target?.nickname ||
                  `用户${item.poke.target_user_id}`
                : target?.nickname ||
                  (item.poke.target_user_id === currentUserId ? "你" : "对方");
            const pokeText = `${senderDisplayName} 戳了戳 ${targetDisplayName}`;

            return (
              <p
                key={item.key}
                className="my-1 text-center text-[11px] text-muted-foreground"
              >
                {pokeText}
              </p>
            );
          }

          const message = item.message;

          const senderDisplayName =
            selectedConversation.source.scene === "group"
              ? groupMemberCards[message.sender_user_id] || sender.nickname
              : sender.nickname;

          const quotedMessagePreview = (() => {
            const quoteMessageId = message.quote_message_id;
            if (!quoteMessageId) {
              return null;
            }

            const quotedMessage = messages.find(
              (msg) => msg.id === quoteMessageId,
            );
            if (!quotedMessage) {
              return {
                senderDisplayName: "引用",
                summary: "",
                missing: true,
              };
            }

            const quotedSender = users.find(
              (user) => user.user_id === quotedMessage.sender_user_id,
            );
            const quotedSenderDisplayName =
              selectedConversation.source.scene === "group"
                ? groupMemberCards[quotedMessage.sender_user_id] ||
                  quotedSender?.nickname ||
                  `用户${quotedMessage.sender_user_id}`
                : quotedSender?.nickname ||
                  (quotedMessage.sender_user_id === currentUserId
                    ? "你"
                    : "对方");

            if (quotedMessage.recall.recalled) {
              return {
                senderDisplayName: quotedSenderDisplayName,
                summary: "[该消息已撤回]",
              };
            }

            return {
              senderDisplayName: quotedSenderDisplayName,
              summary: messageSegmentsToPlainText(quotedMessage.content),
            };
          })();
          const senderGroupProfile = groupMembersById[message.sender_user_id];
          const targetRole = groupMembersById[message.sender_user_id]?.role;
          const canManageTargetByAdmin =
            targetRole === "member" || targetRole === undefined;
          const canManageTargetByOwner = targetRole !== "owner";

          const canRecall =
            !message.recall.recalled &&
            (message.sender_user_id === currentUserId ||
              (item.kind === "message" &&
                selectedConversation.source.scene === "group" &&
                (myGroupRole === "owner" || myGroupRole === "admin")));
          const canAt =
            selectedConversation.source.scene === "group" &&
            message.sender_user_id !== currentUserId;
          const canMute =
            selectedConversation.source.scene === "group" &&
            message.sender_user_id !== currentUserId &&
            ((myGroupRole === "owner" && canManageTargetByOwner) ||
              (myGroupRole === "admin" && canManageTargetByAdmin));
          const canKick = canMute;
          const canToggleAdmin =
            selectedConversation.source.scene === "group" &&
            myGroupRole === "owner" &&
            message.sender_user_id !== currentUserId &&
            targetRole !== "owner";
          const canSetTitle =
            selectedConversation.source.scene === "group" &&
            myGroupRole === "owner" &&
            (targetRole !== "owner" ||
              message.sender_user_id === currentUserId);

          const avatarActions: ChatContextAction[] = [
            {
              key: "poke",
              label: "戳一戳",
              onSelect: () => handlePokeUser(message.sender_user_id),
            },
          ];

          if (canAt) {
            avatarActions.push({
              key: "at",
              label: "@ TA",
              onSelect: () => handleAtUser(message.sender_user_id),
            });
          }

          if (canMute) {
            avatarActions.push({
              key: "mute",
              label: "设置禁言",
              separatorBefore: true,
              onSelect: async () => {
                const input = await promptDialog({
                  title: "设置禁言",
                  description: "请输入禁言时长（秒，0为解除）",
                  confirmText: "确定",
                });
                if (input === null) {
                  return;
                }
                const duration = Number(input.trim());
                if (!Number.isInteger(duration) || duration < 0) {
                  setChatError("禁言时长必须为大于等于 0 的整数");
                  return;
                }
                await handleMuteMember(message.sender_user_id, duration);
              },
            });
          }

          if (canKick) {
            avatarActions.push({
              key: "kick",
              label: "踢出群聊",
              variant: "destructive",
              onSelect: () => handleKickMember(message.sender_user_id),
            });
          }

          if (canToggleAdmin) {
            avatarActions.push({
              key: "toggle-admin",
              label:
                groupMembersById[message.sender_user_id]?.role === "admin"
                  ? "取消管理员"
                  : "设为管理员",
              onSelect: () =>
                handleToggleAdmin(
                  message.sender_user_id,
                  groupMembersById[message.sender_user_id]?.role !== "admin",
                ),
            });
          }

          if (canSetTitle) {
            avatarActions.push({
              key: "set-title",
              label: "设置头衔",
              onSelect: async () => {
                const title = await promptDialog({
                  title: "设置头衔",
                  description: "请输入头衔（可留空清除）",
                  confirmText: "保存",
                });
                if (title === null) {
                  return;
                }
                await handleSetTitle(message.sender_user_id, title.trim());
              },
            });
          }

          const messageActions: ChatContextAction[] = [
            {
              key: "copy",
              label: "复制",
              onSelect: () =>
                handleCopyMessage(messageSegmentsToPlainText(message.content)),
            },
            {
              key: "quote",
              label: "引用",
              onSelect: () => handleQuoteMessage(message, senderDisplayName),
            },
          ];

          if (canRecall) {
            messageActions.push({
              key: "recall",
              label: "撤回",
              variant: "destructive",
              separatorBefore: true,
              onSelect: () => handleRecallMessage(message.id),
            });
          }

          return (
            <ChatMessageItem
              key={item.key}
              isSelf={message.sender_user_id === currentUserId}
              avatarUrl={sender.avatar}
              avatarFallback={sender.nickname.slice(0, 1).toUpperCase()}
              senderDisplayName={senderDisplayName}
              senderRole={senderGroupProfile?.role}
              senderTitle={senderGroupProfile?.title}
              showSenderName={selectedConversation.source.scene === "group"}
              message={message}
              quotedMessagePreview={quotedMessagePreview}
              onAtClick={(t) => composerRef.current?.insertMention(t)}
              resolveMemberName={(id) =>
                mentionableMembers.find((m) => m.id === id)?.name ?? String(id)
              }
              avatarActions={avatarActions}
              messageActions={messageActions}
            />
          );
        })}
      </div>

      <footer className="space-y-1 border-t p-2">
        {chatError ? (
          <p className="rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1 text-destructive text-xs">
            {chatError}
          </p>
        ) : queryError ? (
          <p className="rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1 text-destructive text-xs">
            {queryError}
          </p>
        ) : null}

        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1">
            <FacePicker onSelectFace={handleSelectFace} />
            <Button type="button" variant="ghost" size="icon-sm" title="图片">
              <Image className="size-4" />
            </Button>
            <Button type="button" variant="ghost" size="icon-sm" title="文件">
              <File className="size-4" />
            </Button>
          </div>

          <div className="min-w-0 flex-1">
            {quotedMessage ? (
              <div className="flex items-center gap-2 rounded-md border border-border/80 bg-muted/40 px-2 py-1 text-xs">
                <span className="shrink-0 text-muted-foreground">引用</span>
                <span className="min-w-0 flex-1 truncate text-foreground/90">
                  {quotedMessage.summary}
                </span>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  className="size-5"
                  title="取消引用"
                  onClick={() => setQuotedMessage(null)}
                >
                  <X className="size-3.5" />
                </Button>
              </div>
            ) : null}
          </div>

          <Button
            type="button"
            className="h-8 shrink-0 gap-1.5"
            disabled={
              sendLoading ||
              isCurrentUserMuted ||
              !hasSendableSegments(composerSegments)
            }
            onClick={() => handleSendMessage()}
          >
            <Send className="size-4" /> {isCurrentUserMuted ? "禁言中" : "发送"}
          </Button>
        </div>

        <ChatComposer
          ref={composerRef}
          segments={composerSegments}
          onSegmentsChange={setComposerSegments}
          mentionSupport={
            selectedConversation.source.scene === "group"
              ? myGroupRole === "owner" || myGroupRole === "admin"
                ? "all"
                : "user"
              : "none"
          }
          mentionableMembers={mentionableMembers}
          placeholder="输入消息，Enter 发送，Shift+Enter 换行"
          disabled={sendLoading}
          onSubmit={handleSendMessage}
        />
      </footer>
    </section>
  );
}

export default ChatMainPanel;
