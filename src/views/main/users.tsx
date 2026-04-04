import { invoke } from "@tauri-apps/api/core";
import { MessageCircle, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useNavigate } from "react-router";
import { CreateUserSheet } from "@/components/main/create-user-sheet";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { confirmDialog } from "@/lib/modal";
import { invalidateUsersQuery, useUsersQuery } from "@/lib/query";

function UserManagementView() {
  const navigate = useNavigate();
  const usersQuery = useUsersQuery();
  const users = usersQuery.data ?? [];
  const error = usersQuery.error ? usersQuery.error.message : null;
  const [createOpen, setCreateOpen] = useState(false);
  const [deletingUserId, setDeletingUserId] = useState<number | null>(null);

  const handleOpenUserChatWindow = async (userId: number) => {
    try {
      await invoke<{ created: boolean }>("open_user_chat_window", {
        userId,
      });
    } catch (err) {
      window.alert(err as string);
    }
  };

  const handleDeleteUser = async (userId: number, nickname: string) => {
    const confirmed = await confirmDialog({
      title: "确认删除用户",
      description: `确认删除用户 ${nickname} (${userId}) 吗？`,
      confirmText: "删除",
    });
    if (!confirmed) {
      return;
    }

    setDeletingUserId(userId);
    try {
      await invoke("delete_user", { userId });
      await invalidateUsersQuery();
    } catch (err) {
      window.alert(err as string);
    } finally {
      setDeletingUserId(null);
    }
  };

  return (
    <div className="space-y-3">
      <section className="flex h-14 items-center justify-between rounded-lg border bg-card/70 px-4">
        <div className="flex items-center gap-3">
          <span className="text-muted-foreground text-sm">当前总用户数量</span>
          <span className="font-semibold text-lg">{users.length}</span>
        </div>

        <Button
          type="button"
          size="sm"
          className="gap-1.5"
          onClick={() => setCreateOpen(true)}
        >
          <Plus className="size-4" /> 创建用户
        </Button>
      </section>

      <CreateUserSheet open={createOpen} onOpenChange={setCreateOpen} />

      <section className="overflow-hidden rounded-lg border bg-card/70">
        <div className="max-h-[70vh] overflow-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-20">头像</TableHead>
                <TableHead className="w-40">用户 ID</TableHead>
                <TableHead className="w-52">名称</TableHead>
                <TableHead>签名</TableHead>
                <TableHead className="w-52">操作</TableHead>
              </TableRow>
            </TableHeader>

            <TableBody>
              {error ? (
                <TableRow>
                  <TableCell
                    colSpan={5}
                    className="py-8 text-center text-destructive"
                  >
                    {error}
                  </TableCell>
                </TableRow>
              ) : users.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={5}
                    className="py-8 text-center text-muted-foreground"
                  >
                    暂无用户数据
                  </TableCell>
                </TableRow>
              ) : (
                users.map((user) => {
                  return (
                    <TableRow
                      key={user.user_id}
                      onClick={() => navigate(`/user/${user.user_id}`)}
                    >
                      <TableCell>
                        <Avatar className="size-9">
                          <AvatarImage src={user.avatar} />
                          <AvatarFallback>
                            {user.nickname.slice(0, 1).toUpperCase()}
                          </AvatarFallback>
                        </Avatar>
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {user.user_id}
                      </TableCell>
                      <TableCell>{user.nickname}</TableCell>
                      <TableCell className="text-muted-foreground text-sm">
                        {user.signature || "-"}
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Button
                            type="button"
                            size="icon-sm"
                            variant="outline"
                            aria-label="打开聊天窗口"
                            onClick={(event) => {
                              event.stopPropagation();
                              handleOpenUserChatWindow(user.user_id);
                            }}
                          >
                            <MessageCircle className="size-4" />
                          </Button>
                          <Button
                            type="button"
                            size="icon-sm"
                            variant="outline"
                            aria-label="删除用户"
                            className="border-destructive/40 text-destructive hover:bg-destructive/10 hover:text-destructive"
                            disabled={deletingUserId === user.user_id}
                            onClick={(event) => {
                              event.stopPropagation();
                              handleDeleteUser(user.user_id, user.nickname);
                            }}
                          >
                            <Trash2 className="size-4" />
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </div>
      </section>
    </div>
  );
}

export default UserManagementView;
