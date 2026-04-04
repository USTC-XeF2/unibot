import { Check, Plus } from "lucide-react";
import { useMemo, useState } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useCreateFriendRequestMutation } from "@/lib/mutations";
import { useFriendRequestsQuery, useFriendsQuery } from "@/lib/query";
import { useAuthStore } from "@/store/use-auth-store";
import type { UserProfile } from "@/types/user";

type AddFriendDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  users: UserProfile[];
};

function AddFriendDialog({ open, onOpenChange, users }: AddFriendDialogProps) {
  const currentUserId = useAuthStore((state) => state.currentUserId ?? -1);
  const [sendingFriendRequestIds, setSendingFriendRequestIds] = useState<
    number[]
  >([]);
  const [actionError, setActionError] = useState<string | null>(null);
  const createFriendRequestMutation = useCreateFriendRequestMutation();
  const friendsQuery = useFriendsQuery(currentUserId);
  const friendIds = friendsQuery.data ?? [];
  const friendRequestsQuery = useFriendRequestsQuery(currentUserId, open);
  const friendRequests = friendRequestsQuery.data ?? [];

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

    if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
      setActionError("当前用户无效，无法发送好友请求");
      return;
    }

    setSendingFriendRequestIds((ids) => [...ids, targetUserId]);
    try {
      await createFriendRequestMutation.mutateAsync({
        userId: currentUserId,
        targetUserId,
        comment: "",
      });
      setActionError(null);
    } catch (error) {
      setActionError(String(error));
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
                    {friendIds.includes(user.user_id) ? (
                      <span className="text-muted-foreground text-xs">
                        已是好友
                      </span>
                    ) : (
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-sm"
                        disabled={pending || sending}
                        onClick={() => handleSendFriendRequest(user.user_id)}
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
