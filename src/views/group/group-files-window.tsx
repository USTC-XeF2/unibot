import { useSearchParams } from "react-router";
import { ChatEventBusProvider } from "@/components/chat/chat-event-bus-provider";
import GroupFileBrowser from "@/components/group/group-file-browser";
import { Toaster } from "@/components/ui/sonner";
import { isValidGroupId, isValidUserId } from "@/lib/query/common";

export default function GroupFilesWindow() {
  const [searchParams] = useSearchParams();
  const userId = searchParams.get("userId") || "";
  const groupId = searchParams.get("groupId") || "";

  if (!isValidUserId(userId) || !isValidGroupId(groupId)) {
    return (
      <div className="flex h-screen items-center justify-center text-muted-foreground">
        缺少有效的 userId 或 groupId
      </div>
    );
  }

  const windowLabel = `group-files-${userId}-${groupId}`;

  return (
    <ChatEventBusProvider userId={userId} windowLabel={windowLabel}>
      <main className="flex h-screen w-screen flex-col bg-background">
        <GroupFileBrowser userId={userId} groupId={groupId} />
        <Toaster position="top-center" />
      </main>
    </ChatEventBusProvider>
  );
}
