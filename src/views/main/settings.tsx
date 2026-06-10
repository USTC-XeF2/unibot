import { Database, FileText, Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Field, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  useSetLogLevelMutation,
  useSetLogRetentionMutation,
  useTriggerLogCleanupMutation,
} from "@/lib/mutations";
import {
  invalidateDbStatusQuery,
  invalidateLogSettingsQuery,
  useDbStatusQuery,
  useLogSettingsQuery,
} from "@/lib/query";

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(
    Math.floor(Math.log(bytes) / Math.log(k)),
    sizes.length - 1,
  );
  return `${parseFloat((bytes / k ** i).toFixed(2))} ${sizes[i]}`;
}

function SettingsView() {
  const dbStatus = useDbStatusQuery();
  const dbData = dbStatus.data;

  const logSettings = useLogSettingsQuery();
  const logData = logSettings.data;

  const setLogLevel = useSetLogLevelMutation();
  const setRetention = useSetLogRetentionMutation();
  const triggerCleanup = useTriggerLogCleanupMutation();

  const [pendingDebug, setPendingDebug] = useState<boolean | null>(null);
  const [pendingRetention, setPendingRetention] = useState<number | null>(null);

  const isDebugEnabled =
    pendingDebug ?? (logData?.level === "debug" || logData?.level === "trace");
  const currentRetention = pendingRetention ?? logData?.retention_days ?? 7;

  const handleDebugToggle = (checked: boolean) => {
    setPendingDebug(checked);
    const level = checked ? "debug" : "info";
    setLogLevel.mutate(
      { level },
      {
        onSuccess: () => {
          invalidateLogSettingsQuery();
          toast.success(checked ? "已开启 DEBUG 模式" : "已关闭 DEBUG 模式");
        },
        onError: (err) => {
          toast.error(`切换失败: ${err}`);
          invalidateLogSettingsQuery();
        },
      },
    );
  };

  const handleRetentionChange = (value: string) => {
    const days = Number(value);
    if (days === currentRetention) return;
    setPendingRetention(days);
    setRetention.mutate(
      { days },
      {
        onSuccess: () => {
          invalidateLogSettingsQuery();
          toast.success(
            `日志保留天数已设为 ${days === 0 ? "永不过期" : `${days} 天`}`,
          );
        },
        onError: (err) => {
          toast.error(`保存失败: ${err}`);
          invalidateLogSettingsQuery();
        },
      },
    );
  };

  const handleCleanup = () => {
    triggerCleanup.mutate(undefined, {
      onSuccess: (data) => {
        if (data.deleted_files > 0) {
          toast.success(`已清理 ${data.deleted_files} 个过期日志文件`);
        } else {
          toast.info("没有需要清理的过期日志文件");
        }
      },
      onError: (err) => {
        toast.error(`清理失败: ${err}`);
      },
    });
  };

  return (
    <div className="space-y-4">
      <h1 className="font-semibold text-xl">设置</h1>

      <Card>
        <CardContent className="space-y-3 pt-4">
          <div className="flex items-center gap-2 text-sm font-medium">
            <Database className="size-4" />
            数据库状态
          </div>

          {dbStatus.isPending ? (
            <p className="text-muted-foreground text-sm">读取中...</p>
          ) : dbStatus.isError ? (
            <p className="text-destructive text-sm">
              读取失败:{" "}
              {dbStatus.error instanceof Error
                ? dbStatus.error.message
                : String(dbStatus.error)}
            </p>
          ) : dbData ? (
            <>
              <div className="grid grid-cols-2 gap-3 text-sm">
                <div>
                  <span className="text-muted-foreground">Schema 版本</span>
                  <p className="font-medium">{dbData.schema_version}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">表数量</span>
                  <p className="font-medium">{dbData.table_count}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">数据库大小</span>
                  <p className="font-medium">
                    {formatBytes(dbData.db_size_bytes)}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <span
                    className={
                      dbData.integrity_check === "ok"
                        ? "text-green-600"
                        : "text-destructive"
                    }
                  >
                    {dbData.integrity_check === "ok"
                      ? "完整性正常"
                      : `完整性异常: ${dbData.integrity_check}`}
                  </span>
                </div>
              </div>

              {dbData.foreign_key_check.length > 0 && (
                <div className="rounded border border-destructive/30 bg-destructive/10 p-2 text-destructive text-xs">
                  外键约束异常: {dbData.foreign_key_check.join(", ")}
                </div>
              )}

              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => invalidateDbStatusQuery()}
              >
                刷新状态
              </Button>
            </>
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardContent className="space-y-4 pt-4">
          <div className="flex items-center gap-2 text-sm font-medium">
            <FileText className="size-4" />
            日志设置
          </div>

          {logSettings.isPending ? (
            <p className="text-muted-foreground text-sm">读取中...</p>
          ) : logSettings.isError ? (
            <p className="text-destructive text-sm">
              读取失败:{" "}
              {logSettings.error instanceof Error
                ? logSettings.error.message
                : String(logSettings.error)}
            </p>
          ) : logData ? (
            <>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="font-medium text-sm">DEBUG 模式</p>
                  <p className="text-muted-foreground text-xs">
                    开启后将记录 DEBUG 级别日志（大量调试信息）
                  </p>
                </div>
                <Switch
                  checked={isDebugEnabled}
                  onCheckedChange={handleDebugToggle}
                  disabled={setLogLevel.isPending}
                />
              </div>

              <Field>
                <FieldLabel>日志保留天数</FieldLabel>
                <Select
                  value={String(currentRetention)}
                  onValueChange={handleRetentionChange}
                  disabled={setRetention.isPending}
                >
                  <SelectTrigger className="w-full md:w-48">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent position="popper" align="start">
                    <SelectItem value="1">1 天</SelectItem>
                    <SelectItem value="3">3 天</SelectItem>
                    <SelectItem value="7">7 天</SelectItem>
                    <SelectItem value="14">14 天</SelectItem>
                    <SelectItem value="30">30 天</SelectItem>
                    <SelectItem value="0">永不过期</SelectItem>
                  </SelectContent>
                </Select>
              </Field>

              <Button
                type="button"
                variant="outline"
                size="sm"
                className="gap-1.5"
                onClick={handleCleanup}
                disabled={triggerCleanup.isPending}
              >
                <Trash2 className="size-3.5" />
                立即清理过期日志
              </Button>
            </>
          ) : null}
        </CardContent>
      </Card>
    </div>
  );
}

export default SettingsView;
