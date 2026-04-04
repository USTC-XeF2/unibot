import { create } from "zustand";
import type { MessageSegment } from "@/types/chat";

type QuotedMessage = {
  messageId: number;
  summary: string;
};

type ConversationComposerState = {
  segments: MessageSegment[];
  quotedMessage: QuotedMessage | null;
};

type ChatComposerStore = {
  byConversation: Record<string, ConversationComposerState>;
  setSegments: (conversationKey: string, segments: MessageSegment[]) => void;
  setQuotedMessage: (
    conversationKey: string,
    quotedMessage: QuotedMessage | null,
  ) => void;
  clearConversationState: (conversationKey: string) => void;
};

export const defaultConversationComposerState: ConversationComposerState = {
  segments: [],
  quotedMessage: null,
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
  setQuotedMessage: (conversationKey, quotedMessage) =>
    set((state) => ({
      byConversation: {
        ...state.byConversation,
        [conversationKey]: {
          ...(state.byConversation[conversationKey] ??
            defaultConversationComposerState),
          quotedMessage,
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
