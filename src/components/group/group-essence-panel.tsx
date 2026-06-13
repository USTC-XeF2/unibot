import { Star } from "lucide-react";
import { useGroupEssenceMessagesQuery } from "@/lib/query";

export default function GroupEssencePanel({
  userId,
  groupId,
}: {
  userId: string;
  groupId: string;
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
            <div key={essence.essence_id} className="rounded-lg border p-3">
              <div className="flex items-start gap-2">
                <Star className="mt-0.5 size-4 text-yellow-500" />
                <div className="flex-1">
                  <p className="font-medium text-sm">
                    {essence.sender_user_id}
                  </p>
                  <p className="text-muted-foreground text-sm">
                    消息 ID: {essence.message_id}
                  </p>
                  <p className="mt-1 text-muted-foreground text-xs">
                    {new Date(essence.created_at).toLocaleString()}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
