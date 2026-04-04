import { invoke } from "@tauri-apps/api/core";
import { WindowTitlebar } from "@tauri-controls-v2/react";
import {
  ChevronDown,
  Info,
  LayoutDashboard,
  Logs,
  MessageCircle,
  Settings,
  Users,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Outlet, useLocation, useNavigate } from "react-router";
import appIcon from "@/assets/icon.png";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  SidebarProvider,
  SidebarSeparator,
} from "@/components/ui/sidebar";
import { useUsersQuery } from "@/lib/query";

const footerNavItems = [
  { value: "settings", to: "/settings", label: "设置", icon: Settings },
  { value: "about", to: "/about", label: "关于", icon: Info },
];

const navValues = new Set(["dashboard", "logs", "settings", "about"]);

function MainLayout() {
  const [usersOpen, setUsersOpen] = useState(true);
  const location = useLocation();
  const navigate = useNavigate();

  const usersQuery = useUsersQuery();
  const users = usersQuery.data ?? [];
  const sidebarUsers = useMemo(() => users.slice(0, 5), [users]);

  useEffect(() => {
    if (location.pathname.startsWith("/user/")) {
      setUsersOpen(true);
    }
  }, [location.pathname]);

  const handleUsersMenuClick = () => {
    if (location.pathname !== "/users") {
      navigate("/users");
      if (!usersOpen) {
        setUsersOpen(true);
      }
      return;
    }

    setUsersOpen(!usersOpen);
  };

  const handleOpenUserChatWindow = async (userId: number) => {
    try {
      await invoke<{ created: boolean }>("open_user_chat_window", {
        userId,
      });
    } catch (error) {
      window.alert(error as string);
    }
  };

  const tabValue = useMemo(() => {
    if (location.pathname === "/") {
      return "dashboard";
    }
    const segment = location.pathname.replace(/^\//, "").split("/")[0];
    return navValues.has(segment) ? segment : "";
  }, [location.pathname]);
  const isUsersSectionActive = location.pathname.startsWith("/user");

  return (
    <SidebarProvider className="h-screen w-screen overflow-hidden">
      <Sidebar>
        <SidebarHeader>
          <div className="flex items-center gap-3">
            <img
              src={appIcon}
              alt="UniBot"
              className="size-9 rounded-md object-cover"
              draggable={false}
            />
            <span className="font-semibold text-sm">UniBot</span>
          </div>
        </SidebarHeader>

        <SidebarSeparator className="mx-0" />

        <SidebarContent>
          <SidebarGroup>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton
                  isActive={tabValue === "dashboard"}
                  onClick={() => navigate("/")}
                  tooltip="主面板"
                >
                  <LayoutDashboard className="size-4" />
                  主面板
                </SidebarMenuButton>
              </SidebarMenuItem>

              <SidebarMenuItem>
                <SidebarMenuButton
                  isActive={isUsersSectionActive}
                  onClick={handleUsersMenuClick}
                  tooltip="用户管理"
                >
                  <Users className="size-4" />
                  用户管理
                  <ChevronDown
                    className={`ml-auto size-4 transition-transform ${
                      usersOpen ? "rotate-180" : ""
                    }`}
                  />
                </SidebarMenuButton>

                {usersOpen ? (
                  <SidebarMenuSub>
                    {sidebarUsers.length > 0 ? (
                      sidebarUsers.map((user) => {
                        const userPath = `/user/${user.user_id}`;
                        const isActive = location.pathname === userPath;
                        return (
                          <SidebarMenuSubItem key={user.user_id}>
                            <SidebarMenuSubButton
                              asChild
                              isActive={isActive}
                              onClick={() => navigate(userPath)}
                            >
                              <div className="flex">
                                <Avatar size="sm" className="size-5">
                                  <AvatarImage src={user.avatar} />
                                  <AvatarFallback>
                                    {user.nickname.slice(0, 1).toUpperCase()}
                                  </AvatarFallback>
                                </Avatar>
                                <span className="flex-1">{user.nickname}</span>
                                <Button
                                  type="button"
                                  size="icon-xs"
                                  variant="ghost"
                                  aria-label="打开聊天窗口"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    handleOpenUserChatWindow(user.user_id);
                                  }}
                                >
                                  <MessageCircle className="size-4" />
                                </Button>
                              </div>
                            </SidebarMenuSubButton>
                          </SidebarMenuSubItem>
                        );
                      })
                    ) : (
                      <SidebarMenuSubItem>
                        <SidebarMenuSubButton asChild>
                          <button type="button" disabled className="w-full">
                            暂无用户
                          </button>
                        </SidebarMenuSubButton>
                      </SidebarMenuSubItem>
                    )}
                  </SidebarMenuSub>
                ) : null}
              </SidebarMenuItem>

              <SidebarMenuItem>
                <SidebarMenuButton
                  isActive={tabValue === "logs"}
                  onClick={() => navigate("/logs")}
                  tooltip="日志"
                >
                  <Logs className="size-4" />
                  日志
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroup>
        </SidebarContent>

        <SidebarSeparator className="mx-0" />

        <SidebarFooter>
          <SidebarMenu>
            {footerNavItems.map((item) => {
              const Icon = item.icon;
              return (
                <SidebarMenuItem key={item.value}>
                  <SidebarMenuButton
                    isActive={tabValue === item.value}
                    onClick={() => navigate(item.to)}
                    tooltip={item.label}
                  >
                    <Icon className="size-4" />
                    {item.label}
                  </SidebarMenuButton>
                </SidebarMenuItem>
              );
            })}
          </SidebarMenu>
        </SidebarFooter>
      </Sidebar>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <WindowTitlebar className="border-border border-b" />

        <main className="min-h-0 flex-1 overflow-auto p-4 sm:p-6">
          <Outlet />
        </main>
      </div>
    </SidebarProvider>
  );
}

export default MainLayout;
