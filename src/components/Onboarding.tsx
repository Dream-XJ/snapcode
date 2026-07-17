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
import { useI18n } from "@/lib/i18n";
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
  const { t } = useI18n();
  const state = status?.state;

  if (state === "unsupported") {
    return (
      <Center>
        <div className="rounded-full bg-muted p-4">
          <AlertTriangle className="h-7 w-7 text-amber-500" />
        </div>
        <h1 className="text-lg font-semibold">{t("onboarding.unsupportedTitle")}</h1>
        <p className="text-sm text-muted-foreground">{t("onboarding.unsupportedDesc")}</p>
        <Button variant="ghost" onClick={onComplete}>
          {t("onboarding.enterAnyway")}
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
        <h1 className="text-lg font-semibold">{t("onboarding.deniedTitle")}</h1>
        <p className="max-w-[300px] text-sm leading-relaxed text-muted-foreground">
          {t("onboarding.deniedDesc")}
        </p>
        <div className="flex w-56 flex-col gap-2">
          <Button onClick={() => void openNotificationSettings()}>
            {t("common.openSystemSettings")}
          </Button>
          <Button variant="outline" onClick={() => void retryListener()}>
            {t("common.retry")}
          </Button>
          <Button variant="ghost" onClick={onComplete}>
            {t("onboarding.continueAnyway")}
          </Button>
        </div>
      </Center>
    );
  }

  const steps: { icon: LucideIcon; title: string; desc: ReactNode }[] = [
    {
      icon: Bell,
      title: t("onboarding.stepListenTitle"),
      desc: t("onboarding.stepListenDesc"),
    },
    {
      icon: ScanLine,
      title: t("onboarding.stepDetectTitle"),
      desc: t("onboarding.stepDetectDesc"),
    },
    {
      icon: Keyboard,
      title: t("onboarding.stepPasteTitle"),
      desc: (
        <>
          {t("onboarding.stepPasteDesc1")}{" "}
          <kbd className="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">{shortcut}</kbd>{" "}
          {t("onboarding.stepPasteDesc2")}
        </>
      ),
    },
  ];

  return (
    <Center>
      <div>
        <h1 className="text-xl font-semibold tracking-tight">{t("onboarding.welcomeTitle")}</h1>
        <p className="mt-1.5 text-sm text-muted-foreground">{t("onboarding.welcomeDesc")}</p>
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
          {t("onboarding.openSettings")}
        </Button>
        <Button onClick={onComplete}>{t("onboarding.done")}</Button>
      </div>

      <p className="max-w-[320px] text-xs leading-relaxed text-muted-foreground">
        {t("onboarding.footer")}
      </p>
    </Center>
  );
}
