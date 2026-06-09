import {
  Bot,
  Check,
  Copy,
  Eye,
  MessageCircle,
  Pencil,
  Play,
  Plus,
  Power,
  SquareUser,
  Trash2,
  Users,
  X,
} from "lucide-react";
import { useState } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
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
  useRenameBotMutation,
  useStartBotMutation,
  useStopBotMutation,
} from "@/lib/mutations";
import {
  useBotConfigQuery,
  useBotStatsQuery,
  useBotsQuery,
  useGroupsQuery,
  useUsersQuery,
} from "@/lib/query";
import type { BotProfile } from "@/types/bot";
import type { UserProfile } from "@/types/user";

const botStatusConfig = {
  running: {
    label: "运行中",
    className:
      "inline-flex rounded bg-green-100 px-1.5 py-0.5 text-green-700 text-xs",
  },
  error: {
    label: "异常",
    className:
      "inline-flex rounded bg-red-100 px-1.5 py-0.5 text-red-700 text-xs",
  },
  stopped: {
    label: "已停止",
    className:
      "inline-flex rounded bg-gray-100 px-1.5 py-0.5 text-gray-700 text-xs",
  },
} satisfies Record<
  BotProfile["runtime_status"],
  { label: string; className: string }
>;

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
  const [displayName, setDisplayName] = useState("");
  const [submitError, setSubmitError] = useState<string | null>(null);

  const unboundUsers = users.filter((user) => !boundUserIds.has(user.user_id));
  const selectedUser = users.find((user) => user.user_id === selectedUserId);
  const selectedUserLabel =
    selectedUser?.nickname.trim() || selectedUser?.user_id || "?";

  const resetForm = () => {
    setSelectedUserId("");
    setDisplayName("");
    setSubmitError(null);
  };

  const handleOpenChange = (nextOpen: boolean) => {
    onOpenChange(nextOpen);
    if (!nextOpen) {
      resetForm();
    }
  };

  const handleSelectUser = (value: string) => {
    setSelectedUserId(value);
    const user = users.find((u) => u.user_id === value);
    if (user) {
      setDisplayName(user.nickname.trim());
    }
    setSubmitError(null);
  };

  const handleCreateBot = () => {
    if (!selectedUser) {
      setSubmitError("请选择要绑定的用户");
      return;
    }
    const name = displayName.trim();
    if (!name) {
      setSubmitError("Bot 名称不能为空");
      return;
    }

    setSubmitError(null);
    createBot.mutate(
      {
        boundUserId: selectedUser.user_id,
        displayName: name,
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
              onValueChange={handleSelectUser}
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

          <Field>
            <FieldLabel htmlFor="create-bot-name">Bot 名称</FieldLabel>
            <Input
              id="create-bot-name"
              value={displayName}
              onChange={(e) => {
                setDisplayName(e.target.value);
                setSubmitError(null);
              }}
              placeholder="输入 Bot 显示名称"
              disabled={!selectedUserId}
            />
          </Field>

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

type BotCardProps = {
  bot: BotProfile;
  onStart: (botId: string) => void;
  onStop: (botId: string) => void;
  onDelete: (botId: string) => void;
  isStartPending: boolean;
  isStopPending: boolean;
  isDeletePending: boolean;
};

function BotConfigSheet({
  botId,
  displayName,
}: {
  botId: string;
  displayName: string;
}) {
  const { data: configJson, isLoading } = useBotConfigQuery(botId);
  const [copied, setCopied] = useState(false);

  const config = configJson
    ? (JSON.parse(configJson) as {
        protocol: string;
        http: { host: string; port: number };
        access_token: string;
        event_transport: string;
      })
    : null;

  const baseUrl = config
    ? `http://${config.http.host}:${config.http.port}`
    : "";

  const curlExample = config
    ? `curl -X POST "${baseUrl}/api/get_login_info?access_token=${config.access_token}" \\\n     -H "Content-Type: application/json" \\\n     -d '{}'`
    : "";

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <Sheet>
      <SheetTrigger asChild>
        <Button type="button" size="icon-xs" variant="ghost">
          <Eye className="size-4 text-muted-foreground" />
        </Button>
      </SheetTrigger>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>{displayName} 的配置</SheetTitle>
        </SheetHeader>

        <div className="mt-4 space-y-4 px-4">
          {isLoading ? (
            <p className="text-muted-foreground text-sm">读取中...</p>
          ) : config ? (
            <>
              <div className="space-y-2 text-sm">
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">协议</span>
                  <span className="font-medium">{config.protocol}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">地址</span>
                  <span className="font-medium">{baseUrl}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">传输</span>
                  <span className="font-medium">{config.event_transport}</span>
                </div>
                <div className="flex items-center justify-between gap-2">
                  <span className="text-muted-foreground shrink-0">Token</span>
                  <code className="truncate rounded bg-muted px-1.5 py-0.5 text-xs">
                    {config.access_token}
                  </code>
                </div>
              </div>

              <Button
                type="button"
                variant="outline"
                size="sm"
                className="w-full"
                onClick={() => handleCopy(curlExample)}
              >
                {copied ? (
                  <Check className="mr-1 size-3" />
                ) : (
                  <Copy className="mr-1 size-3" />
                )}
                {copied ? "已复制" : "复制 curl 示例"}
              </Button>

              <div>
                <p className="mb-1 text-muted-foreground text-xs">
                  完整配置 JSON
                </p>
                <pre className="max-h-64 overflow-auto whitespace-pre-wrap break-all rounded border bg-muted/30 p-3 text-xs leading-relaxed">
                  {configJson}
                </pre>
              </div>
            </>
          ) : (
            <p className="text-muted-foreground text-sm">无法读取配置</p>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

function BotCard({
  bot,
  onStart,
  onStop,
  onDelete,
  isStartPending,
  isStopPending,
  isDeletePending,
}: BotCardProps) {
  const statusConfig = botStatusConfig[bot.runtime_status];
  const renameBot = useRenameBotMutation();
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(bot.display_name);

  const handleRename = () => {
    const name = editName.trim();
    if (!name || name === bot.display_name) {
      setIsEditing(false);
      setEditName(bot.display_name);
      return;
    }
    renameBot.mutate(
      { botId: bot.bot_id, displayName: name },
      {
        onSuccess: () => setIsEditing(false),
        onError: () => setEditName(bot.display_name),
      },
    );
  };

  const handleCancelEdit = () => {
    setIsEditing(false);
    setEditName(bot.display_name);
  };

  return (
    <div className="flex items-center justify-between gap-3 rounded-lg border p-3">
      <div className="min-w-0 flex-1 space-y-1">
        {isEditing ? (
          <div className="flex items-center gap-2">
            <Input
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              className="h-7 text-sm"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter") handleRename();
                if (e.key === "Escape") handleCancelEdit();
              }}
            />
            <Button
              type="button"
              size="icon-xs"
              variant="ghost"
              onClick={handleRename}
              disabled={renameBot.isPending}
            >
              <Check className="size-3 text-green-600" />
            </Button>
            <Button
              type="button"
              size="icon-xs"
              variant="ghost"
              onClick={handleCancelEdit}
            >
              <X className="size-3 text-red-500" />
            </Button>
          </div>
        ) : (
          <div className="flex items-center gap-2">
            <p className="truncate font-medium text-sm">{bot.display_name}</p>
            <Button
              type="button"
              size="icon-xs"
              variant="ghost"
              className="opacity-0 group-hover:opacity-100"
              onClick={() => {
                setEditName(bot.display_name);
                setIsEditing(true);
              }}
            >
              <Pencil className="size-3 text-muted-foreground" />
            </Button>
          </div>
        )}
        <p className="truncate text-muted-foreground text-xs">
          绑定用户: {bot.bound_user_id}
        </p>
        <span className={statusConfig.className}>{statusConfig.label}</span>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <BotConfigSheet botId={bot.bot_id} displayName={bot.display_name} />
        {bot.runtime_status === "running" ? (
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            disabled={isStopPending}
            onClick={() => onStop(bot.bot_id)}
          >
            <Power className="size-4 text-red-500" />
          </Button>
        ) : (
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            disabled={isStartPending}
            onClick={() => onStart(bot.bot_id)}
          >
            <Play className="size-4 text-green-500" />
          </Button>
        )}
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button
              type="button"
              size="icon-xs"
              variant="ghost"
              disabled={isDeletePending}
            >
              <Trash2 className="size-4 text-destructive" />
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent size="sm">
            <AlertDialogHeader>
              <AlertDialogTitle>删除 Bot</AlertDialogTitle>
              <AlertDialogDescription>
                {`将删除 ${bot.display_name}，并停止关联的调试会话。`}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={isDeletePending}>
                取消
              </AlertDialogCancel>
              <AlertDialogAction
                variant="destructive"
                disabled={isDeletePending}
                onClick={() => onDelete(bot.bot_id)}
              >
                删除
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </div>
    </div>
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
                <BotCard
                  key={bot.bot_id}
                  bot={bot}
                  onStart={(botId) => startBot.mutate({ botId })}
                  onStop={(botId) => stopBot.mutate({ botId })}
                  onDelete={(botId) => deleteBot.mutate({ botId })}
                  isStartPending={startBot.isPending}
                  isStopPending={stopBot.isPending}
                  isDeletePending={deleteBot.isPending}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default DashboardView;
