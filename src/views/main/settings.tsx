import { Database, Shield } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { invalidateDbStatusQuery, useDbStatusQuery } from "@/lib/query";

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
  const data = dbStatus.data;

  return (
    <div className="space-y-4">
      <h1 className="font-semibold text-xl">设置</h1>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2 text-sm">
            <Database className="size-4" />
            数据库状态
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {dbStatus.isPending ? (
            <p className="text-muted-foreground text-sm">读取中...</p>
          ) : dbStatus.isError ? (
            <p className="text-destructive text-sm">
              读取失败: {String(dbStatus.error)}
            </p>
          ) : data ? (
            <>
              <div className="grid grid-cols-2 gap-3 text-sm">
                <div>
                  <span className="text-muted-foreground">Schema 版本</span>
                  <p className="font-medium">{data.schema_version}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">表数量</span>
                  <p className="font-medium">{data.table_count}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">数据库大小</span>
                  <p className="font-medium">
                    {formatBytes(data.db_size_bytes)}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <Shield className="size-4" />
                  <span
                    className={
                      data.integrity_check === "ok"
                        ? "text-green-600"
                        : "text-destructive"
                    }
                  >
                    {data.integrity_check === "ok"
                      ? "完整性正常"
                      : `完整性异常: ${data.integrity_check}`}
                  </span>
                </div>
              </div>

              {data.foreign_key_check.length > 0 && (
                <div className="rounded border border-destructive/30 bg-destructive/10 p-2 text-destructive text-xs">
                  外键约束异常: {data.foreign_key_check.join(", ")}
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
    </div>
  );
}

export default SettingsView;
