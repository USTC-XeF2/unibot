import { useEffect, useRef } from "react";
import {
  type ChatEventSubscriber,
  useChatEventBusContext,
} from "@/components/chat/chat-event-bus-provider";
import type { InternalEventPayload } from "@/types/event";

/**
 * 订阅当前窗口的聊天事件，执行组件级回调。
 * Query 失效由 ChatEventBusProvider 统一处理，此 hook 只负责组件回调。
 *
 * @param userId 当前窗口用户 id；为空时不订阅。
 * @param onEvent 组件级回调；不传则不订阅。
 */
export function useChatEventBus(
  userId: string,
  onEvent?: (payload: InternalEventPayload) => void,
) {
  const { subscribe } = useChatEventBusContext();
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  useEffect(() => {
    if (!userId) {
      return;
    }

    const subscriber: ChatEventSubscriber = (payload) => {
      onEventRef.current?.(payload);
    };

    return subscribe(subscriber);
  }, [userId, subscribe]);
}
