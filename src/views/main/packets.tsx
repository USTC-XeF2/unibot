import { ArrowDownToLine, ArrowUpFromLine, Network } from "lucide-react";
import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
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

function PacketCard({
  packet,
  isSelected,
  onClick,
}: {
  packet: ProtocolPacket;
  isSelected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`w-full rounded-lg border p-3 text-left transition-colors hover:bg-muted/40 ${
        isSelected ? "bg-muted" : ""
      }`}
    >
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0 flex-1 space-y-1">
          <div className="flex items-center gap-2">
            <PacketDirectionBadge direction={packet.direction} />
            <span className="truncate font-medium text-sm">
              {packet.action_name}
            </span>
          </div>
          <div className="flex items-center gap-2 text-muted-foreground text-xs">
            <span>{formatTime(packet.created_at)}</span>
            <span>·</span>
            <span className="truncate">{packet.bot_id}</span>
          </div>
        </div>
      </div>
    </button>
  );
}

export default function PacketsView() {
  const [selectedPacketId, setSelectedPacketId] = useState<string | null>(null);
  const { data: packets, isLoading } = useProtocolPackets({ limit: 100 });
  const { data: packetJson, isLoading: detailLoading } =
    useProtocolPacketDetail(selectedPacketId ?? "");

  const selectedPacket = packets?.find((p) => p.packet_id === selectedPacketId);

  return (
    <div className="space-y-4">
      <h1 className="font-semibold text-xl">接口调试</h1>

      <div className="grid gap-4 sm:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Network className="size-4" /> 报文总数
            </CardTitle>
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <span className="text-lg text-muted-foreground">读取中...</span>
            ) : (
              <span className="font-semibold text-2xl">
                {packets?.length ?? 0}
              </span>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <ArrowDownToLine className="size-4" /> 接收
            </CardTitle>
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <span className="text-lg text-muted-foreground">读取中...</span>
            ) : (
              <span className="font-semibold text-2xl">
                {packets?.filter((p) => p.direction === "receive").length ?? 0}
              </span>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <ArrowUpFromLine className="size-4" /> 发送
            </CardTitle>
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <span className="text-lg text-muted-foreground">读取中...</span>
            ) : (
              <span className="font-semibold text-2xl">
                {packets?.filter((p) => p.direction === "send").length ?? 0}
              </span>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Network className="size-4" /> 报文列表
            </CardTitle>
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <p className="text-muted-foreground text-sm">加载中...</p>
            ) : packets && packets.length > 0 ? (
              <div className="space-y-2">
                {packets.map((packet: ProtocolPacket) => (
                  <PacketCard
                    key={packet.packet_id}
                    packet={packet}
                    isSelected={selectedPacketId === packet.packet_id}
                    onClick={() => setSelectedPacketId(packet.packet_id)}
                  />
                ))}
              </div>
            ) : (
              <p className="text-muted-foreground text-sm">暂无报文记录</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Network className="size-4" /> 报文详情
            </CardTitle>
          </CardHeader>
          <CardContent>
            {selectedPacketId ? (
              detailLoading ? (
                <p className="text-muted-foreground text-sm">读取中...</p>
              ) : packetJson ? (
                <div className="space-y-3">
                  {selectedPacket && (
                    <div className="space-y-1 text-xs text-muted-foreground">
                      <p>
                        <span className="font-medium">动作:</span>{" "}
                        {selectedPacket.action_name}
                      </p>
                      <p>
                        <span className="font-medium">方向:</span>{" "}
                        {selectedPacket.direction === "receive"
                          ? "接收"
                          : "发送"}
                      </p>
                      <p>
                        <span className="font-medium">时间:</span>{" "}
                        {formatTime(selectedPacket.created_at)}
                      </p>
                    </div>
                  )}
                  <pre className="whitespace-pre-wrap break-all rounded border bg-muted/30 p-3 text-xs leading-relaxed">
                    {packetJson}
                  </pre>
                </div>
              ) : (
                <p className="text-muted-foreground text-sm">
                  无法读取报文内容
                </p>
              )
            ) : (
              <p className="text-muted-foreground text-sm">
                选择一条报文查看详情
              </p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
