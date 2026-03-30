import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { cn } from "@/lib/utils";
import type { ChatMessage } from "@/types/chat";
import type { GroupRole } from "@/types/group";

export type ChatContextAction = {
  key: string;
  label: string;
  variant?: "default" | "destructive";
  separatorBefore?: boolean;
  onSelect: () => void;
};

type ChatMessageItemProps = {
  isSelf: boolean;
  avatarUrl?: string;
  avatarFallback: string;
  senderDisplayName: string;
  senderRole?: GroupRole;
  senderTitle?: string;
  showSenderName: boolean;
  message: ChatMessage;
  avatarActions: ChatContextAction[];
  messageActions: ChatContextAction[];
};

function extractMessageText(message: ChatMessage): string {
  const parts = message.content
    .map((segment) => {
      if (segment.type === "Text") {
        return segment.data?.text ?? "";
      }
      return `[${segment.type}]`;
    })
    .filter(Boolean);

  return parts.join("") || "[空消息]";
}

function formatMessageTime(ts: number): string {
  if (!ts) {
    return "";
  }
  const date = new Date(ts * 1000);
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();

  if (sameDay) {
    return date.toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  const datePart = date.toLocaleDateString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
  });
  const timePart = date.toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });

  return `${datePart} ${timePart}`;
}

function ChatMessageItem({
  isSelf,
  avatarUrl,
  avatarFallback,
  senderDisplayName,
  senderRole,
  senderTitle,
  showSenderName,
  message,
  avatarActions,
  messageActions,
}: ChatMessageItemProps) {
  const messageText = extractMessageText(message);
  const messageTime = formatMessageTime(message.created_at);
  const roleLabel =
    senderRole === "owner" ? "群主" : senderRole === "admin" ? "管理员" : null;
  const cleanTitle = senderTitle?.trim() ?? "";
  const badgeLabel = cleanTitle || roleLabel;
  const badgeClass = cleanTitle
    ? senderRole === "owner"
      ? "border-amber-300/70 bg-amber-100/60 text-amber-800"
      : senderRole === "admin"
        ? "border-blue-300/70 bg-blue-100/60 text-blue-800"
        : "border-border bg-muted/50 text-muted-foreground"
    : senderRole === "owner"
      ? "border-amber-300/70 bg-amber-100/60 text-amber-800"
      : senderRole === "admin"
        ? "border-blue-300/70 bg-blue-100/60 text-blue-800"
        : "";

  const avatarNode = (
    <Avatar className="size-9">
      <AvatarImage src={avatarUrl} alt={senderDisplayName} />
      <AvatarFallback>{avatarFallback}</AvatarFallback>
    </Avatar>
  );

  const avatarWithMenu = (
    <ContextMenu>
      <ContextMenuTrigger>{avatarNode}</ContextMenuTrigger>
      <ContextMenuContent>
        {avatarActions.map((action) => (
          <div key={action.key}>
            {action.separatorBefore ? <ContextMenuSeparator /> : null}
            <ContextMenuItem
              variant={action.variant}
              onSelect={action.onSelect}
            >
              {action.label}
            </ContextMenuItem>
          </div>
        ))}
      </ContextMenuContent>
    </ContextMenu>
  );

  return (
    <div
      className={cn(
        "flex items-start gap-2",
        isSelf ? "justify-end" : "justify-start",
      )}
    >
      {!isSelf ? avatarWithMenu : null}
      <div className="max-w-[50%]">
        {showSenderName ? (
          <div
            className={cn(
              "mb-1 flex items-center gap-1 text-[11px]",
              isSelf ? "justify-end" : "justify-start",
            )}
          >
            <p className="text-muted-foreground">{senderDisplayName}</p>
            {badgeLabel ? (
              <span
                className={cn(
                  "rounded border px-1 py-0.5 text-[10px] leading-none",
                  badgeClass,
                )}
              >
                {badgeLabel}
              </span>
            ) : null}
          </div>
        ) : null}
        <ContextMenu>
          <ContextMenuTrigger
            className={cn("select-text", isSelf && "block w-full")}
          >
            <div
              className={cn(
                "w-fit max-w-full rounded-lg border px-3 py-2 text-sm",
                message.recall.recalled
                  ? "border-destructive/50"
                  : isSelf
                    ? "border-primary/30 bg-primary/10"
                    : "border-border bg-card",
                isSelf && "ml-auto",
              )}
            >
              <p className="wrap-break-word select-text whitespace-pre-wrap">
                {messageText}
              </p>
            </div>
          </ContextMenuTrigger>
          <ContextMenuContent>
            {messageActions.map((action) => (
              <div key={action.key}>
                {action.separatorBefore ? <ContextMenuSeparator /> : null}
                <ContextMenuItem
                  variant={action.variant}
                  onSelect={action.onSelect}
                >
                  {action.label}
                </ContextMenuItem>
              </div>
            ))}
          </ContextMenuContent>
        </ContextMenu>
        <p
          className={cn(
            "mt-1 text-[11px] text-muted-foreground",
            isSelf ? "text-right" : "text-left",
          )}
        >
          {messageTime}
        </p>
      </div>
      {isSelf ? avatarWithMenu : null}
    </div>
  );
}

export default ChatMessageItem;
