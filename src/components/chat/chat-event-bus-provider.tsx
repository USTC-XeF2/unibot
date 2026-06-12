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
    console.log(
      "[ChatEventBusProvider] waiting for Tauri internals for",
      windowLabel,
    );

    waitForTauriInternals()
      .then(() => {
        if (cancelled) {
          return;
        }
        console.log(
          "[ChatEventBusProvider] Tauri ready, registering listener for",
          windowLabel,
        );
        return listen<InternalEventPayload>(
          "chat:event",
          (event) => {
            const payload = event.payload;
            console.log(
              "[ChatEventBusProvider] received event:",
              payload?.kind,
              "for user",
              userId,
            );
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
        console.log(
          "[ChatEventBusProvider] listener registered for",
          windowLabel,
        );
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error) => {
        console.error(
          "[ChatEventBusProvider] failed to listen chat:event",
          error,
        );
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
