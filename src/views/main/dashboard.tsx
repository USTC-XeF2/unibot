import { Bot, MessageCircle, SquareUser, Users } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useGroupsQuery } from "@/lib/groups-query";
import { useUsersQuery } from "@/lib/users-query";

function StatValue({
  value,
  loading,
}: {
  value: number | null;
  loading: boolean;
}) {
  if (loading) {
    return <span className="text-lg text-muted-foreground">读取中...</span>;
  }

  if (value === null) {
    return <span className="text-lg text-muted-foreground">--</span>;
  }

  return (
    <span className="font-semibold text-2xl">
      {value.toLocaleString("zh-CN")}
    </span>
  );
}

function DashboardView() {
  const usersQuery = useUsersQuery();
  const groupsQuery = useGroupsQuery();

  return (
    <div className="space-y-4">
      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <SquareUser className="size-4" /> 总用户数
            </CardTitle>
          </CardHeader>
          <CardContent>
            <StatValue
              value={usersQuery.data?.length ?? null}
              loading={usersQuery.isPending}
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Users className="size-4" /> 总群聊数
            </CardTitle>
          </CardHeader>
          <CardContent>
            <StatValue
              value={groupsQuery.data?.length ?? null}
              loading={groupsQuery.isPending}
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <MessageCircle className="size-4" /> 总消息数
            </CardTitle>
          </CardHeader>
          <CardContent>
            <StatValue value={null} loading={false} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2 text-sm">
              <Bot className="size-4" /> 在线机器人数
            </CardTitle>
          </CardHeader>
          <CardContent>
            <StatValue value={null} loading={false} />
          </CardContent>
        </Card>
      </section>
    </div>
  );
}

export default DashboardView;
