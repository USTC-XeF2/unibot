import { useState } from "react";
import { useDbSchemaQuery, useTableRowPreviewQuery } from "@/lib/query";
import type { DbTable } from "@/types/dev-tools";

function TableDetail({ table }: { table: DbTable }) {
  return (
    <div className="space-y-4">
      <section>
        <h3 className="mb-2 font-semibold text-sm">列</h3>
        <div className="overflow-auto rounded-xl border bg-card">
          <table className="w-full text-sm">
            <thead className="bg-muted">
              <tr>
                <th className="px-3 py-2 text-left font-medium">名</th>
                <th className="px-3 py-2 text-left font-medium">类型</th>
                <th className="px-3 py-2 text-left font-medium">非空</th>
                <th className="px-3 py-2 text-left font-medium">默认值</th>
                <th className="px-3 py-2 text-left font-medium">PK</th>
              </tr>
            </thead>
            <tbody className="divide-y">
              {table.columns.map((col) => (
                <tr key={col.name}>
                  <td className="px-3 py-2">{col.name}</td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {col.type_name}
                  </td>
                  <td className="px-3 py-2">{col.not_null ? "是" : ""}</td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {col.default_value ?? "-"}
                  </td>
                  <td className="px-3 py-2">{col.primary_key ? "是" : ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {table.indexes.length > 0 && (
        <section>
          <h3 className="mb-2 font-semibold text-sm">索引</h3>
          <div className="space-y-2">
            {table.indexes.map((idx) => (
              <div
                key={idx.name}
                className="flex items-center gap-2 rounded-lg border bg-card px-3 py-2 text-sm"
              >
                <span className="font-medium">{idx.name}</span>
                <span className="rounded border bg-muted/40 px-1.5 py-0.5 text-[11px] text-muted-foreground">
                  {idx.unique ? "唯一" : "非唯一"}
                </span>
                <span className="text-muted-foreground">
                  → {idx.columns.join(", ")}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}

      {table.sql && (
        <section>
          <h3 className="mb-2 font-semibold text-sm">DDL</h3>
          <pre className="overflow-auto rounded-xl bg-muted p-3 text-xs">
            {table.sql}
          </pre>
        </section>
      )}
    </div>
  );
}

function RowPreview({ tableName }: { tableName: string }) {
  const previewQuery = useTableRowPreviewQuery(tableName, 50);

  if (previewQuery.isPending) {
    return <p className="text-muted-foreground text-sm">读取数据中...</p>;
  }

  if (previewQuery.isError) {
    return (
      <p className="text-destructive text-sm">
        预览失败: {String(previewQuery.error)}
      </p>
    );
  }

  const data = previewQuery.data;
  if (!data || data.columns.length === 0) {
    return <p className="text-muted-foreground text-sm">表为空</p>;
  }

  return (
    <section>
      <h3 className="mb-2 font-semibold text-sm">
        数据预览 (前 {data.rows.length} 行)
      </h3>
      <div className="overflow-auto rounded-xl border bg-card">
        <table className="w-full text-sm">
          <thead className="bg-muted">
            <tr>
              {data.columns.map((col) => (
                <th
                  key={col}
                  className="whitespace-nowrap px-3 py-2 text-left font-medium"
                >
                  {col}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y">
            {data.rows.map((row) => {
              const rowKey = row.map((c) => String(c)).join("|");
              return (
                <tr key={rowKey}>
                  {row.map((cell, j) => (
                    <td
                      key={data.columns[j]}
                      className="whitespace-nowrap px-3 py-2"
                    >
                      {cell === null
                        ? "NULL"
                        : typeof cell === "boolean"
                          ? cell
                            ? "true"
                            : "false"
                          : String(cell)}
                    </td>
                  ))}
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </section>
  );
}

export function SchemaPanel() {
  const schemaQuery = useDbSchemaQuery();
  const [selectedTable, setSelectedTable] = useState<string | null>(null);

  const selected = schemaQuery.data?.tables.find(
    (t) => t.name === selectedTable,
  );

  return (
    <div className="flex h-full min-h-0 gap-4">
      <div className="flex h-full w-56 flex-col overflow-hidden rounded-xl border bg-card">
        <div className="flex-1 overflow-auto">
          {schemaQuery.isPending ? (
            <p className="p-3 text-muted-foreground text-sm">读取中...</p>
          ) : schemaQuery.isError ? (
            <p className="p-3 text-destructive text-sm">读取失败</p>
          ) : schemaQuery.data?.tables.length === 0 ? (
            <p className="p-3 text-muted-foreground text-sm">无表</p>
          ) : (
            <ul className="divide-y">
              {schemaQuery.data?.tables.map((table) => (
                <li key={table.name}>
                  <button
                    type="button"
                    className={`block w-full cursor-pointer px-3 py-2 text-left text-sm ${
                      selectedTable === table.name
                        ? "bg-muted font-medium"
                        : "hover:bg-muted/50"
                    }`}
                    onClick={() => setSelectedTable(table.name)}
                  >
                    {table.name}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-xl border bg-card/60 p-4">
        {selected ? (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="font-semibold text-lg">{selected.name}</h2>
            </div>
            <TableDetail table={selected} />
            <RowPreview tableName={selected.name} />
          </div>
        ) : (
          <p className="text-muted-foreground text-sm">选择一个表查看结构</p>
        )}
      </div>
    </div>
  );
}
