import { Star, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { segmentsToPlainText } from "@/lib/message-content";
import { confirmDialog } from "@/lib/modal";
import { useSetGroupEssenceMessageMutation } from "@/lib/mutations";
import { useGroupEssenceMessagesQuery } from "@/lib/query";
import type { GroupEssenceMessage } from "@/types/group";

export default function GroupEssencePanel({
  userId,
  groupId,
  canManage,
}: {
  userId: string;
  groupId: string;
  canManage: boolean;
}) {
  const { data: essenceMessages = [] } = useGroupEssenceMessagesQuery(
    userId,
    groupId,
  );

  return (
    <div className="flex h-full flex-col">
      <div className="border-b p-3">
        <h2 className="font-semibold">精华消息</h2>
      </div>

      <div className="flex-1 overflow-auto p-3">
        {essenceMessages.length === 0 && (
          <div className="py-8 text-center text-muted-foreground text-sm">
            暂无精华消息
          </div>
        )}
        <div className="space-y-3">
          {essenceMessages.map((essence) => (
            <EssenceItem
              key={essence.essence_id}
              userId={userId}
              groupId={groupId}
              essence={essence}
              canManage={canManage}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function EssenceItem({
  userId,
  groupId,
  essence,
  canManage,
}: {
  userId: string;
  groupId: string;
  essence: GroupEssenceMessage;
  canManage: boolean;
}) {
  const unsetMutation = useSetGroupEssenceMessageMutation();

  const handleUnset = async () => {
    const confirmed = await confirmDialog({
      title: "确认取消精华",
      description: "确定要取消这条精华消息吗？",
      confirmText: "取消精华",
    });
    if (!confirmed) return;

    unsetMutation.mutate({
      userId,
      groupId,
      messageId: essence.message_id,
      isSet: false,
    });
  };

  const preview = segmentsToPlainText(essence.content);

  return (
    <div className="rounded-lg border p-3">
      <div className="flex items-start gap-2">
        <Star className="mt-0.5 size-4 text-yellow-500" />
        <div className="min-w-0 flex-1">
          <p className="font-medium text-sm">{essence.sender_user_id}</p>
          <p className="mt-1 line-clamp-3 text-sm">{preview || "[无内容]"}</p>
          <div className="mt-2 flex items-center justify-between">
            <div className="text-muted-foreground text-xs">
              <p>设置者：{essence.operator_user_id}</p>
              <p>{new Date(essence.created_at).toLocaleString()}</p>
            </div>
            {canManage && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleUnset}
                disabled={unsetMutation.isPending}
              >
                <X className="mr-1 size-4" />
                取消精华
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
