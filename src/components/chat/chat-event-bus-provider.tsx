import { type Event, listen, type UnlistenFn } from "@tauri-apps/api/event";
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

// Tauri's event listener may be called before `window.__TAURI_INTERNALS__`
// is injected in a freshly opened webview. Retry on the specific "Tauri
// internals not available" error instead of polling the private global.
async function listenWithRetry<T>(
  event: string,
  handler: (event: Event<T>) => void,
  options: { target?: string },
  maxRetries = 30,
  delayMs = 50,
): Promise<UnlistenFn> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await listen<T>(event, handler, options);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const name = error instanceof Error ? error.name : "";
      const isReadinessError =
        message.includes("Tauri internals not available") ||
        message.includes("__TAURI_INTERNALS__") ||
        name.includes("TauriError");
      if (isReadinessError && i < maxRetries - 1) {
        await new Promise((resolve) => setTimeout(resolve, delayMs));
        continue;
      }
      throw error;
    }
  }
  throw new Error(`failed to register listener for ${event} after retries`);
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
  windowLabel,
  children,
}: {
  userId: string;
  windowLabel?: string;
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

    const label = windowLabel || `chat-${userId}`;

    let cancelled = false;
    let unlisten: (() => void) | null = null;

    listenWithRetry<InternalEventPayload>(
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
        target: label,
      },
    )
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error) => {
        console.error(`[event-bus] failed for ${label}:`, error);
      });

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  }, [userId, windowLabel]);

  return (
    <ChatEventBusContext.Provider value={{ subscribe }}>
      {children}
    </ChatEventBusContext.Provider>
  );
}
