import { ArrowDownToLine, ArrowUpFromLine, Network } from "lucide-react";
import { useState } from "react";
import {
  useProtocolPacketDetail,
  useProtocolPackets,
} from "@/lib/query/packets";
import type { ProtocolPacket } from "@/types/packet";

function PacketDirectionBadge({ direction }: { direction: string }) {
  const isReceive = direction === "receive";
  return (
    <span
      className={`inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-xs font-medium ${
        isReceive
          ? "border-sky-500/30 bg-sky-500/10 text-sky-600"
          : "border-green-500/30 bg-green-500/10 text-green-600"
      }`}
    >
      {isReceive ? (
        <ArrowDownToLine className="size-3" />
      ) : (
        <ArrowUpFromLine className="size-3" />
      )}
      {isReceive ? "接收" : "发送"}
    </span>
  );
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export default function LogsView() {
  const [selectedPacketId, setSelectedPacketId] = useState<string | null>(null);
  const { data: packets, isLoading } = useProtocolPackets({ limit: 100 });
  const { data: packetJson, isLoading: detailLoading } =
    useProtocolPacketDetail(selectedPacketId ?? "");

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-hidden">
      <div className="space-y-3 rounded-xl border bg-card/60 p-3">
        <div className="flex items-center gap-2 text-sm">
          <Network className="size-4" />
          <span className="font-medium">协议报文</span>
          <span className="text-muted-foreground text-xs">
            {packets?.length ?? 0} 条记录
          </span>
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border bg-card/60">
        {isLoading ? (
          <div className="flex flex-1 items-center justify-center text-muted-foreground text-sm">
            加载中...
          </div>
        ) : (
          <div className="flex h-full min-h-0">
            {/* 左侧列表 */}
            <div className="flex min-h-0 w-2/3 flex-col border-r">
              <div className="grid grid-cols-[140px_80px_1fr_120px] gap-2 border-b px-3 py-2 text-muted-foreground text-xs">
                <span>时间</span>
                <span>方向</span>
                <span>动作</span>
                <span>Bot</span>
              </div>
              <div className="min-h-0 flex-1 overflow-auto">
                {packets && packets.length > 0 ? (
                  packets.map((packet: ProtocolPacket) => (
                    <button
                      key={packet.packet_id}
                      type="button"
                      onClick={() => setSelectedPacketId(packet.packet_id)}
                      className={`grid w-full grid-cols-[140px_80px_1fr_120px] gap-2 border-b px-3 py-2 text-left text-xs transition-colors hover:bg-muted/40 ${
                        selectedPacketId === packet.packet_id ? "bg-muted" : ""
                      }`}
                    >
                      <span className="text-muted-foreground">
                        {formatTime(packet.created_at)}
                      </span>
                      <span>
                        <PacketDirectionBadge direction={packet.direction} />
                      </span>
                      <span className="truncate">{packet.action_name}</span>
                      <span className="truncate text-muted-foreground">
                        {packet.bot_id.slice(0, 8)}...
                      </span>
                    </button>
                  ))
                ) : (
                  <div className="flex flex-1 items-center justify-center py-12 text-muted-foreground text-sm">
                    暂无报文记录
                  </div>
                )}
              </div>
            </div>

            {/* 右侧详情 */}
            <div className="flex min-h-0 w-1/3 flex-col">
              <div className="border-b px-3 py-2 text-muted-foreground text-xs">
                报文详情
              </div>
              <div className="min-h-0 flex-1 overflow-auto p-3">
                {selectedPacketId ? (
                  detailLoading ? (
                    <div className="flex items-center justify-center py-12 text-muted-foreground text-sm">
                      读取中...
                    </div>
                  ) : packetJson ? (
                    <pre className="whitespace-pre-wrap break-all rounded border bg-muted/30 p-3 text-xs leading-relaxed">
                      {packetJson}
                    </pre>
                  ) : (
                    <div className="flex items-center justify-center py-12 text-muted-foreground text-sm">
                      无法读取报文内容
                    </div>
                  )
                ) : (
                  <div className="flex items-center justify-center py-12 text-muted-foreground text-sm">
                    选择一条报文查看详情
                  </div>
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
