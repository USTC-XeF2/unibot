import emojiConfig from "@/assets/default-emojis/default_config.json";

type EmojiConfig = {
  normalPanelResult?: {
    SysEmojiGroupList?: Array<{
      groupName?: string;
      SysEmojiList?: Array<{
        emojiId: string;
        describe: string;
        isHide: boolean;
      }>;
    }>;
  };
};

export type FaceDefinition = {
  id: string;
  label: string;
  src: string;
  groupName: string;
};

export type FaceGroup = {
  groupName: string;
  faces: FaceDefinition[];
};

const emojiAssets = import.meta.glob("../assets/default-emojis/*.png", {
  eager: true,
  import: "default",
}) as Record<string, string>;

function resolveEmojiAsset(emojiId: string) {
  return emojiAssets[`../assets/default-emojis/${emojiId}.png`] ?? null;
}

function normalizeLabel(label: string) {
  return label.startsWith("/") ? label.slice(1) : label;
}

const configuredGroups =
  (emojiConfig as EmojiConfig).normalPanelResult?.SysEmojiGroupList ?? [];

export const faceGroups: FaceGroup[] = configuredGroups
  .map((group) => {
    const faces =
      group.SysEmojiList?.flatMap((emoji) => {
        if (emoji.isHide) {
          return [];
        }

        const src = resolveEmojiAsset(emoji.emojiId);
        if (!src) {
          return [];
        }

        return [
          {
            id: emoji.emojiId,
            label: normalizeLabel(emoji.describe),
            src,
            groupName: group.groupName?.trim() || "默认表情",
          },
        ];
      }) ?? [];

    return {
      groupName: group.groupName?.trim() || "默认表情",
      faces,
    };
  })
  .filter((group) => group.faces.length > 0);

export const faceLibrary = faceGroups.flatMap((group) => group.faces);

const faceById = new Map(faceLibrary.map((face) => [face.id, face] as const));
const faceByLabel = new Map(
  faceLibrary.map((face) => [face.label, face] as const),
);

export function getFaceById(id: string) {
  return faceById.get(id) ?? null;
}

export function getFaceByLabel(label: string) {
  return faceByLabel.get(label) ?? null;
}

export function faceToken(face: Pick<FaceDefinition, "label">) {
  return `[${face.label}]`;
}
