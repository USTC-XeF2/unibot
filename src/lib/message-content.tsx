import Face from "@/components/chat/face";
import { faceById, resolveUserDisplayName } from "@/lib/utils";
import type { MessageSegment } from "@/types/chat";
import type { UserProfile } from "@/types/user";

function segmentToText(segment: MessageSegment) {
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
      const face = faceById.get(segment.data.id);
      if (!face) {
        return "[表情]";
      }
      return `[${face.label}]`;
    }
    default:
      return "";
  }
}

export function segmentsToPlainText(segments: MessageSegment[]) {
  return segments.map(segmentToText).join("");
}

export function segmentsToNodes(
  segments: MessageSegment[],
  users: UserProfile[],
) {
  return segments.map((segment) => {
    switch (segment.type) {
      case "At": {
        const targetUserId = segment.data.target;
        const displayName = resolveUserDisplayName(
          targetUserId,
          users.find((user) => user.user_id === targetUserId)?.nickname,
        );
        return `@${displayName}`;
      }
      case "Face":
        return (
          <Face
            key={`face-${segment.data.id}`}
            id={segment.data.id}
            className="mx-0.5 size-3.5 align-[-2px]"
          />
        );
      default:
        return segmentToText(segment);
    }
  });
}
