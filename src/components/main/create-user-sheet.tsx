import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { ButtonGroup } from "@/components/ui/button-group";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { invalidateUsersQuery } from "@/lib/query";
import type { UserProfile } from "@/types/user";

type CreateUserSheetProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

export function CreateUserSheet({ open, onOpenChange }: CreateUserSheetProps) {
  const [submitting, setSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [formUserId, setFormUserId] = useState("");
  const [formNickname, setFormNickname] = useState("");
  const [formAvatar, setFormAvatar] = useState("");

  const avatarPreviewLabel = formNickname.trim() || formUserId.trim() || "?";

  const resetForm = () => {
    setFormUserId("");
    setFormNickname("");
    setFormAvatar("");
    setSubmitError(null);
  };

  const handleCreateUser = async () => {
    const parsedUserId = Number(formUserId);
    if (
      !Number.isInteger(parsedUserId) ||
      parsedUserId <= 10000 ||
      parsedUserId > 4294967295
    ) {
      setSubmitError("用户 ID 必须是 10001 至 4294967295 的整数");
      return;
    }
    if (!formNickname.trim()) {
      setSubmitError("名称不能为空");
      return;
    }

    setSubmitting(true);
    setSubmitError(null);

    try {
      await invoke<UserProfile>("register_user", {
        userId: parsedUserId,
        nickname: formNickname.trim(),
        avatar: formAvatar.trim(),
        signature: "",
      });
      onOpenChange(false);
      resetForm();
      await invalidateUsersQuery();
    } catch (err) {
      setSubmitError(err as string);
    } finally {
      setSubmitting(false);
    }
  };

  const handleFillQqAvatar = () => {
    const userId = formUserId.trim();
    if (userId) {
      setFormAvatar(`http://q.qlogo.cn/g?b=qq&nk=${userId}&s=640`);
    }
  };

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>创建用户</SheetTitle>
        </SheetHeader>

        <FieldGroup className="px-4">
          <div className="flex justify-start">
            <Avatar className="size-24">
              <AvatarImage src={formAvatar.trim()} alt="用户头像预览" />
              <AvatarFallback className="text-2xl">
                {avatarPreviewLabel.slice(0, 1).toUpperCase()}
              </AvatarFallback>
            </Avatar>
          </div>

          <Field>
            <FieldLabel htmlFor="create-user-id">用户 ID</FieldLabel>
            <Input
              id="create-user-id"
              inputMode="numeric"
              autoComplete="off"
              value={formUserId}
              onChange={(event) => setFormUserId(event.target.value)}
            />
          </Field>

          <Field>
            <FieldLabel htmlFor="create-user-name">名称</FieldLabel>
            <Input
              id="create-user-name"
              autoComplete="off"
              value={formNickname}
              onChange={(event) => setFormNickname(event.target.value)}
            />
          </Field>

          <Field>
            <FieldLabel htmlFor="create-user-avatar">头像 URL</FieldLabel>

            <ButtonGroup>
              <Input
                id="create-user-avatar"
                autoComplete="off"
                value={formAvatar}
                onChange={(event) => setFormAvatar(event.target.value)}
              />
              <Button
                type="button"
                variant="outline"
                onClick={handleFillQqAvatar}
              >
                填入QQ头像
              </Button>
            </ButtonGroup>
          </Field>

          {submitError ? (
            <p className="rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1.5 text-destructive text-xs">
              {submitError}
            </p>
          ) : null}

          <Field orientation="horizontal">
            <Button
              type="button"
              onClick={handleCreateUser}
              disabled={submitting}
            >
              {submitting ? "创建中..." : "创建用户"}
            </Button>
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                onOpenChange(false);
                resetForm();
              }}
              disabled={submitting}
            >
              取消
            </Button>
          </Field>
        </FieldGroup>
      </SheetContent>
    </Sheet>
  );
}
