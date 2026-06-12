import { Wrench } from "lucide-react";
import { useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { EventsPanel } from "@/views/dev-tools/events-panel";
import { LogsPanel } from "@/views/dev-tools/logs-panel";
import { SchemaPanel } from "@/views/dev-tools/schema-panel";

export default function DevToolsWindow() {
  const [activeTab, setActiveTab] = useState("logs");

  return (
    <div className="flex h-screen w-screen flex-col bg-background p-4">
      <header className="mb-4 flex items-center gap-2">
        <div className="flex size-8 items-center justify-center rounded-lg border bg-card">
          <Wrench className="size-4 text-muted-foreground" />
        </div>
        <div>
          <h1 className="font-semibold text-lg">开发者工具</h1>
          <p className="text-muted-foreground text-xs">
            日志、事件流、数据库结构与 SQL 调试
          </p>
        </div>
      </header>

      <Card className="flex flex-1 flex-col overflow-hidden">
        <CardContent className="flex flex-1 flex-col p-4">
          <Tabs
            value={activeTab}
            onValueChange={setActiveTab}
            className="flex flex-1 flex-col"
          >
            <TabsList className="mb-4 self-start">
              <TabsTrigger value="logs">日志</TabsTrigger>
              <TabsTrigger value="events">事件流</TabsTrigger>
              <TabsTrigger value="schema">数据库</TabsTrigger>
              <TabsTrigger value="sql" disabled>
                SQL（PR2）
              </TabsTrigger>
            </TabsList>

            <TabsContent value="logs" className="flex-1 overflow-hidden">
              <LogsPanel />
            </TabsContent>
            <TabsContent
              value="events"
              className="flex-1 overflow-hidden"
              forceMount
            >
              <EventsPanel />
            </TabsContent>
            <TabsContent value="schema" className="flex-1 overflow-hidden">
              <SchemaPanel />
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  );
}
