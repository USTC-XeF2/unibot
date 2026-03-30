import { invoke } from "@tauri-apps/api/core";
import { Pencil, Save } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useParams } from "react-router";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { invalidateUsersQuery, useUsersQuery } from "@/lib/users-query";
import type { UserProfile } from "@/types/user";

type UseEditableFieldReturn = {
  value: string;
  draft: string;
  isEditing: boolean;
  setDraft: (next: string) => void;
  startEditing: () => void;
  finishEditing: () => void;
};

function useEditableField(value: string): UseEditableFieldReturn {
  const [draft, setDraft] = useState(value);
  const [isEditing, setIsEditing] = useState(false);

  const startEditing = useCallback(() => {
    setIsEditing(true);
  }, []);

  const finishEditing = useCallback(() => {
    setIsEditing(false);
    setDraft(value);
  }, [value]);

  useEffect(finishEditing, [finishEditing]);

  return {
    value,
    draft,
    isEditing,
    setDraft,
    startEditing,
    finishEditing,
  };
}

function UserSettingsView() {
  const { userId } = useParams();
  const usersQuery = useUsersQuery();
  const parsedUserId = useMemo(() => Number(userId), [userId]);
  const user = useMemo(
    () => (usersQuery.data ?? []).find((item) => item.user_id === parsedUserId),
    [parsedUserId, usersQuery.data],
  );

  const nicknameField = useEditableField(user?.nickname ?? "");
  const avatarField = useEditableField(user?.avatar ?? "");
  const signatureField = useEditableField(user?.signature ?? "");
  const [submitError, setSubmitError] = useState<string | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: We only want to reset the fields when the user ID changes.
  useEffect(() => {
    if (parsedUserId === undefined) {
      return;
    }

    nicknameField.finishEditing();
    avatarField.finishEditing();
    signatureField.finishEditing();
    setSubmitError(null);
  }, [parsedUserId]);

  const handleSave = async (
    field: "nickname" | "avatar" | "signature",
    editableField: UseEditableFieldReturn,
  ) => {
    if (!Number.isInteger(parsedUserId) || parsedUserId <= 0) {
      setSubmitError("无效的用户 ID");
      return;
    }

    if (!user) {
      setSubmitError("未找到该用户，无法保存");
      return;
    }

    if (field === "nickname" && !editableField.draft.trim()) {
      setSubmitError("名称不能为空");
      return;
    }

    const trimmedValue = editableField.draft.trim();
    const nickname = field === "nickname" ? trimmedValue : undefined;
    const avatar = field === "avatar" ? trimmedValue : undefined;
    const signature = field === "signature" ? trimmedValue : undefined;

    setSubmitError(null);

    await invoke<UserProfile>("update_user_profile", {
      userId: parsedUserId,
      nickname,
      avatar,
      signature,
    });
    await invalidateUsersQuery();

    editableField.finishEditing();
  };

  if (usersQuery.isPending) {
    return (
      <Card>
        <CardContent>
          <p className="text-muted-foreground text-sm">正在读取用户数据...</p>
        </CardContent>
      </Card>
    );
  }

  if (!user) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>{`用户设置 · ${userId ?? "未知用户"}`}</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="rounded-md border border-amber-300/50 bg-amber-50 px-2 py-1.5 text-amber-700 text-xs">
            未找到该用户，可能已被删除。
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-3">
      <Card>
        <CardContent className="space-y-4 text-sm">
          <div className="flex flex-1 flex-col gap-4">
            <div className="flex items-center gap-3">
              <Avatar className="size-14">
                <AvatarImage src={avatarField.value} />
                <AvatarFallback>
                  {nicknameField.value.slice(0, 1).toUpperCase()}
                </AvatarFallback>
              </Avatar>

              <div className="flex min-w-0 flex-1 flex-col gap-2">
                <div className="flex items-center gap-2">
                  {nicknameField.isEditing ? (
                    <div className="flex min-w-0 flex-1 flex-col gap-2 sm:flex-row sm:items-center">
                      <Input
                        value={nicknameField.draft}
                        onChange={(event) =>
                          nicknameField.setDraft(event.target.value)
                        }
                        placeholder="输入昵称"
                      />
                      <div className="flex items-center gap-2">
                        <Button
                          type="button"
                          size="sm"
                          onClick={() =>
                            void handleSave("nickname", nicknameField)
                          }
                        >
                          <Save className="size-3.5" />
                          保存
                        </Button>
                        <Button
                          type="button"
                          size="sm"
                          variant="outline"
                          onClick={nicknameField.finishEditing}
                        >
                          取消
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <>
                      <span className="truncate font-semibold text-lg">
                        {nicknameField.value || `用户 ${parsedUserId}`}
                      </span>
                      <Button
                        type="button"
                        size="icon-sm"
                        variant="ghost"
                        aria-label="编辑昵称"
                        onClick={() => {
                          nicknameField.startEditing();
                          setSubmitError(null);
                        }}
                      >
                        <Pencil className="size-4" />
                      </Button>
                    </>
                  )}
                </div>

                <p className="text-muted-foreground text-xs">{parsedUserId}</p>
              </div>
            </div>

            <div className="space-y-2 rounded-lg border bg-muted/30 p-3">
              <div className="flex items-center justify-between">
                <span className="font-medium text-sm">头像地址</span>
                {!avatarField.isEditing ? (
                  <Button
                    type="button"
                    size="icon-sm"
                    variant="outline"
                    aria-label="编辑头像"
                    onClick={() => {
                      avatarField.startEditing();
                      setSubmitError(null);
                    }}
                  >
                    <Pencil className="size-4" />
                  </Button>
                ) : null}
              </div>

              {avatarField.isEditing ? (
                <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
                  <Input
                    value={avatarField.draft}
                    onChange={(event) =>
                      avatarField.setDraft(event.target.value)
                    }
                  />
                  <div className="flex items-center gap-2">
                    <Button
                      type="button"
                      size="sm"
                      onClick={() => void handleSave("avatar", avatarField)}
                    >
                      <Save className="size-3.5" />
                      保存
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      onClick={avatarField.finishEditing}
                    >
                      取消
                    </Button>
                  </div>
                </div>
              ) : (
                <p className="break-all text-muted-foreground text-xs">
                  {avatarField.value || "未设置"}
                </p>
              )}
            </div>
          </div>

          <div className="space-y-2 rounded-lg border bg-muted/20 p-3">
            <div className="flex items-center justify-between">
              <span className="font-medium text-sm">个性签名</span>
              {!signatureField.isEditing ? (
                <Button
                  type="button"
                  size="icon-sm"
                  variant="outline"
                  aria-label="编辑签名"
                  onClick={() => {
                    signatureField.startEditing();
                    setSubmitError(null);
                  }}
                >
                  <Pencil className="size-4" />
                </Button>
              ) : null}
            </div>

            {signatureField.isEditing ? (
              <div className="space-y-2">
                <Textarea
                  value={signatureField.draft}
                  onChange={(event) =>
                    signatureField.setDraft(event.target.value)
                  }
                  className="min-h-24"
                />
                <div className="flex items-center gap-2">
                  <Button
                    type="button"
                    size="sm"
                    onClick={() => void handleSave("signature", signatureField)}
                  >
                    <Save className="size-3.5" />
                    保存
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={signatureField.finishEditing}
                  >
                    取消
                  </Button>
                </div>
              </div>
            ) : (
              <p className="whitespace-pre-wrap text-muted-foreground text-sm">
                {signatureField.value || "这个用户还没有设置签名。"}
              </p>
            )}
          </div>

          {submitError ? (
            <p className="rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1.5 text-destructive text-xs">
              {submitError}
            </p>
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>状态</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div className="rounded-lg border bg-muted/20 p-3">
              <p className="text-muted-foreground text-xs">好友数</p>
              <p className="mt-1 font-semibold text-lg">0</p>
            </div>
            <div className="rounded-lg border bg-muted/20 p-3">
              <p className="text-muted-foreground text-xs">添加群聊数</p>
              <p className="mt-1 font-semibold text-lg">0</p>
            </div>
            <div className="rounded-lg border bg-muted/20 p-3">
              <p className="text-muted-foreground text-xs">活跃会话数</p>
              <p className="mt-1 font-semibold text-lg">0</p>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>连接配置</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="rounded-lg border border-dashed bg-muted/20 p-6 text-center text-muted-foreground text-sm">
            暂未配置内容
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export default UserSettingsView;
