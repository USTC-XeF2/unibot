import { Check } from "lucide-react";
import { useMemo, useState } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useCreateGroupMutation } from "@/lib/mutations";
import { useAuthStore } from "@/store/use-auth-store";
import type { GroupProfile } from "@/types/group";
import type { UserProfile } from "@/types/user";

type CreateGroupDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  users: UserProfile[];
  groups: GroupProfile[];
};

function CreateGroupDialog({
  open,
  onOpenChange,
  users,
  groups,
}: CreateGroupDialogProps) {
  const currentUserId = useAuthStore((state) => state.currentUserId ?? -1);
  const [selectedGroupMemberIds, setSelectedGroupMemberIds] = useState<
    number[]
  >([]);
  const [groupIdInput, setGroupIdInput] = useState("");
  const [groupNameInput, setGroupNameInput] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);
  const createGroupMutation = useCreateGroupMutation();
  const creatingGroup = createGroupMutation.isPending;

  const selectableUsers = useMemo(
    () => users.filter((user) => user.user_id !== currentUserId),
    [currentUserId, users],
  );

  const toggleGroupMember = (userId: number) => {
    setSelectedGroupMemberIds((ids) => {
      if (ids.includes(userId)) {
        return ids.filter((id) => id !== userId);
      }
      return [...ids, userId];
    });
  };

  const handleCreateGroup = async () => {
    if (selectedGroupMemberIds.length === 0 || creatingGroup) {
      return;
    }

    if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
      setActionError("当前用户无效，无法创建群聊");
      return;
    }

    const parsedGroupId = Number(groupIdInput.trim());
    if (!Number.isInteger(parsedGroupId) || parsedGroupId <= 0) {
      setActionError("群聊 ID 必须是大于 0 的整数");
      return;
    }
    const parsedGroupName = groupNameInput.trim();
    if (parsedGroupName.length === 0) {
      setActionError("群名称不能为空");
      return;
    }
    if (groups.some((group) => group.group_id === parsedGroupId)) {
      setActionError(`群聊 ID ${parsedGroupId} 已存在`);
      return;
    }

    try {
      const initialMemberUserIds = [...selectedGroupMemberIds].sort(
        (a, b) => a - b,
      );

      await createGroupMutation.mutateAsync({
        userId: currentUserId,
        groupId: parsedGroupId,
        groupName: parsedGroupName,
        initialMemberUserIds,
      });

      setGroupIdInput("");
      setGroupNameInput("");
      setSelectedGroupMemberIds([]);
      setActionError(null);
      onOpenChange(false);
    } catch (error) {
      setActionError(String(error));
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        onOpenChange(nextOpen);
        if (!nextOpen) {
          setActionError(null);
          setGroupIdInput("");
          setGroupNameInput("");
          setSelectedGroupMemberIds([]);
        }
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>创建群聊</DialogTitle>
        </DialogHeader>
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="create-group-id">群聊 ID</FieldLabel>
            <Input
              id="create-group-id"
              value={groupIdInput}
              onChange={(event) => {
                setGroupIdInput(event.target.value);
                if (actionError) {
                  setActionError(null);
                }
              }}
              placeholder="请输入群聊 ID"
              inputMode="numeric"
            />
          </Field>
          <Field>
            <FieldLabel htmlFor="create-group-name">群名称</FieldLabel>
            <Input
              id="create-group-name"
              value={groupNameInput}
              onChange={(event) => {
                setGroupNameInput(event.target.value);
                if (actionError) {
                  setActionError(null);
                }
              }}
              placeholder="请输入群名称"
              autoComplete="off"
            />
          </Field>
        </FieldGroup>
        <div className="max-h-72 space-y-1.5 overflow-auto">
          {selectableUsers.length === 0 ? (
            <p className="px-2 py-7 text-center text-muted-foreground text-sm">
              没有可选用户
            </p>
          ) : (
            selectableUsers.map((user) => {
              const selected = selectedGroupMemberIds.includes(user.user_id);
              return (
                <button
                  key={user.user_id}
                  type="button"
                  className="flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-left hover:bg-muted/60"
                  onClick={() => toggleGroupMember(user.user_id)}
                >
                  <Avatar className="size-8">
                    <AvatarImage src={user.avatar} />
                    <AvatarFallback>
                      {user.nickname.slice(0, 1).toUpperCase()}
                    </AvatarFallback>
                  </Avatar>
                  <span className="min-w-0 flex-1 truncate text-sm">
                    {user.nickname}
                  </span>
                  {selected ? (
                    <Check className="size-3.5 text-primary" />
                  ) : null}
                </button>
              );
            })
          )}
        </div>
        <Button
          type="button"
          size="default"
          className="w-full"
          disabled={
            selectedGroupMemberIds.length === 0 ||
            creatingGroup ||
            groupIdInput.trim().length === 0 ||
            groupNameInput.trim().length === 0
          }
          onClick={() => handleCreateGroup()}
        >
          {creatingGroup
            ? "创建中..."
            : `创建群聊 (${selectedGroupMemberIds.length})`}
        </Button>
        {actionError ? (
          <p className="rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1 text-destructive text-xs">
            {actionError}
          </p>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

export default CreateGroupDialog;
