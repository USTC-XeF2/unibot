import { useSearchParams } from "react-router";
import { ChatEventBusProvider } from "@/components/chat/chat-event-bus-provider";
import GroupFileBrowser from "@/components/group/group-file-browser";
import { Toaster } from "@/components/ui/sonner";

export default function GroupFilesWindow() {
  const [searchParams] = useSearchParams();
  const userId = searchParams.get("userId") || "";
  const groupId = searchParams.get("groupId") || "";
  const windowLabel = `group-files-${userId}-${groupId}`;

  if (!userId || !groupId) {
    return <div>缺少 userId 或 groupId</div>;
  }

  return (
    <ChatEventBusProvider userId={userId} windowLabel={windowLabel}>
      <main className="flex h-screen w-screen flex-col bg-background">
        <GroupFileBrowser userId={userId} groupId={groupId} />
        <Toaster position="top-center" />
      </main>
    </ChatEventBusProvider>
  );
}
