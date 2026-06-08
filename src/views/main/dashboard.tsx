import {
  Bot,
  MessageCircle,
  Play,
  Plus,
  Power,
  SquareUser,
  Trash2,
  Users,
} from "lucide-react";
import { useState } from "react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import {
  useCreateBotMutation,
  useDeleteBotMutation,
  useStartBotMutation,
  useStopBotMutation,
} from "@/lib/mutations";
import {
  useBotStatsQuery,
  useBotsQuery,
  useGroupsQuery,
  useUsersQuery,
} from "@/lib/query";
import type { UserProfile } from "@/types/user";

type CreateBotSheetProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  users: UserProfile[];
  boundUserIds: Set<string>;
};

function CreateBotSheet({
  open,
  onOpenChange,
  users,
  boundUserIds,
}: CreateBotSheetProps) {
  const createBot = useCreateBotMutation();
  const [selectedUserId, setSelectedUserId] = useState("");
  const [submitError, setSubmitError] = useState<string | null>(null);

  const unboundUsers = users.filter((user) => !boundUserIds.has(user.user_id));
  const selectedUser = users.find((user) => user.user_id === selectedUserId);
  const selectedUserLabel =
    selectedUser?.nickname.trim() || selectedUser?.user_id || "?";

  const resetForm = () => {
    setSelectedUserId("");
    setSubmitError(null);
  };

  const handleOpenChange = (nextOpen: boolean) => {
    onOpenChange(nextOpen);
    if (!nextOpen) {
      resetForm();
    }
  };

  const handleCreateBot = () => {
    if (!selectedUser) {
      setSubmitError("请选择要绑定的用户");
      return;
    }

    setSubmitError(null);
    createBot.mutate(
      {
        boundUserId: selectedUser.user_id,
        displayName: selectedUser.nickname,
      },
      {
        onSuccess: () => {
          handleOpenChange(false);
        },
        onError: (err) => {
          setSubmitError(String(err));
        },
      },
    );
  };

  return (
    <Sheet open={open} onOpenChange={handleOpenChange}>
      <SheetTrigger asChild>
        <Button type="button" size="sm" variant="outline">
          <Plus className="size-4" />
          创建 Bot
        </Button>
      </SheetTrigger>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>创建 Bot</SheetTitle>
        </SheetHeader>

        <FieldGroup className="px-4">
          <div className="flex justify-start">
            <Avatar className="size-16">
              <AvatarImage src={selectedUser?.avatar} alt="绑定用户头像预览" />
              <AvatarFallback className="text-lg">
                {selectedUserLabel.slice(0, 1).toUpperCase()}
              </AvatarFallback>
            </Avatar>
          </div>

          <Field>
            <FieldLabel htmlFor="create-bot-user">绑定用户</FieldLabel>
            <Select
              value={selectedUserId}
              onValueChange={(value) => {
                setSelectedUserId(value);
                setSubmitError(null);
              }}
              disabled={unboundUsers.length === 0}
            >
              <SelectTrigger id="create-bot-user" className="w-full">
                <SelectValue
                  placeholder={
                    unboundUsers.length === 0
                      ? "没有可绑定的用户"
                      : "选择要绑定的用户"
                  }
                />
              </SelectTrigger>
              <SelectContent>
                {unboundUsers.map((user) => (
                  <SelectItem key={user.user_id} value={user.user_id}>
                    {user.nickname} ({user.user_id})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          {selectedUser ? (
            <p className="text-muted-foreground text-xs">
              将创建名为 {selectedUser.nickname} 的 Bot，并绑定到用户{" "}
              {selectedUser.user_id}。
            </p>
          ) : null}

          {submitError ? (
            <p className="rounded-md border border-destructive/30 bg-destructive/5 px-2 py-1.5 text-destructive text-xs">
              {submitError}
            </p>
          ) : null}

          <Field orientation="horizontal">
            <Button
              type="button"
              onClick={handleCreateBot}
              disabled={!selectedUserId || createBot.isPending}
            >
              {createBot.isPending ? "创建中..." : "创建 Bot"}
            </Button>
            <Button
              type="button"
              variant="outline"
              onClick={() => handleOpenChange(false)}
              disabled={createBot.isPending}
            >
              取消
            </Button>
          </Field>
        </FieldGroup>
      </SheetContent>
    </Sheet>
  );
}

function StatValue({
  value,
  loading,
}: {
  value: number | null;
  loading: boolean;
}) {
  if (loading) {
    return <span className="text-lg text-muted-foreground">读取中...</span>;
  }

  if (value === null) {
    return <span className="text-lg text-muted-foreground">--</span>;
  }

  return (
    <span className="font-semibold text-2xl">
      {value.toLocaleString("zh-CN")}
    </span>
  );
}

function DashboardView() {
  const usersQuery = useUsersQuery();
  const groupsQuery = useGroupsQuery();
  const statsQuery = useBotStatsQuery();
  const botsQuery = useBotsQuery();

  const deleteBot = useDeleteBotMutation();
  const startBot = useStartBotMutation();
  const stopBot = useStopBotMutation();

  const [sheetOpen, setSheetOpen] = useState(false);

  const users = usersQuery.data ?? [];
  const bots = botsQuery.data ?? [];
  const stats = statsQuery.data;
  const boundUserIds = new Set(bots.map((bot) => bot.bound_user_id));

  return (
    <div className="space-y-4">
      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <SquareUser className="size-4" /> 总用户数
            </CardTitle>
          </CardHeader>
          <CardContent>
            <StatValue value={users.length} loading={usersQuery.isPending} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Users className="size-4" /> 总群聊数
            </CardTitle>
          </CardHeader>
          <CardContent>
            <StatValue
              value={groupsQuery.data?.length ?? null}
              loading={groupsQuery.isPending}
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <MessageCircle className="size-4" /> 总消息数
            </CardTitle>
          </CardHeader>
          <CardContent>
            <StatValue
              value={stats?.total_messages ?? null}
              loading={statsQuery.isPending}
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Bot className="size-4" /> 在线机器人数
            </CardTitle>
          </CardHeader>
          <CardContent>
            <StatValue
              value={stats?.online_bots ?? null}
              loading={statsQuery.isPending}
            />
          </CardContent>
        </Card>
      </section>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between gap-3 pb-2">
          <CardTitle className="flex items-center gap-2 text-sm">
            <Bot className="size-4" /> Bot 管理
          </CardTitle>
          <CreateBotSheet
            open={sheetOpen}
            onOpenChange={setSheetOpen}
            users={users}
            boundUserIds={boundUserIds}
          />
        </CardHeader>
        <CardContent>
          {bots.length === 0 ? (
            <p className="text-muted-foreground text-sm">暂无 Bot</p>
          ) : (
            <div className="space-y-2">
              {bots.map((bot) => (
                <div
                  key={bot.bot_id}
                  className="flex items-center justify-between gap-3 rounded-lg border p-3"
                >
                  <div className="min-w-0 space-y-1">
                    <p className="truncate font-medium text-sm">
                      {bot.display_name}
                    </p>
                    <p className="truncate text-muted-foreground text-xs">
                      绑定用户: {bot.bound_user_id}
                    </p>
                    <span
                      className={
                        bot.runtime_status === "running"
                          ? "inline-flex rounded bg-green-100 px-1.5 py-0.5 text-green-700 text-xs"
                          : bot.runtime_status === "error"
                            ? "inline-flex rounded bg-red-100 px-1.5 py-0.5 text-red-700 text-xs"
                            : "inline-flex rounded bg-gray-100 px-1.5 py-0.5 text-gray-700 text-xs"
                      }
                    >
                      {bot.runtime_status === "running"
                        ? "运行中"
                        : bot.runtime_status === "error"
                          ? "异常"
                          : "已停止"}
                    </span>
                  </div>
                  <div className="flex shrink-0 items-center gap-1">
                    {bot.runtime_status === "running" ? (
                      <Button
                        type="button"
                        size="icon-xs"
                        variant="ghost"
                        disabled={stopBot.isPending}
                        onClick={() => stopBot.mutate({ botId: bot.bot_id })}
                      >
                        <Power className="size-4 text-red-500" />
                      </Button>
                    ) : (
                      <Button
                        type="button"
                        size="icon-xs"
                        variant="ghost"
                        disabled={startBot.isPending}
                        onClick={() => startBot.mutate({ botId: bot.bot_id })}
                      >
                        <Play className="size-4 text-green-500" />
                      </Button>
                    )}
                    <Button
                      type="button"
                      size="icon-xs"
                      variant="ghost"
                      disabled={deleteBot.isPending}
                      onClick={() => deleteBot.mutate({ botId: bot.bot_id })}
                    >
                      <Trash2 className="size-4 text-destructive" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default DashboardView;
