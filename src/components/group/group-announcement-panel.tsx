import { Megaphone, Pencil, Plus, Trash2 } from "lucide-react";
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
import { confirmDialog } from "@/lib/modal";
import {
  useDeleteGroupAnnouncementMutation,
  useUpsertGroupAnnouncementMutation,
} from "@/lib/mutations";
import { useGroupAnnouncementsQuery } from "@/lib/query";
import type { GroupAnnouncement } from "@/types/group";

type DialogMode =
  | { type: "create" }
  | { type: "edit"; announcement: GroupAnnouncement };

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
  const [dialogMode, setDialogMode] = useState<DialogMode | null>(null);
  const [content, setContent] = useState("");
  const mutation = useUpsertGroupAnnouncementMutation();
  const deleteMutation = useDeleteGroupAnnouncementMutation();

  const isEdit = dialogMode?.type === "edit";
  const dialogTitle = isEdit ? "编辑公告" : "发布公告";

  const openCreate = () => {
    setContent("");
    setDialogMode({ type: "create" });
  };

  const openEdit = (announcement: GroupAnnouncement) => {
    setContent(announcement.content);
    setDialogMode({ type: "edit", announcement });
  };

  const closeDialog = () => {
    setDialogMode(null);
    setContent("");
  };

  const handleSubmit = async () => {
    if (!content.trim()) return;
    await mutation.mutateAsync({
      userId,
      groupId,
      announcementId: isEdit
        ? dialogMode.announcement.announcement_id
        : undefined,
      content: content.trim(),
    });
    closeDialog();
  };

  const handleDelete = async (announcement: GroupAnnouncement) => {
    const confirmed = await confirmDialog({
      title: "确认删除公告",
      description: "确定要删除这条公告吗？此操作不可恢复。",
      confirmText: "删除",
    });
    if (!confirmed) return;

    deleteMutation.mutate({
      userId,
      groupId,
      announcementId: announcement.announcement_id,
    });
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b p-3">
        <h2 className="font-semibold">群公告</h2>
        {canManage && (
          <Dialog
            open={dialogMode !== null}
            onOpenChange={(open) => {
              if (!open) closeDialog();
            }}
          >
            <DialogTrigger asChild>
              <Button size="sm" onClick={openCreate}>
                <Plus className="mr-1 size-4" />
                发布公告
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>{dialogTitle}</DialogTitle>
              </DialogHeader>
              <Textarea
                value={content}
                onChange={(e) => setContent(e.target.value)}
                placeholder="请输入公告内容..."
                rows={6}
              />
              <Button onClick={handleSubmit} disabled={!content.trim()}>
                {isEdit ? "保存" : "发布"}
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
            <AnnouncementItem
              key={announcement.announcement_id}
              announcement={announcement}
              canManage={canManage}
              onEdit={openEdit}
              onDelete={handleDelete}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function AnnouncementItem({
  announcement,
  canManage,
  onEdit,
  onDelete,
}: {
  announcement: GroupAnnouncement;
  canManage: boolean;
  onEdit: (announcement: GroupAnnouncement) => void;
  onDelete: (announcement: GroupAnnouncement) => Promise<void> | void;
}) {
  return (
    <div className="group relative rounded-lg border p-3">
      <div className="flex items-start gap-2">
        <Megaphone className="mt-0.5 size-4 text-primary" />
        <div className="flex-1">
          <p className="whitespace-pre-wrap text-sm">{announcement.content}</p>
          <p className="mt-2 text-muted-foreground text-xs">
            — {announcement.sender_user_id} ·{" "}
            {new Date(announcement.created_at).toLocaleString()}
          </p>
        </div>
      </div>
      {canManage && (
        <div className="absolute top-2 right-2 flex items-center gap-1 opacity-0 group-hover:opacity-100">
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => onEdit(announcement)}
          >
            <Pencil className="size-4 text-muted-foreground" />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => onDelete(announcement)}
          >
            <Trash2 className="size-4 text-destructive" />
          </Button>
        </div>
      )}
    </div>
  );
}
