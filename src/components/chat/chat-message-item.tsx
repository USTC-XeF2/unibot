import Face from "@/components/chat/face";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { formatMessageTimestamp } from "@/lib/time-format";
import { cn } from "@/lib/utils";
import type { ChatMessage, MessageSegment } from "@/types/chat";
import type { GroupRole } from "@/types/group";

export type ChatContextAction = {
  key: string;
  label: string;
  variant?: "default" | "destructive";
  separatorBefore?: boolean;
  onSelect: () => void;
};

type ChatMessageContentProps = {
  content: MessageSegment[];
  className?: string;
  onAtClick: (target: number | "all") => void;
  resolveMemberName: (userId: number) => string;
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
  quotedMessagePreview?: {
    senderDisplayName: string;
    summary: string;
    missing?: boolean;
  } | null;
  onAtClick: (target: number | "all") => void;
  resolveMemberName: (userId: number) => string;
  avatarActions: ChatContextAction[];
  messageActions: ChatContextAction[];
};

function renderFallbackSegment(label: string, key: string) {
  return (
    <span
      key={key}
      className="mx-0.5 inline-flex items-center rounded-md border bg-muted/40 px-1.5 py-0.5 text-[12px] text-muted-foreground leading-none"
    >
      {label}
    </span>
  );
}

function ChatMessageContent({
  content,
  className,
  onAtClick,
  resolveMemberName,
}: ChatMessageContentProps) {
  return (
    <div
      className={cn(
        "wrap-break-word select-text whitespace-pre-wrap text-sm leading-relaxed",
        className,
      )}
    >
      {content.map((segment, index) => {
        const key = `${segment.type}-${index}`;

        switch (segment.type) {
          case "Text":
            return <span key={key}>{segment.data.text}</span>;
          case "At":
            return (
              <button
                key={key}
                type="button"
                className="cursor-pointer text-sky-600 dark:text-sky-300"
                onClick={() => onAtClick(segment.data.target)}
              >
                @{resolveMemberName(segment.data.target)}
              </button>
            );
          case "AtAll":
            return (
              <button
                key={key}
                type="button"
                className="cursor-pointer text-sky-600 dark:text-sky-300"
                onClick={() => onAtClick("all")}
              >
                @全体成员
              </button>
            );
          case "Face": {
            return (
              <Face
                key={key}
                id={segment.data.id}
                className="size-5.5 px-0.5 pb-0.5"
              />
            );
          }
          case "Image":
            return renderFallbackSegment("[图片]", key);
          default:
            return null;
        }
      })}
    </div>
  );
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
  quotedMessagePreview,
  onAtClick,
  resolveMemberName,
  avatarActions,
  messageActions,
}: ChatMessageItemProps) {
  const messageTime = formatMessageTimestamp(message.created_at);
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
              {quotedMessagePreview ? (
                <div className="mb-2 rounded-md border border-border/70 bg-muted/40 px-2 py-1">
                  <p className="truncate text-[11px] text-muted-foreground">
                    {quotedMessagePreview.senderDisplayName}
                  </p>
                  <p className="truncate text-[12px] leading-4">
                    {quotedMessagePreview.missing
                      ? "[引用消息不可用]"
                      : quotedMessagePreview.summary}
                  </p>
                </div>
              ) : null}
              <ChatMessageContent
                content={message.content}
                onAtClick={onAtClick}
                resolveMemberName={resolveMemberName}
              />
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
