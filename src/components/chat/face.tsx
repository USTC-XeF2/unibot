import { cn, faceById } from "@/lib/utils";

type FaceProps = {
  id: string;
  className?: string;
};

function Face({ id, className }: FaceProps) {
  const face = faceById.get(id);

  return (
    <img
      src={`/faces/${id}.png`}
      alt={face?.label}
      className={cn("inline object-contain", className)}
      draggable={false}
    />
  );
}

export default Face;
