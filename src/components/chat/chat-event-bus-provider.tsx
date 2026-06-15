import { type Event, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
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

/**
 * 当目标用户被移出/离开群（或群解散）时，独立的群文件/相册窗口必须关闭，
 * 防止已被撤销访问的用户继续操作缓存数据。主聊天窗口（`chat-*`）不受影响。
 */
function shouldCloseWindowOnMemberLeft(
  windowLabel: string,
  userId: string,
  payload: InternalEventPayload,
): boolean {
  if (
    payload.kind !== "group_member_left" ||
    payload.target_user_id !== userId
  ) {
    return false;
  }
  return (
    windowLabel === `group-files-${userId}-${payload.group_id}` ||
    windowLabel === `group-albums-${userId}-${payload.group_id}`
  );
}

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
  const [listenError, setListenError] = useState<string | null>(null);

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
        if (shouldCloseWindowOnMemberLeft(label, userId, payload)) {
          void getCurrentWindow()
            .close()
            .catch((error) => {
              console.error(
                `[event-bus] failed to close revoked window ${label}:`,
                error,
              );
            });
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
        const message = error instanceof Error ? error.message : String(error);
        console.error(`[event-bus] failed for ${label}:`, error);
        setListenError(message);
      });

    return () => {
      cancelled = true;
      setListenError(null);
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
  }, [userId, windowLabel]);

  return (
    <ChatEventBusContext.Provider value={{ subscribe }}>
      {listenError && (
        <div className="border-destructive/20 border-b bg-destructive/10 px-4 py-2 text-destructive text-sm">
          实时连接断开：{listenError}。请关闭窗口重新打开。
        </div>
      )}
      {children}
    </ChatEventBusContext.Provider>
  );
}
