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
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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

  const createBot = useCreateBotMutation();
  const deleteBot = useDeleteBotMutation();
  const startBot = useStartBotMutation();
  const stopBot = useStopBotMutation();

  const [selectedUserId, setSelectedUserId] = useState("");
  const [sheetOpen, setSheetOpen] = useState(false);

  const users = usersQuery.data ?? [];
  const bots = botsQuery.data ?? [];
  const stats = statsQuery.data;
  const unboundUsers = users.filter(
    (user) => !bots.some((bot) => bot.bound_user_id === user.user_id),
  );

  const handleCreateBot = () => {
    const user = users.find((item) => item.user_id === selectedUserId);
    if (!user) {
      return;
    }

    createBot.mutate(
      {
        boundUserId: selectedUserId,
        displayName: user.nickname,
      },
      {
        onSuccess: () => {
          setSelectedUserId("");
          setSheetOpen(false);
        },
      },
    );
  };

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
          <Sheet open={sheetOpen} onOpenChange={setSheetOpen}>
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
              <div className="mt-4 space-y-4">
                <div className="space-y-2">
                  <p className="font-medium text-sm">选择用户</p>
                  <Select
                    value={selectedUserId}
                    onValueChange={setSelectedUserId}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="选择要绑定的用户" />
                    </SelectTrigger>
                    <SelectContent>
                      {unboundUsers.map((user) => (
                        <SelectItem key={user.user_id} value={user.user_id}>
                          {user.nickname} ({user.user_id})
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <Button
                  type="button"
                  className="w-full"
                  disabled={!selectedUserId || createBot.isPending}
                  onClick={handleCreateBot}
                >
                  {createBot.isPending ? "创建中..." : "确认创建"}
                </Button>
              </div>
            </SheetContent>
          </Sheet>
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
