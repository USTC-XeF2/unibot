import { invoke } from "@tauri-apps/api/core";
import { Check, Plus } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { UserProfile } from "@/types/user";

type RequestState = "pending" | "accepted" | "rejected" | "ignored";

type FriendRequestEntity = {
  request_id: number;
  initiator_user_id: number;
  target_user_id: number;
  comment: string;
  state: RequestState;
  created_at: number;
  handled_at: number | null;
  operator_user_id: number | null;
};

type AddFriendDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  currentUserId: number;
  users: UserProfile[];
};

function AddFriendDialog({
  open,
  onOpenChange,
  currentUserId,
  users,
}: AddFriendDialogProps) {
  const [friendRequests, setFriendRequests] = useState<FriendRequestEntity[]>(
    [],
  );
  const [sendingFriendRequestIds, setSendingFriendRequestIds] = useState<
    number[]
  >([]);
  const [actionError, setActionError] = useState<string | null>(null);

  const refreshFriendRequests = useCallback(async () => {
    if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
      return;
    }

    try {
      const rows = await invoke<FriendRequestEntity[]>("list_friend_requests", {
        userId: currentUserId,
      });
      setFriendRequests(rows);
    } catch {
      setFriendRequests([]);
    }
  }, [currentUserId]);

  useEffect(() => {
    if (!open) {
      return;
    }
    void refreshFriendRequests();
  }, [open, refreshFriendRequests]);

  const acceptedFriendIds = useMemo(() => {
    const set = new Set<number>();
    for (const request of friendRequests) {
      if (request.state !== "accepted") {
        continue;
      }
      if (request.initiator_user_id === currentUserId) {
        set.add(request.target_user_id);
      }
      if (request.target_user_id === currentUserId) {
        set.add(request.initiator_user_id);
      }
    }
    return set;
  }, [currentUserId, friendRequests]);

  const pendingFriendIds = useMemo(() => {
    const set = new Set<number>();
    for (const request of friendRequests) {
      if (request.state !== "pending") {
        continue;
      }
      if (request.initiator_user_id === currentUserId) {
        set.add(request.target_user_id);
      }
      if (request.target_user_id === currentUserId) {
        set.add(request.initiator_user_id);
      }
    }
    return set;
  }, [currentUserId, friendRequests]);

  const handleSendFriendRequest = async (targetUserId: number) => {
    if (sendingFriendRequestIds.includes(targetUserId)) {
      return;
    }

    setSendingFriendRequestIds((ids) => [...ids, targetUserId]);
    try {
      await invoke("create_friend_request", {
        userId: currentUserId,
        targetUserId,
        comment: "",
      });
      setActionError(null);
      await refreshFriendRequests();
    } catch (error) {
      setActionError(error as string);
    } finally {
      setSendingFriendRequestIds((ids) =>
        ids.filter((id) => id !== targetUserId),
      );
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        onOpenChange(nextOpen);
        if (!nextOpen) {
          setActionError(null);
        }
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>添加好友</DialogTitle>
        </DialogHeader>
        <div className="max-h-80 space-y-1.5 overflow-auto">
          {users.length <= 1 ? (
            <p className="px-2 py-7 text-center text-muted-foreground text-sm">
              没有可用用户
            </p>
          ) : (
            users
              .filter((user) => user.user_id !== currentUserId)
              .map((user) => {
                const pending = pendingFriendIds.has(user.user_id);
                const accepted = acceptedFriendIds.has(user.user_id);
                const sending = sendingFriendRequestIds.includes(user.user_id);
                return (
                  <div
                    key={user.user_id}
                    className="flex items-center gap-2.5 rounded-md px-3 py-2"
                  >
                    <Avatar className="size-8">
                      <AvatarImage src={user.avatar} />
                      <AvatarFallback>
                        {user.nickname.slice(0, 1).toUpperCase()}
                      </AvatarFallback>
                    </Avatar>
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm">{user.nickname}</p>
                    </div>
                    {accepted ? (
                      <span className="text-muted-foreground text-xs">
                        已是好友
                      </span>
                    ) : (
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-sm"
                        disabled={pending || sending}
                        onClick={() =>
                          void handleSendFriendRequest(user.user_id)
                        }
                      >
                        {pending ? (
                          <Check className="size-3.5" />
                        ) : (
                          <Plus className="size-3.5" />
                        )}
                      </Button>
                    )}
                  </div>
                );
              })
          )}
        </div>
        {actionError ? (
          <p className="rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1 text-destructive text-xs">
            {actionError}
          </p>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

export default AddFriendDialog;
