import { getFaceById, getFaceByLabel } from "@/lib/face-library";
import type { MessageSegment } from "@/types/chat";

type ParseDraftOptions = {
  allowMentions?: boolean;
};

export type DraftDisplayToken =
  | {
      kind: "text";
      raw: string;
      text: string;
    }
  | {
      kind: "face";
      raw: string;
      faceId: string;
    }
  | {
      kind: "mention";
      raw: string;
      label: string;
      userId: number;
    };

const tokenPattern = /\[@([^\]]+):(\d+)\]|\[([^[\]]+)\]/g;

function pushTextSegment(segments: MessageSegment[], text: string) {
  if (!text) {
    return;
  }

  const previous = segments[segments.length - 1];
  if (previous?.type === "Text") {
    previous.data.text += text;
    return;
  }

  segments.push({ type: "Text", data: { text } });
}

export function tokenizeDraftForDisplay(
  draft: string,
  options: ParseDraftOptions = {},
) {
  const { allowMentions = true } = options;
  const tokens: DraftDisplayToken[] = [];
  let lastIndex = 0;

  tokenPattern.lastIndex = 0;

  let match = tokenPattern.exec(draft);
  while (match) {
    const [rawToken, mentionName, mentionUserId, faceLabel] = match;
    const tokenStart = match.index;

    if (tokenStart > lastIndex) {
      const text = draft.slice(lastIndex, tokenStart);
      tokens.push({
        kind: "text",
        raw: text,
        text,
      });
    }

    if (mentionName && mentionUserId && allowMentions) {
      tokens.push({
        kind: "mention",
        raw: rawToken,
        label: mentionName,
        userId: Number(mentionUserId),
      });
    } else if (faceLabel) {
      const face = getFaceByLabel(faceLabel);
      if (face) {
        tokens.push({
          kind: "face",
          raw: rawToken,
          faceId: face.id,
        });
      } else {
        tokens.push({
          kind: "text",
          raw: rawToken,
          text: rawToken,
        });
      }
    } else {
      tokens.push({
        kind: "text",
        raw: rawToken,
        text: rawToken,
      });
    }

    lastIndex = tokenPattern.lastIndex;
    match = tokenPattern.exec(draft);
  }

  if (lastIndex < draft.length) {
    const text = draft.slice(lastIndex);
    tokens.push({
      kind: "text",
      raw: text,
      text,
    });
  }

  return tokens;
}

export function parseDraftToMessageSegments(
  draft: string,
  options: ParseDraftOptions = {},
) {
  const segments: MessageSegment[] = [];
  const tokens = tokenizeDraftForDisplay(draft, options);

  for (const token of tokens) {
    if (token.kind === "text") {
      pushTextSegment(segments, token.text);
      continue;
    }

    if (token.kind === "mention") {
      segments.push({ type: "At", data: { target: token.userId } });
      continue;
    }

    segments.push({ type: "Face", data: { id: token.faceId } });
  }

  return segments;
}

export function messageSegmentsToPlainText(segments: MessageSegment[]) {
  const text = segments
    .map((segment) => {
      switch (segment.type) {
        case "Text":
          return segment.data.text;
        case "At":
          return `@${segment.data.target}`;
        case "Reply":
          return `[回复:${segment.data.message_id}]`;
        case "Image":
          return "[图片]";
        case "Face": {
          const face = getFaceById(segment.data.id);
          if (!face) {
            return "[表情]";
          }
          return `[${face.label}]`;
        }
        default:
          return "";
      }
    })
    .join("");

  return text || "[空消息]";
}
