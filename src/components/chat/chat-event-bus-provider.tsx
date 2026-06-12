import { listen } from "@tauri-apps/api/event";
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

// Tauri internals may not be ready immediately when a chat window webview
// finishes loading. Poll briefly before registering the listener to avoid
// a runtime "Tauri internals not available" error.
function waitForTauriInternals(timeoutMs = 5000): Promise<void> {
  return new Promise((resolve, reject) => {
    if (typeof window === "undefined") {
      reject(new Error("window is not available"));
      return;
    }

    if (
      (window as unknown as { __TAURI_INTERNALS__?: unknown })
        .__TAURI_INTERNALS__
    ) {
      resolve();
      return;
    }

    const startTime = Date.now();
    const interval = window.setInterval(() => {
      if (
        (window as unknown as { __TAURI_INTERNALS__?: unknown })
          .__TAURI_INTERNALS__
      ) {
        window.clearInterval(interval);
        resolve();
        return;
      }

      if (Date.now() - startTime > timeoutMs) {
        window.clearInterval(interval);
        reject(new Error("Tauri internals not available within timeout"));
      }
    }, 50);
  });
}

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

    const windowLabel = `chat-${userId}`;

    waitForTauriInternals(1000)
      .then(() => {
        if (cancelled) {
          return;
        }
        return listen<InternalEventPayload>(
          "chat:event",
          (event) => {
            const payload = event.payload;
            if (!payload) {
              return;
            }
            handleQueryInvalidation(userId, payload);
            for (const subscriber of subscribersRef.current) {
              subscriber(payload);
            }
          },
          {
            target: windowLabel,
          },
        );
      })
      .then((fn) => {
        if (!fn) {
          return;
        }
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error) => {
        console.error(`[event-bus] failed for ${windowLabel}:`, error);
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
