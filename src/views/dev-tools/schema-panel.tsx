import { useState } from "react";
import { useDbSchemaQuery, useTableRowPreviewQuery } from "@/lib/query";
import type { DbTable } from "@/types/dev-tools";

function TableDetail({ table }: { table: DbTable }) {
  return (
    <div className="space-y-3">
      <div>
        <h3 className="font-semibold text-sm">列</h3>
        <table className="w-full text-sm">
          <thead className="bg-muted">
            <tr>
              <th className="px-2 py-1 text-left">名</th>
              <th className="px-2 py-1 text-left">类型</th>
              <th className="px-2 py-1 text-left">非空</th>
              <th className="px-2 py-1 text-left">默认值</th>
              <th className="px-2 py-1 text-left">PK</th>
            </tr>
          </thead>
          <tbody>
            {table.columns.map((col) => (
              <tr key={col.name} className="border-t">
                <td className="px-2 py-1">{col.name}</td>
                <td className="px-2 py-1">{col.type_name}</td>
                <td className="px-2 py-1">{col.not_null ? "是" : ""}</td>
                <td className="px-2 py-1">{col.default_value ?? "-"}</td>
                <td className="px-2 py-1">{col.primary_key ? "是" : ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {table.indexes.length > 0 && (
        <div>
          <h3 className="font-semibold text-sm">索引</h3>
          <ul className="list-disc pl-5 text-sm">
            {table.indexes.map((idx) => (
              <li key={idx.name}>
                {idx.name} ({idx.unique ? "唯一" : "非唯一"}) →{" "}
                {idx.columns.join(", ")}
              </li>
            ))}
          </ul>
        </div>
      )}

      {table.sql && (
        <div>
          <h3 className="font-semibold text-sm">DDL</h3>
          <pre className="rounded bg-muted p-2 text-xs">{table.sql}</pre>
        </div>
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
    <div className="space-y-2">
      <h3 className="font-semibold text-sm">
        数据预览 (前 {data.rows.length} 行)
      </h3>
      <div className="overflow-auto rounded border">
        <table className="w-full text-sm">
          <thead className="bg-muted">
            <tr>
              {data.columns.map((col) => (
                <th key={col} className="whitespace-nowrap px-2 py-1 text-left">
                  {col}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {data.rows.map((row) => (
              <tr key={JSON.stringify(row)} className="border-t">
                {row.map((cell, j) => (
                  <td
                    key={`${tableName}-${JSON.stringify(row)}-${data.columns[j]}`}
                    className="whitespace-nowrap px-2 py-1"
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
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

export function SchemaPanel() {
  const schemaQuery = useDbSchemaQuery();
  const [selectedTable, setSelectedTable] = useState<string | null>(null);

  const selected = schemaQuery.data?.tables.find(
    (t) => t.name === selectedTable,
  );

  return (
    <div className="flex h-full gap-4">
      <div className="w-56 overflow-auto rounded border">
        {schemaQuery.isPending ? (
          <p className="p-2 text-muted-foreground text-sm">读取中...</p>
        ) : schemaQuery.isError ? (
          <p className="p-2 text-destructive text-sm">读取失败</p>
        ) : (
          <ul className="divide-y text-sm">
            {schemaQuery.data?.tables.map((table) => (
              <li key={table.name}>
                <button
                  type="button"
                  className={`block w-full cursor-pointer px-3 py-2 text-left ${
                    selectedTable === table.name
                      ? "bg-muted"
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

      <div className="flex-1 space-y-4 overflow-auto rounded border p-3">
        {selected ? (
          <>
            <TableDetail table={selected} />
            <RowPreview tableName={selected.name} />
          </>
        ) : (
          <p className="text-muted-foreground text-sm">选择一个表查看结构</p>
        )}
      </div>
    </div>
  );
}
