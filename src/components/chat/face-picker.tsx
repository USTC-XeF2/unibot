import { Smile } from "lucide-react";
import { useState } from "react";
import Face from "@/components/chat/face";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import faceGroups from "@/data/faces.json";
import { cn } from "@/lib/utils";

type FacePickerProps = {
  onSelectFace: (faceId: string) => void;
};

function FacePicker({ onSelectFace }: FacePickerProps) {
  const [open, setOpen] = useState(false);

  return (
    <DropdownMenu open={open} onOpenChange={setOpen} modal={false}>
      <DropdownMenuTrigger asChild>
        <Button type="button" variant="ghost" size="icon-sm" title="表情">
          <Smile className="size-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="top"
        align="start"
        className="w-80 min-w-80 overflow-hidden p-0"
      >
        <div className="max-h-80 space-y-3 overflow-auto p-2">
          {faceGroups.map((group) => (
            <section key={group.groupName}>
              <p className="p-1 text-muted-foreground text-xs">
                {group.groupName}
              </p>
              <div className="grid grid-cols-8 gap-1">
                {group.faces.map((face) => (
                  <button
                    key={face.id}
                    type="button"
                    className={cn(
                      "flex items-center justify-center rounded-md border border-transparent px-1 transition-colors hover:border-border hover:bg-muted/60 focus-visible:border-ring focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
                    )}
                    title={face.label}
                    onClick={() => {
                      onSelectFace(face.id);
                      setOpen(false);
                    }}
                  >
                    <Face id={face.id} className="size-8" />
                  </button>
                ))}
              </div>
            </section>
          ))}
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export default FacePicker;
