import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";

/**
 * 群内容（文件/文件夹/相册/照片）查询失败时的内联错误面板。
 * 在渲染空集合之前展示，避免把"加载失败"误显示成"暂无内容"，
 * 并提供重试入口。
 */
export function GroupContentError({
  message,
  onRetry,
}: {
  message: string;
  onRetry: () => void;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
      <AlertTriangle className="size-8 text-destructive" />
      <p className="max-w-md break-words text-muted-foreground text-sm">
        {message}
      </p>
      <Button variant="outline" size="sm" onClick={onRetry}>
        重试
      </Button>
    </div>
  );
}
