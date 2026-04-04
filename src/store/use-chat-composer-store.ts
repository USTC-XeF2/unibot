import { create } from "zustand";
import type { MessageSegment } from "@/types/chat";

type ConversationComposerState = {
  segments: MessageSegment[];
  quotedMessageId: number | null;
};

type ChatComposerStore = {
  byConversation: Record<string, ConversationComposerState>;
  setSegments: (conversationKey: string, segments: MessageSegment[]) => void;
  setQuotedMessage: (
    conversationKey: string,
    quotedMessageId: number | null,
  ) => void;
  clearConversationState: (conversationKey: string) => void;
};

export const defaultConversationComposerState: ConversationComposerState = {
  segments: [],
  quotedMessageId: null,
};

export const useChatComposerStore = create<ChatComposerStore>((set) => ({
  byConversation: {},
  setSegments: (conversationKey, segments) =>
    set((state) => ({
      byConversation: {
        ...state.byConversation,
        [conversationKey]: {
          ...(state.byConversation[conversationKey] ??
            defaultConversationComposerState),
          segments,
        },
      },
    })),
  setQuotedMessage: (conversationKey, quotedMessageId) =>
    set((state) => ({
      byConversation: {
        ...state.byConversation,
        [conversationKey]: {
          ...(state.byConversation[conversationKey] ??
            defaultConversationComposerState),
          quotedMessageId,
        },
      },
    })),
  clearConversationState: (conversationKey) =>
    set((state) => {
      if (!(conversationKey in state.byConversation)) {
        return state;
      }

      const next = { ...state.byConversation };
      delete next[conversationKey];
      return { byConversation: next };
    }),
}));
