import { Clock, Crown, Shield, ShieldCheck, UserX } from "lucide-react";
import { toast } from "sonner";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { confirmDialog, promptDialog } from "@/lib/modal";
import {
  useKickGroupMemberMutation,
  useMuteGroupMemberMutation,
  useSetGroupMemberRoleMutation,
  useSetGroupMemberTitleMutation,
} from "@/lib/mutations";
import { useUsersQuery } from "@/lib/query";
import { resolveUserDisplayName } from "@/lib/utils";
import type { GroupMemberProfile, GroupRole } from "@/types/group";

type GroupMembersPanelProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  groupId: string;
  groupName: string;
  members: GroupMemberProfile[];
  currentUserId: string;
  myRole: GroupRole | null;
};

function RoleBadge({ role }: { role: GroupRole }) {
  if (role === "owner") {
    return (
      <span className="inline-flex items-center gap-0.5 rounded-full bg-amber-100 px-1.5 py-0.5 text-[10px] font-medium text-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
        <Crown className="size-2.5" /> 群主
      </span>
    );
  }
  if (role === "admin") {
    return (
      <span className="inline-flex items-center gap-0.5 rounded-full bg-blue-100 px-1.5 py-0.5 text-[10px] font-medium text-blue-700 dark:bg-blue-900/30 dark:text-blue-400">
        <ShieldCheck className="size-2.5" /> 管理
      </span>
    );
  }
  return null;
}

export default function GroupMembersPanel({
  open,
  onOpenChange,
  groupId,
  groupName,
  members,
  currentUserId,
  myRole,
}: GroupMembersPanelProps) {
  const usersQuery = useUsersQuery();
  const users = usersQuery.data ?? [];

  const muteMutation = useMuteGroupMemberMutation();
  const kickMutation = useKickGroupMemberMutation();
  const setRoleMutation = useSetGroupMemberRoleMutation();
  const setTitleMutation = useSetGroupMemberTitleMutation();

  const isOwner = myRole === "owner";
  const isAdmin = myRole === "admin";
  const canManage = isOwner || isAdmin;

  const sortedMembers = [...members].sort((a, b) => {
    const roleOrder = { owner: 0, admin: 1, member: 2 };
    if (roleOrder[a.role] !== roleOrder[b.role]) {
      return roleOrder[a.role] - roleOrder[b.role];
    }
    return a.joined_at - b.joined_at;
  });

  const handleMute = async (targetUserId: string) => {
    const input = await promptDialog({
      title: "设置禁言",
      description: "请输入禁言时长（秒，0为解除）",
      confirmText: "确定",
    });
    if (input === null) return;
    const duration = Number(input.trim());
    if (!Number.isInteger(duration) || duration < 0) {
      toast.error("禁言时长必须为大于等于 0 的整数");
      return;
    }
    try {
      await muteMutation.mutateAsync({
        userId: currentUserId,
        groupId,
        targetUserId,
        durationSeconds: duration,
      });
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleKick = async (targetUserId: string) => {
    const confirmed = await confirmDialog({
      title: "确认踢出成员",
      description: "确认踢出该成员？",
      confirmText: "踢出",
    });
    if (!confirmed) return;
    try {
      await kickMutation.mutateAsync({
        userId: currentUserId,
        groupId,
        targetUserId,
      });
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleToggleAdmin = async (
    targetUserId: string,
    makeAdmin: boolean,
  ) => {
    try {
      await setRoleMutation.mutateAsync({
        userId: currentUserId,
        groupId,
        targetUserId,
        isAdmin: makeAdmin,
      });
    } catch (error) {
      toast.error(String(error));
    }
  };

  const handleSetTitle = async (targetUserId: string) => {
    const title = await promptDialog({
      title: "设置头衔",
      description: "请输入头衔（可留空清除）",
      confirmText: "保存",
    });
    if (title === null) return;
    try {
      await setTitleMutation.mutateAsync({
        userId: currentUserId,
        groupId,
        targetUserId,
        title: title.trim(),
      });
    } catch (error) {
      toast.error(String(error));
    }
  };

  const canManageTarget = (target: GroupMemberProfile) => {
    if (!canManage) return false;
    if (target.user_id === currentUserId) return false;
    if (isOwner) return target.role !== "owner";
    if (isAdmin) return target.role === "member";
    return false;
  };

  const canToggleAdmin = (target: GroupMemberProfile) => {
    return (
      isOwner && target.user_id !== currentUserId && target.role !== "owner"
    );
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="w-80 sm:w-96">
        <SheetHeader>
          <SheetTitle className="text-base">
            {groupName}
            <span className="ml-2 text-muted-foreground text-sm">
              ({members.length}人)
            </span>
          </SheetTitle>
        </SheetHeader>

        <div className="mt-4 space-y-1">
          {sortedMembers.map((member) => {
            const user = users.find((u) => u.user_id === member.user_id);
            const displayName = resolveUserDisplayName(
              member.user_id,
              user?.nickname,
              Object.fromEntries(members.map((m) => [m.user_id, m])),
            );
            const cardName =
              member.card && member.card !== displayName ? member.card : null;
            const manageable = canManageTarget(member);
            const toggleAdminable = canToggleAdmin(member);

            return (
              <div
                key={member.user_id}
                className="group flex items-center gap-2 rounded-lg px-2 py-1.5 hover:bg-muted/50"
              >
                <Avatar className="size-8">
                  <AvatarImage src={user?.avatar} />
                  <AvatarFallback>
                    {displayName.slice(0, 1).toUpperCase()}
                  </AvatarFallback>
                </Avatar>

                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1">
                    <span className="truncate text-sm font-medium">
                      {displayName}
                    </span>
                    <RoleBadge role={member.role} />
                  </div>
                  {cardName && (
                    <p className="truncate text-[11px] text-muted-foreground">
                      {cardName}
                    </p>
                  )}
                  {member.title && (
                    <p className="text-[11px] text-amber-600">{member.title}</p>
                  )}
                </div>

                {(manageable || toggleAdminable) && (
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        className="size-7 opacity-0 group-hover:opacity-100 hover:opacity-100"
                        onPointerDown={(e) => e.stopPropagation()}
                      >
                        <Shield className="size-3.5 text-muted-foreground" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      {manageable && (
                        <>
                          <DropdownMenuItem
                            onSelect={() => handleMute(member.user_id)}
                          >
                            <Clock className="size-3.5 mr-1.5" /> 禁言
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            variant="destructive"
                            onSelect={() => handleKick(member.user_id)}
                          >
                            <UserX className="size-3.5 mr-1.5" /> 踢出
                          </DropdownMenuItem>
                        </>
                      )}
                      {toggleAdminable && (
                        <DropdownMenuItem
                          onSelect={() =>
                            handleToggleAdmin(
                              member.user_id,
                              member.role !== "admin",
                            )
                          }
                        >
                          <ShieldCheck className="size-3.5 mr-1.5" />
                          {member.role === "admin" ? "取消管理" : "设为管理"}
                        </DropdownMenuItem>
                      )}
                      {isOwner && (
                        <DropdownMenuItem
                          onSelect={() => handleSetTitle(member.user_id)}
                        >
                          <Crown className="size-3.5 mr-1.5" /> 设置头衔
                        </DropdownMenuItem>
                      )}
                    </DropdownMenuContent>
                  </DropdownMenu>
                )}
              </div>
            );
          })}
        </div>
      </SheetContent>
    </Sheet>
  );
}
