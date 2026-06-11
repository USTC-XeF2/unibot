import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useRef,
} from "react";
import { handleQueryInvalidation } from "@/lib/query";
import type { InternalEventPayload } from "@/types/event";

export type ChatEventSubscriber = (payload: InternalEventPayload) => void;

type ChatEventBusContextValue = {
  subscribe: (callback: ChatEventSubscriber) => () => void;
};

const ChatEventBusContext = createContext<ChatEventBusContextValue | null>(
  null,
);

export function useChatEventBusContext(): ChatEventBusContextValue {
  const value = useContext(ChatEventBusContext);
  if (!value) {
    throw new Error(
      "useChatEventBus must be used within a ChatEventBusProvider",
    );
  }
  return value;
}

export function ChatEventBusProvider({
  userId,
  children,
}: {
  userId: string;
  children: ReactNode;
}) {
  const subscribersRef = useRef<Set<ChatEventSubscriber>>(new Set());

  const subscribe = useCallback((callback: ChatEventSubscriber) => {
    subscribersRef.current.add(callback);
    return () => {
      subscribersRef.current.delete(callback);
    };
  }, []);

  useEffect(() => {
    if (!userId) {
      return;
    }

    let cancelled = false;
    let unlisten: (() => void) | null = null;

    const webview = getCurrentWebviewWindow();
    webview
      .listen<InternalEventPayload>("chat:event", (event) => {
        const payload = event.payload;
        if (!payload) {
          return;
        }
        handleQueryInvalidation(userId, payload);
        for (const subscriber of subscribersRef.current) {
          subscriber(payload);
        }
      })
      .then((fn) => {
        if (cancelled) {
          // effect 已在 listen resolve 前 cleanup，立即解绑避免泄漏
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error) => {
        console.error("failed to listen chat:event", error);
      });

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  }, [userId]);

  return (
    <ChatEventBusContext.Provider value={{ subscribe }}>
      {children}
    </ChatEventBusContext.Provider>
  );
}
