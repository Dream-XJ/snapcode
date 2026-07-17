import type { ReactNode } from "react";
import {
  AlertTriangle,
  Bell,
  Keyboard,
  ScanLine,
  ShieldAlert,
  type LucideIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { openNotificationSettings, retryListener } from "@/lib/tauri";
import type { ListenerState } from "@/types";

interface OnboardingProps {
  status: ListenerState | null;
  shortcut: string;
  onComplete: () => void;
}

function Center({ children }: { children: ReactNode }) {
  return (
    <div className="h-full overflow-y-auto">
      <div className="flex min-h-full flex-col items-center justify-center gap-5 px-8 py-10 text-center">
        {children}
      </div>
    </div>
  );
}

export function Onboarding({ status, shortcut, onComplete }: OnboardingProps) {
  const state = status?.state;

  if (state === "unsupported") {
    return (
      <Center>
        <div className="rounded-full bg-muted p-4">
          <AlertTriangle className="h-7 w-7 text-amber-500" />
        </div>
        <h1 className="text-lg font-semibold">系统版本不受支持</h1>
        <p className="text-sm text-muted-foreground">需要 Windows 10 1809 或更高版本</p>
        <Button variant="ghost" onClick={onComplete}>
          仍要进入应用
        </Button>
      </Center>
    );
  }

  if (state === "access_denied") {
    return (
      <Center>
        <div className="rounded-full bg-muted p-4">
          <ShieldAlert className="h-7 w-7 text-red-500" />
        </div>
        <h1 className="text-lg font-semibold">通知访问权限被拒绝</h1>
        <p className="max-w-[300px] text-sm leading-relaxed text-muted-foreground">
          SnapCode 需要读取系统通知才能识别短信验证码，请在系统设置中授权后重新检测。
        </p>
        <div className="flex w-56 flex-col gap-2">
          <Button onClick={() => void openNotificationSettings()}>打开系统设置</Button>
          <Button variant="outline" onClick={() => void retryListener()}>
            重新检测
          </Button>
          <Button variant="ghost" onClick={onComplete}>
            暂不授权，继续使用
          </Button>
        </div>
      </Center>
    );
  }

  const steps: { icon: LucideIcon; title: string; desc: ReactNode }[] = [
    {
      icon: Bell,
      title: "监听通知",
      desc: "读取「手机连接」同步到 Windows 的短信通知",
    },
    {
      icon: ScanLine,
      title: "识别验证码",
      desc: "自动提取短信中的数字验证码，存入本地历史",
    },
    {
      icon: Keyboard,
      title: "快捷粘贴",
      desc: (
        <>
          按下{" "}
          <kbd className="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">{shortcut}</kbd>{" "}
          即可粘贴最新验证码
        </>
      ),
    },
  ];

  return (
    <Center>
      <div>
        <h1 className="text-xl font-semibold tracking-tight">欢迎使用 SnapCode 闪码</h1>
        <p className="mt-1.5 text-sm text-muted-foreground">
          自动捕获 Windows 通知里的短信验证码
        </p>
      </div>

      <ol className="w-full max-w-[320px] space-y-2.5 text-left">
        {steps.map((s, i) => {
          const Icon = s.icon;
          return (
            <li key={s.title} className="flex items-start gap-3 rounded-xl border bg-card p-3">
              <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <Icon className="h-4 w-4" />
              </div>
              <div className="min-w-0">
                <p className="text-sm font-medium">
                  {i + 1}. {s.title}
                </p>
                <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{s.desc}</p>
              </div>
            </li>
          );
        })}
      </ol>

      <div className="flex w-56 flex-col gap-2">
        <Button variant="outline" onClick={() => void openNotificationSettings()}>
          打开通知设置
        </Button>
        <Button onClick={onComplete}>我已完成授权，开始使用</Button>
      </div>

      <p className="max-w-[320px] text-xs leading-relaxed text-muted-foreground">
        使用前请在「手机连接」中开启短信同步：iPhone 需保持蓝牙连接，并在 iOS
        通知设置中允许短信显示内容；Android 请在「连接至 Windows」中开启短信同步。
      </p>
    </Center>
  );
}
