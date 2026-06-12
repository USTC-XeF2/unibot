import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { confirmDialog } from "@/lib/modal";
import { checkWriteQuery, useExecuteSqlMutation } from "@/lib/query";
import type { SqlQueryResult } from "@/types/dev-tools";

export function SqlPanel() {
  const [query, setQuery] = useState("SELECT * FROM im_accounts LIMIT 10");
  const [allowWrite, setAllowWrite] = useState(false);
  const [result, setResult] = useState<SqlQueryResult | null>(null);
  const execute = useExecuteSqlMutation();

  const handleExecute = async () => {
    execute.reset();

    const trimmed = query.trim();
    if (!trimmed) {
      toast.error("SQL 为空");
      return;
    }

    // Backend is the authoritative source for write detection. This call is
    // only used to decide whether to show the confirmation dialog.
    const isWrite = await checkWriteQuery(trimmed);

    if (isWrite && !allowWrite) {
      toast.error("写操作需在上方开启“允许写操作”");
      return;
    }

    if (isWrite && allowWrite) {
      const confirmed = await confirmDialog({
        title: "确认执行写操作",
        description: "该 SQL 可能修改数据库，确定继续？",
        confirmText: "执行",
      });
      if (!confirmed) return;
    }

    execute.mutate(
      { query: trimmed, allowWrite },
      {
        onSuccess: (data) => {
          setResult(data);
          toast.success("执行成功");
        },
        onError: (err) => {
          setResult(null);
          toast.error(`执行失败: ${err}`);
        },
      },
    );
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-hidden">
      <div className="shrink-0 rounded-xl border bg-card/60 p-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Switch
              checked={allowWrite}
              onCheckedChange={setAllowWrite}
              id="allow-write"
            />
            <label htmlFor="allow-write" className="text-sm">
              允许写操作
            </label>
          </div>
          <Button
            onClick={handleExecute}
            disabled={execute.isPending}
            size="sm"
          >
            {execute.isPending ? "执行中..." : "执行"}
          </Button>
        </div>

        <Textarea
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="mt-3 min-h-24 font-mono text-sm"
          placeholder="输入 SQL..."
        />
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-xl border bg-card/60 p-3">
        {execute.isError && (
          <div className="mb-3 rounded border border-destructive/30 bg-destructive/10 p-2 text-destructive text-xs">
            执行失败: {execute.error?.message || "未知错误"}
          </div>
        )}

        {!result && !execute.isError && (
          <p className="text-muted-foreground text-sm">
            执行 SQL 后在此查看结果
          </p>
        )}

        {result && (
          <>
            {result.rows_affected !== undefined && (
              <p className="mb-2 text-muted-foreground text-sm">
                受影响行数: {result.rows_affected}
              </p>
            )}

            {result.rows.length === 0 ? (
              <p className="text-muted-foreground text-sm">无返回数据</p>
            ) : (
              <table className="w-full text-xs">
                <thead className="sticky top-0 bg-muted">
                  <tr>
                    {result.columns.map((col) => (
                      <th key={col} className="px-3 py-2 text-left font-medium">
                        {col}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody className="divide-y">
                  {result.rows.map((row, ridx) => (
                    // biome-ignore lint/suspicious/noArrayIndexKey: SQL rows are read-only and may contain duplicate values
                    <tr key={`row-${ridx}`}>
                      {row.map((cell, cidx) => {
                        const colName = result.columns[cidx];
                        return (
                          <td key={colName} className="px-3 py-2">
                            {cell === null ? "NULL" : String(cell)}
                          </td>
                        );
                      })}
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </>
        )}
      </div>
    </div>
  );
}
