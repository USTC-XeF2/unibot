import { useMutation } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { invalidateGroupsQuery } from "@/lib/groups-query";
import {
  invalidateFriendRequestsQuery,
  invalidateManageableGroupRequestsQueries,
  useFriendRequestsQuery,
  useManageableGroupRequestsQuery,
} from "@/lib/request-query";
import type { InternalEventPayload } from "@/types/chat";
import type { GroupProfile } from "@/types/group";
import type { UserProfile } from "@/types/user";

type RequestManageDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  currentUserId: number;
  users: UserProfile[];
  groups: GroupProfile[];
};

function formatRequestTime(ts: number): string {
  if (!ts) {
    return "";
  }
  return new Date(ts * 1000).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function RequestManageDialog({
  open,
  onOpenChange,
  currentUserId,
  users,
  groups,
}: RequestManageDialogProps) {
  const [requestError, setRequestError] = useState<string | null>(null);
  const [handlingRequestKeys, setHandlingRequestKeys] = useState<string[]>([]);

  const usersById = useMemo(
    () => new Map(users.map((user) => [user.user_id, user])),
    [users],
  );

  const friendRequestsQuery = useFriendRequestsQuery(currentUserId, open);
  const pendingGroupRequestsQuery = useManageableGroupRequestsQuery(
    currentUserId,
    groups,
    open,
  );

  const friendRequests = useMemo(
    () =>
      [...(friendRequestsQuery.data ?? [])].sort(
        (a, b) => b.created_at - a.created_at,
      ),
    [friendRequestsQuery.data],
  );
  const pendingGroupRequests = pendingGroupRequestsQuery.data ?? [];

  useEffect(() => {
    if (!open || !Number.isInteger(currentUserId) || currentUserId <= 0) {
      return;
    }

    let disposed = false;
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      unlisten = await listen<InternalEventPayload>("chat:event", (event) => {
        if (disposed || !event.payload) {
          return;
        }

        const kind = event.payload.kind;
        if (
          kind === "friend_request_created" ||
          kind === "friend_request_handled" ||
          kind === "group_request_created" ||
          kind === "group_request_handled"
        ) {
          void invalidateFriendRequestsQuery(currentUserId);
          void invalidateManageableGroupRequestsQueries(currentUserId);
        }
      });
    };

    void setup();

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [open, currentUserId]);

  const handleFriendRequestMutation = useMutation({
    mutationFn: (payload: {
      requestId: number;
      state: "accepted" | "rejected";
    }) =>
      invoke("handle_friend_request", {
        userId: currentUserId,
        requestId: payload.requestId,
        state: payload.state,
      }),
  });

  const handleGroupRequestMutation = useMutation({
    mutationFn: (payload: {
      requestId: number;
      state: "accepted" | "rejected";
    }) =>
      invoke("handle_group_request", {
        userId: currentUserId,
        requestId: payload.requestId,
        state: payload.state,
      }),
  });

  const handleFriendRequestAction = async (
    requestId: number,
    state: "accepted" | "rejected",
  ) => {
    const key = `friend-${requestId}`;
    if (handlingRequestKeys.includes(key)) {
      return;
    }

    setHandlingRequestKeys((keys) => [...keys, key]);
    try {
      await handleFriendRequestMutation.mutateAsync({ requestId, state });
      setRequestError(null);
      await invalidateFriendRequestsQuery(currentUserId);
      await invalidateManageableGroupRequestsQueries(currentUserId);
    } catch (error) {
      setRequestError(error as string);
    } finally {
      setHandlingRequestKeys((keys) => keys.filter((item) => item !== key));
    }
  };

  const handleGroupRequestAction = async (
    requestId: number,
    state: "accepted" | "rejected",
  ) => {
    const key = `group-${requestId}`;
    if (handlingRequestKeys.includes(key)) {
      return;
    }

    setHandlingRequestKeys((keys) => [...keys, key]);
    try {
      await handleGroupRequestMutation.mutateAsync({ requestId, state });
      setRequestError(null);
      await invalidateGroupsQuery();
      await invalidateManageableGroupRequestsQueries(currentUserId);
    } catch (error) {
      setRequestError(error as string);
    } finally {
      setHandlingRequestKeys((keys) => keys.filter((item) => item !== key));
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        onOpenChange(nextOpen);
        if (!nextOpen) {
          setRequestError(null);
        }
      }}
    >
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>请求管理</DialogTitle>
        </DialogHeader>

        <div className="max-h-96 space-y-4 overflow-auto pr-1">
          <section className="space-y-2">
            <h3 className="font-medium text-sm">好友请求</h3>
            {friendRequestsQuery.error ? (
              <p className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-4 text-center text-destructive text-xs">
                {String(friendRequestsQuery.error)}
              </p>
            ) : null}
            {friendRequests.length === 0 ? (
              <p className="rounded-md border border-dashed px-3 py-4 text-center text-muted-foreground text-xs">
                {friendRequestsQuery.isPending
                  ? "正在加载好友请求..."
                  : "暂无好友请求记录"}
              </p>
            ) : (
              friendRequests.map((request) => {
                const sender = usersById.get(request.initiator_user_id);
                const target = usersById.get(request.target_user_id);
                const isOutgoing = request.initiator_user_id === currentUserId;
                const isActionableIncomingPending =
                  !isOutgoing && request.state === "pending";
                const handling = handlingRequestKeys.includes(
                  `friend-${request.request_id}`,
                );

                const statusText =
                  request.state === "accepted"
                    ? "已同意"
                    : request.state === "rejected"
                      ? "已拒绝"
                      : request.state === "ignored"
                        ? "已忽略"
                        : isOutgoing
                          ? "等待对方处理"
                          : "待你处理";

                const actionText = isOutgoing
                  ? `你向 ${target?.nickname || `用户 ${request.target_user_id}`} 发送了好友请求`
                  : `${sender?.nickname || `用户 ${request.initiator_user_id}`} 请求添加你为好友`;

                return (
                  <div
                    key={request.request_id}
                    className="space-y-2 rounded-md border px-3 py-2"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-xs">
                        <span className="text-muted-foreground">
                          {actionText}
                        </span>
                      </p>
                      <div className="flex shrink-0 items-center gap-2">
                        <span
                          className={`rounded border px-1.5 py-0.5 text-[11px] ${
                            request.state === "accepted"
                              ? "border-emerald-500/40 text-emerald-600"
                              : request.state === "rejected"
                                ? "border-destructive/40 text-destructive"
                                : "border-amber-500/40 text-amber-600"
                          }`}
                        >
                          {statusText}
                        </span>
                        <span className="text-[11px] text-muted-foreground">
                          {formatRequestTime(request.created_at)}
                        </span>
                      </div>
                    </div>
                    {isActionableIncomingPending ? (
                      <div className="flex items-center justify-end gap-2">
                        <Button
                          type="button"
                          size="xs"
                          variant="outline"
                          disabled={handling}
                          onClick={() =>
                            void handleFriendRequestAction(
                              request.request_id,
                              "rejected",
                            )
                          }
                        >
                          拒绝
                        </Button>
                        <Button
                          type="button"
                          size="xs"
                          disabled={handling}
                          onClick={() =>
                            void handleFriendRequestAction(
                              request.request_id,
                              "accepted",
                            )
                          }
                        >
                          同意
                        </Button>
                      </div>
                    ) : null}
                  </div>
                );
              })
            )}
          </section>

          <section className="space-y-2">
            <h3 className="font-medium text-sm">加群请求</h3>
            {pendingGroupRequestsQuery.error ? (
              <p className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-4 text-center text-destructive text-xs">
                {String(pendingGroupRequestsQuery.error)}
              </p>
            ) : null}
            {pendingGroupRequests.length === 0 ? (
              <p className="rounded-md border border-dashed px-3 py-4 text-center text-muted-foreground text-xs">
                {pendingGroupRequestsQuery.isPending
                  ? "正在加载加群请求..."
                  : "暂无待处理加群请求"}
              </p>
            ) : (
              pendingGroupRequests.map((request) => {
                const initiator = usersById.get(request.initiator_user_id);
                const target =
                  request.target_user_id === null
                    ? null
                    : usersById.get(request.target_user_id);
                const handling = handlingRequestKeys.includes(
                  `group-${request.request_id}`,
                );
                const actionText =
                  request.request_type === "join"
                    ? `申请加入 ${request.group_name}`
                    : `邀请 ${
                        target?.nickname ||
                        (request.target_user_id === null
                          ? "指定用户"
                          : `用户 ${request.target_user_id}`)
                      } 加入 ${request.group_name}`;

                return (
                  <div
                    key={request.request_id}
                    className="space-y-2 rounded-md border px-3 py-2"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-xs">
                        <span className="font-medium">
                          {initiator?.nickname ||
                            `用户 ${request.initiator_user_id}`}
                        </span>
                        <span className="text-muted-foreground">
                          {" "}
                          {actionText}
                        </span>
                      </p>
                      <span className="shrink-0 text-[11px] text-muted-foreground">
                        {formatRequestTime(request.created_at)}
                      </span>
                    </div>
                    <div className="flex items-center justify-end gap-2">
                      <Button
                        type="button"
                        size="xs"
                        variant="outline"
                        disabled={handling}
                        onClick={() =>
                          void handleGroupRequestAction(
                            request.request_id,
                            "rejected",
                          )
                        }
                      >
                        拒绝
                      </Button>
                      <Button
                        type="button"
                        size="xs"
                        disabled={handling}
                        onClick={() =>
                          void handleGroupRequestAction(
                            request.request_id,
                            "accepted",
                          )
                        }
                      >
                        同意
                      </Button>
                    </div>
                  </div>
                );
              })
            )}
          </section>
        </div>

        {requestError ? (
          <p className="rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1 text-destructive text-xs">
            {requestError}
          </p>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

export default RequestManageDialog;
