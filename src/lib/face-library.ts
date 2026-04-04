import emojiConfig from "@/assets/emojis.json";

export type FaceDefinition = {
  id: string;
  label: string;
  src: string;
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

export const faceGroups: FaceGroup[] = emojiConfig.map((group) => {
  const faces = group.emojis
    .map((emoji) => {
      const src = resolveEmojiAsset(emoji.id);
      if (!src) {
        return null;
      }

      return {
        id: emoji.id,
        label: emoji.label,
        src,
      };
    })
    .filter((v) => !!v);

  return {
    groupName: group.groupName,
    faces,
  };
});

const faceById = new Map(
  faceGroups
    .flatMap((group) => group.faces)
    .map((face) => [face.id, face] as const),
);

export function getFaceById(id: string) {
  return faceById.get(id) ?? null;
}
