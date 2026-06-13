import { Megaphone, Plus } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { useUpsertGroupAnnouncementMutation } from "@/lib/mutations";
import { useGroupAnnouncementsQuery } from "@/lib/query";

export default function GroupAnnouncementPanel({
  userId,
  groupId,
  canManage,
}: {
  userId: string;
  groupId: string;
  canManage: boolean;
}) {
  const { data: announcements = [] } = useGroupAnnouncementsQuery(
    userId,
    groupId,
  );
  const [open, setOpen] = useState(false);
  const [content, setContent] = useState("");
  const mutation = useUpsertGroupAnnouncementMutation();

  const handleSubmit = async () => {
    if (!content.trim()) return;
    await mutation.mutateAsync({
      userId,
      groupId,
      content: content.trim(),
    });
    setContent("");
    setOpen(false);
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b p-3">
        <h2 className="font-semibold">群公告</h2>
        {canManage && (
          <Dialog open={open} onOpenChange={setOpen}>
            <DialogTrigger asChild>
              <Button size="sm">
                <Plus className="mr-1 size-4" />
                发布公告
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>发布公告</DialogTitle>
              </DialogHeader>
              <Textarea
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder="请输入公告内容..."
                rows={6}
              />
              <Button onClick={handleSubmit} disabled={!content.trim()}>
                发布
              </Button>
            </DialogContent>
          </Dialog>
        )}
      </div>

      <div className="flex-1 overflow-auto p-3">
        {announcements.length === 0 && (
          <div className="py-8 text-center text-muted-foreground text-sm">
            暂无公告
          </div>
        )}
        <div className="space-y-3">
          {announcements.map((announcement) => (
            <div
              key={announcement.announcement_id}
              className="rounded-lg border p-3"
            >
              <div className="flex items-start gap-2">
                <Megaphone className="mt-0.5 size-4 text-primary" />
                <div className="flex-1">
                  <p className="whitespace-pre-wrap text-sm">
                    {announcement.content}
                  </p>
                  <p className="mt-2 text-muted-foreground text-xs">
                    — {announcement.sender_user_id} ·{" "}
                    {new Date(announcement.created_at).toLocaleString()}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
