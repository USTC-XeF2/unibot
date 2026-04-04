import { useEffect, useState } from "react";
import { useParams } from "react-router";
import ChatMainPanel from "@/components/chat/chat-main-panel";
import ConversationList from "@/components/chat/conversation-list";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { Toaster } from "@/components/ui/sonner";
import { isValidUserId } from "@/lib/query";
import { useAuthStore } from "@/store/use-auth-store";
import type { MessageSource } from "@/types/chat";

function ChatWindowView() {
  const { userId } = useParams();
  const currentUserId = useAuthStore((state) => state.currentUserId ?? -1);
  const setCurrentUserId = useAuthStore((state) => state.setCurrentUserId);
  const [selectedConversation, setSelectedConversation] =
    useState<MessageSource | null>(null);

  useEffect(() => {
    const userIdNum = Number(userId);
    const normalizedUserId = isValidUserId(userIdNum) ? userIdNum : null;

    setCurrentUserId(normalizedUserId);
    setSelectedConversation(null);

    return () => {
      setCurrentUserId(null);
    };
  }, [userId, setCurrentUserId]);

  if (!Number.isInteger(currentUserId) || currentUserId <= 0) {
    return null;
  }

  return (
    <main className="flex h-screen w-screen overflow-hidden bg-background">
      <ResizablePanelGroup orientation="horizontal" className="flex-1">
        <ResizablePanel defaultSize={240} minSize={200} maxSize={280}>
          <ConversationList
            onSelectedConversationChange={setSelectedConversation}
          />
        </ResizablePanel>

        <ResizableHandle />

        <ResizablePanel>
          {selectedConversation ? (
            <ChatMainPanel conversation={selectedConversation} />
          ) : (
            <div className="flex h-full items-center justify-center text-muted-foreground text-sm">
              请选择一个会话开始聊天
            </div>
          )}
        </ResizablePanel>
      </ResizablePanelGroup>

      <Toaster position="top-center" />
    </main>
  );
}

export default ChatWindowView;
