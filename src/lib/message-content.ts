import { getFaceById } from "@/lib/face-library";
import type { MessageSegment } from "@/types/chat";

export function messageSegmentsToPlainText(segments: MessageSegment[]) {
  const text = segments
    .map((segment) => {
      switch (segment.type) {
        case "Text":
          return segment.data.text;
        case "At":
          return `@${segment.data.target}`;
        case "AtAll":
          return "@全体成员";
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
