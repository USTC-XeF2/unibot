import { getTauriVersion, getVersion } from "@tauri-apps/api/app";
import { ExternalLink, Layers, Monitor, Package } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import appIcon from "@/assets/icon.png";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import packageJson from "../../../package.json";

type UserAgentBrand = {
  brand: string;
  version: string;
};

type NavigatorWithUserAgentData = Navigator & {
  userAgentData?: {
    brands?: UserAgentBrand[];
  };
};

type RuntimeInfo = {
  appName: string;
  appVersion: string;
  tauriVersion: string;
  webviewVersion: string;
  userAgent: string;
};

const repoUrl = "https://github.com/USTC-XeF2/unibot";

function detectWebviewVersion(): string {
  if (typeof navigator === "undefined") {
    return "未知";
  }

  const uaData = (navigator as NavigatorWithUserAgentData).userAgentData;
  if (uaData?.brands?.length) {
    const chromeLike = uaData.brands.find(
      (brand) =>
        brand.brand.includes("Chrom") ||
        brand.brand.includes("Edge") ||
        brand.brand.includes("WebView"),
    );
    if (chromeLike?.version) {
      return chromeLike.version;
    }
  }

  const ua = navigator.userAgent;
  const matchers = [
    /WebView2\/([\d.]+)/,
    /Chrome\/([\d.]+)/,
    /Chromium\/([\d.]+)/,
    /Version\/([\d.]+) Safari\//,
  ];

  for (const matcher of matchers) {
    const matched = ua.match(matcher);
    if (matched?.[1]) {
      return matched[1];
    }
  }

  return "未知";
}

function AboutView() {
  const [runtimeInfo, setRuntimeInfo] = useState<RuntimeInfo>({
    appName: "UniBot",
    appVersion: packageJson.version,
    tauriVersion: "读取中...",
    webviewVersion: detectWebviewVersion(),
    userAgent:
      typeof navigator !== "undefined" ? navigator.userAgent : "未知环境",
  });

  useEffect(() => {
    let cancelled = false;

    const loadVersions = async () => {
      try {
        const [appVersion, tauriVersion] = await Promise.all([
          getVersion(),
          getTauriVersion(),
        ]);
        if (!cancelled) {
          setRuntimeInfo((prev) => ({
            ...prev,
            appVersion,
            tauriVersion,
          }));
        }
      } catch {
        if (!cancelled) {
          setRuntimeInfo((prev) => ({
            ...prev,
            tauriVersion: "读取失败",
            webviewVersion: detectWebviewVersion(),
          }));
        }
      }
    };

    loadVersions();

    return () => {
      cancelled = true;
    };
  }, []);

  const stackSummary = useMemo(
    () => ({
      frontend: ["React 19", "Tailwind CSS 4", "Shadcn UI"],
      backend: ["Rust", "Tauri 2", "SQLite"],
    }),
    [],
  );

  return (
    <div className="space-y-4">
      <Card>
        <CardContent className="flex flex-col items-center gap-3 py-8 text-center">
          <img
            src={appIcon}
            alt="UniBot 图标"
            className="size-20 rounded-2xl border object-cover shadow-sm"
            draggable={false}
          />
          <div className="space-y-1">
            <h2 className="font-semibold text-2xl">{runtimeInfo.appName}</h2>
            <p className="max-w-xl text-muted-foreground text-sm">
              本地优先的机器人调试平台，集成协议对接、事件观测与消息链路验证。
            </p>
          </div>
          <a
            href={repoUrl}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm transition-colors hover:bg-accent"
          >
            <ExternalLink className="size-4" /> GitHub 仓库
          </a>
        </CardContent>
      </Card>

      <div className="grid gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <Package className="size-4" /> 版本信息
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2 text-sm">
            <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2">
              <span className="text-muted-foreground">应用版本</span>
              <span className="font-medium">{runtimeInfo.appVersion}</span>
            </div>
            <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2">
              <span className="text-muted-foreground">Tauri 版本</span>
              <span className="font-medium">{runtimeInfo.tauriVersion}</span>
            </div>
            <div className="flex items-center justify-between rounded-md border bg-muted/20 px-3 py-2">
              <span className="text-muted-foreground">WebView 版本</span>
              <span className="font-medium">{runtimeInfo.webviewVersion}</span>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <Layers className="size-4" /> 技术栈
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-sm">
            <div>
              <p className="mb-1 font-medium">前端</p>
              <p className="text-muted-foreground">
                {stackSummary.frontend.join(" / ")}
              </p>
            </div>
            <div>
              <p className="mb-1 font-medium">后端</p>
              <p className="text-muted-foreground">
                {stackSummary.backend.join(" / ")}
              </p>
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Monitor className="size-4" /> 运行环境
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-sm">
          <div className="rounded-md border bg-muted/20 px-3 py-2">
            <p className="mb-1 text-muted-foreground text-xs">User Agent</p>
            <p className="break-all font-mono text-xs">
              {runtimeInfo.userAgent}
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export default AboutView;
