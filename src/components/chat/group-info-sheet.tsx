import { Bell, LogOut, Pin, Users } from "lucide-react";
import { useCallback } from "react";
import { toast } from "sonner";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldSeparator,
} from "@/components/ui/field";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Switch } from "@/components/ui/switch";
import { confirmDialog } from "@/lib/modal";
import {
  useDissolveGroupMutation,
  useLeaveGroupMutation,
  useSetConversationMutedMutation,
  useSetConversationPinnedMutation,
} from "@/lib/mutations";
import { useUsersQuery } from "@/lib/query";
import { resolveUserDisplayName } from "@/lib/utils";
import type { GroupMemberProfile } from "@/types/group";

type GroupInfoSheetProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  groupId: string;
  groupName: string;
  memberCount: number;
  members: GroupMemberProfile[];
  currentUserId: string;
  isPinned: boolean;
  isMuted: boolean;
};

export default function GroupInfoSheet({
  open,
  onOpenChange,
  groupId,
  groupName,
  memberCount,
  members,
  currentUserId,
  isPinned,
  isMuted,
}: GroupInfoSheetProps) {
  const usersQuery = useUsersQuery();
  const users = usersQuery.data ?? [];

  const setPinnedMutation = useSetConversationPinnedMutation();
  const setMutedMutation = useSetConversationMutedMutation();
  const leaveGroupMutation = useLeaveGroupMutation();
  const dissolveGroupMutation = useDissolveGroupMutation();

  const isOwner =
    members.find((m) => m.user_id === currentUserId)?.role === "owner";

  const handleTogglePinned = useCallback(
    (checked: boolean) => {
      setPinnedMutation.mutate({
        userId: currentUserId,
        scene: "group",
        peerUserId: null,
        groupId,
        isPinned: checked,
      });
    },
    [currentUserId, groupId, setPinnedMutation],
  );

  const handleToggleMuted = useCallback(
    (checked: boolean) => {
      setMutedMutation.mutate({
        userId: currentUserId,
        scene: "group",
        peerUserId: null,
        groupId,
        isMuted: checked,
      });
    },
    [currentUserId, groupId, setMutedMutation],
  );

  const handleLeave = async () => {
    const confirmed = await confirmDialog({
      title: isOwner ? "确认解散群聊" : "确认退出群聊",
      description: isOwner
        ? "确认解散该群聊？该操作不可恢复。"
        : "确认退出该群聊？",
      confirmText: isOwner ? "解散" : "退出",
    });
    if (!confirmed) return;
    try {
      if (isOwner) {
        await dissolveGroupMutation.mutateAsync({
          userId: currentUserId,
          groupId,
        });
      } else {
        await leaveGroupMutation.mutateAsync({
          userId: currentUserId,
          groupId,
        });
      }
      onOpenChange(false);
    } catch (error) {
      toast.error(String(error));
    }
  };

  const displayMembers = members.slice(0, 15);
  const hasMore = members.length > 15;

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>群聊信息</SheetTitle>
        </SheetHeader>

        <FieldGroup className="px-4">
          {/* 群名称 */}
          <Field>
            <div className="flex items-center gap-3 py-2">
              <Avatar className="size-12">
                <AvatarFallback className="bg-primary/10 text-lg">
                  {groupName.slice(0, 1)}
                </AvatarFallback>
              </Avatar>
              <div>
                <p className="font-medium">{groupName}</p>
                <p className="text-muted-foreground text-sm">
                  {memberCount} 名成员
                </p>
              </div>
            </div>
          </Field>

          <FieldSeparator />

          {/* 成员网格 */}
          <Field>
            <div className="flex items-center justify-between">
              <FieldLabel>群聊成员</FieldLabel>
              <span className="text-muted-foreground text-sm">
                查看 {memberCount} 名成员
              </span>
            </div>
            <div className="grid grid-cols-5 gap-3 pt-2">
              {displayMembers.map((member) => {
                const user = users.find((u) => u.user_id === member.user_id);
                const name = resolveUserDisplayName(
                  member.user_id,
                  user?.nickname,
                  Object.fromEntries(members.map((m) => [m.user_id, m])),
                );
                return (
                  <div
                    key={member.user_id}
                    className="flex flex-col items-center gap-1"
                  >
                    <Avatar className="size-10">
                      <AvatarImage src={user?.avatar} />
                      <AvatarFallback className="text-xs">
                        {name.slice(0, 1)}
                      </AvatarFallback>
                    </Avatar>
                    <span className="max-w-full truncate text-muted-foreground text-xs">
                      {name}
                    </span>
                  </div>
                );
              })}
              {hasMore && (
                <div className="flex flex-col items-center gap-1">
                  <div className="flex size-10 items-center justify-center rounded-full bg-muted">
                    <Users className="size-4 text-muted-foreground" />
                  </div>
                  <span className="text-muted-foreground text-xs">更多</span>
                </div>
              )}
            </div>
          </Field>

          <FieldSeparator />

          {/* 置顶 / 免打扰 */}
          <Field orientation="horizontal">
            <FieldLabel className="flex items-center gap-2">
              <Pin className="size-4" />
              设为置顶
            </FieldLabel>
            <Switch checked={isPinned} onCheckedChange={handleTogglePinned} />
          </Field>

          <Field orientation="horizontal">
            <FieldLabel className="flex items-center gap-2">
              <Bell className="size-4" />
              消息免打扰
            </FieldLabel>
            <Switch checked={isMuted} onCheckedChange={handleToggleMuted} />
          </Field>

          <FieldSeparator />

          {/* 退出/解散 */}
          <Field>
            <Button
              variant="ghost"
              className="w-full justify-start gap-2 text-destructive hover:bg-destructive/10 hover:text-destructive"
              onClick={handleLeave}
            >
              <LogOut className="size-4" />
              {isOwner ? "解散群聊" : "退出群聊"}
            </Button>
          </Field>
        </FieldGroup>
      </SheetContent>
    </Sheet>
  );
}
