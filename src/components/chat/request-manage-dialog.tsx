import { useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  useHandleFriendRequestMutation,
  useHandleGroupRequestMutation,
} from "@/lib/mutations";
import { useFriendRequestsQuery, useGroupRequestsQuery } from "@/lib/query";
import { formatMonthDayTime } from "@/lib/time-format";
import { resolveUserDisplayName } from "@/lib/utils";
import { useAuthStore } from "@/store/use-auth-store";
import type { GroupProfile } from "@/types/group";
import type { UserProfile } from "@/types/user";

type RequestManageDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  users: UserProfile[];
  groups: GroupProfile[];
};

function RequestManageDialog({
  open,
  onOpenChange,
  users,
  groups,
}: RequestManageDialogProps) {
  const currentUserId = useAuthStore((state) => state.currentUserId ?? -1);
  const [requestError, setRequestError] = useState<string | null>(null);
  const [handlingRequestKeys, setHandlingRequestKeys] = useState<string[]>([]);

  const usersById = useMemo(
    () => new Map(users.map((user) => [user.user_id, user])),
    [users],
  );
  const groupNameById = useMemo(
    () =>
      new Map(groups.map((group) => [group.group_id, group.group_name || ""])),
    [groups],
  );

  const friendRequestsQuery = useFriendRequestsQuery(currentUserId, open);
  const pendingGroupRequestsQuery = useGroupRequestsQuery(currentUserId, open);

  const friendRequests = useMemo(
    () =>
      [...(friendRequestsQuery.data ?? [])].sort(
        (a, b) => b.created_at - a.created_at,
      ),
    [friendRequestsQuery.data],
  );
  const pendingGroupRequests = pendingGroupRequestsQuery.data ?? [];
  const handleFriendRequestMutation = useHandleFriendRequestMutation();
  const handleGroupRequestMutation = useHandleGroupRequestMutation();

  const handleFriendRequestAction = async (
    requestId: number,
    state: "accepted" | "rejected",
  ) => {
    const key = `friend-${requestId}`;
    if (handlingRequestKeys.includes(key)) {
      return;
    }

    if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
      setRequestError("当前用户无效，无法处理请求");
      return;
    }

    setHandlingRequestKeys((keys) => [...keys, key]);
    try {
      await handleFriendRequestMutation.mutateAsync({
        userId: currentUserId,
        requestId,
        state,
      });
      setRequestError(null);
    } catch (error) {
      setRequestError(String(error));
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

    if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
      setRequestError("当前用户无效，无法处理请求");
      return;
    }

    setHandlingRequestKeys((keys) => [...keys, key]);
    try {
      await handleGroupRequestMutation.mutateAsync({
        userId: currentUserId,
        requestId,
        state,
      });
      setRequestError(null);
    } catch (error) {
      setRequestError(String(error));
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
                  ? `你向 ${resolveUserDisplayName(request.target_user_id, target?.nickname)} 发送了好友请求`
                  : `${resolveUserDisplayName(request.initiator_user_id, sender?.nickname)} 请求添加你为好友`;

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
                          {formatMonthDayTime(request.created_at)}
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
                            handleFriendRequestAction(
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
                            handleFriendRequestAction(
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
                暂无待处理加群请求
              </p>
            ) : (
              pendingGroupRequests.map((request) => {
                const initiator = usersById.get(request.initiator_user_id);
                const handling = handlingRequestKeys.includes(
                  `group-${request.request_id}`,
                );
                const groupName =
                  groupNameById.get(request.group_id) ||
                  `群 ${request.group_id}`;

                const actionText =
                  request.request_type === "join" ||
                  request.target_user_id === null
                    ? `申请加入 ${groupName}`
                    : `邀请 ${resolveUserDisplayName(
                        request.target_user_id,
                        usersById.get(request.target_user_id)?.nickname,
                      )} 加入 ${groupName}`;

                return (
                  <div
                    key={request.request_id}
                    className="space-y-2 rounded-md border px-3 py-2"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-xs">
                        <span className="font-medium">
                          {resolveUserDisplayName(
                            request.initiator_user_id,
                            initiator?.nickname,
                          )}
                        </span>
                        <span className="text-muted-foreground">
                          {" "}
                          {actionText}
                        </span>
                      </p>
                      <span className="shrink-0 text-[11px] text-muted-foreground">
                        {formatMonthDayTime(request.created_at)}
                      </span>
                    </div>
                    <div className="flex items-center justify-end gap-2">
                      <Button
                        type="button"
                        size="xs"
                        variant="outline"
                        disabled={handling}
                        onClick={() =>
                          handleGroupRequestAction(
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
                          handleGroupRequestAction(
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
