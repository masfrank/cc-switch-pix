import { useEffect, useMemo, useState } from "react";
import {
  Activity,
  Copy,
  ExternalLink,
  Loader2,
  Play,
  Save,
  Square,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  usePxpipeConfig,
  usePxpipeStatus,
  useStartPxpipeBridge,
  useStopPxpipeBridge,
  useUpdatePxpipeConfig,
} from "@/lib/query/proxy";
import type { PxpipeConfig } from "@/types/proxy";

const DEFAULT_CONFIG: PxpipeConfig = {
  enabled: false,
  command: "npx",
  args: ["pxpipe-proxy"],
  listenHost: "127.0.0.1",
  listenPort: 47821,
  models: "claude-fable-5,gpt-5.6",
};

export function PxpipeSettings() {
  const { t } = useTranslation();
  const { data: config, isLoading: isConfigLoading } = usePxpipeConfig();
  const { data: status, isLoading: isStatusLoading } = usePxpipeStatus();
  const updateConfig = useUpdatePxpipeConfig();
  const startBridge = useStartPxpipeBridge();
  const stopBridge = useStopPxpipeBridge();

  const [form, setForm] = useState(DEFAULT_CONFIG);
  const [argsText, setArgsText] = useState("");
  const [portText, setPortText] = useState(String(DEFAULT_CONFIG.listenPort));

  useEffect(() => {
    if (!config) return;
    setForm(config);
    setArgsText(config.args.join("\n"));
    setPortText(String(config.listenPort));
  }, [config]);

  const listenUrl =
    status?.listenUrl || `http://${form.listenHost}:${portText || form.listenPort}`;
  const claudeHint = `ANTHROPIC_BASE_URL=${listenUrl} claude`;
  const isPending =
    updateConfig.isPending || startBridge.isPending || stopBridge.isPending;

  const statusItems = useMemo(
    () => [
      {
        label: t("proxy.pxpipe.listenUrl", { defaultValue: "Listen URL" }),
        value: listenUrl,
      },
      {
        label: t("proxy.pxpipe.upstreamUrl", { defaultValue: "Upstream URL" }),
        value: status?.upstreamUrl || "http://127.0.0.1:15721",
      },
      {
        label: t("proxy.pxpipe.dashboardUrl", {
          defaultValue: "Dashboard URL",
        }),
        value: status?.dashboardUrl || "",
      },
    ],
    [listenUrl, status?.dashboardUrl, status?.upstreamUrl, t],
  );

  const copyText = async (text: string) => {
    if (!text) return;
    await navigator.clipboard.writeText(text);
    toast.success(t("common.copied", { defaultValue: "已复制" }), {
      closeButton: true,
    });
  };

  const handleSave = async () => {
    const port = Number(portText);
    if (!Number.isInteger(port) || port < 1024 || port > 65535) {
      toast.error(
        t("proxy.pxpipe.invalidPort", {
          defaultValue: "端口无效，请输入 1024-65535 之间的数字",
        }),
      );
      return;
    }

    await updateConfig.mutateAsync({
      ...form,
      listenHost: form.listenHost.trim() || DEFAULT_CONFIG.listenHost,
      listenPort: port,
      command: form.command.trim() || DEFAULT_CONFIG.command,
      args: argsText
        .split(/\r?\n/)
        .map((arg) => arg.trim())
        .filter(Boolean),
      models: form.models.trim() || "off",
      logPath: form.logPath?.trim() || undefined,
    });
  };

  if (isConfigLoading || isStatusLoading) {
    return (
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin" />
        {t("common.loading", { defaultValue: "加载中..." })}
      </div>
    );
  }

  return (
    <section className="space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Badge
              variant={status?.running ? "default" : "secondary"}
              className="gap-1.5"
            >
              <Activity
                className={`h-3 w-3 ${status?.running ? "animate-pulse" : ""}`}
              />
              {status?.running
                ? t("settings.advanced.proxy.running")
                : t("settings.advanced.proxy.stopped")}
            </Badge>
            {status?.pid && (
              <span className="text-xs text-muted-foreground">
                PID {status.pid}
              </span>
            )}
          </div>
          <p className="text-xs text-muted-foreground">
            {t("proxy.pxpipe.keyHint", {
              defaultValue:
                "Provider keys stay in CC Switch. PxPipe only compresses requests before the local proxy.",
            })}
          </p>
        </div>

        <div className="flex gap-2">
          {status?.running ? (
            <Button
              size="sm"
              variant="outline"
              onClick={() => void stopBridge.mutateAsync()}
              disabled={isPending}
            >
              {stopBridge.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Square className="mr-2 h-4 w-4" />
              )}
              {t("common.stop", { defaultValue: "停止" })}
            </Button>
          ) : (
            <Button
              size="sm"
              variant="outline"
              onClick={() => void startBridge.mutateAsync()}
              disabled={isPending}
            >
              {startBridge.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Play className="mr-2 h-4 w-4" />
              )}
              {t("common.start", { defaultValue: "启动" })}
            </Button>
          )}
          <Button size="sm" onClick={() => void handleSave()} disabled={isPending}>
            {updateConfig.isPending ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Save className="mr-2 h-4 w-4" />
            )}
            {t("common.save", { defaultValue: "保存" })}
          </Button>
        </div>
      </div>

      {status?.lastError && (
        <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {status.lastError}
        </div>
      )}

      <div className="flex items-center justify-between rounded-lg border border-border bg-muted/40 px-3 py-2">
        <div className="space-y-0.5">
          <Label htmlFor="pxpipe-enabled">
            {t("proxy.pxpipe.enabled", { defaultValue: "Enable PxPipe" })}
          </Label>
          <p className="text-xs text-muted-foreground">
            {t("proxy.pxpipe.enabledDescription", {
              defaultValue: "Allow CC Switch to manage the optional bridge.",
            })}
          </p>
        </div>
        <Switch
          id="pxpipe-enabled"
          checked={form.enabled}
          onCheckedChange={(enabled) =>
            setForm((prev) => ({ ...prev, enabled }))
          }
        />
      </div>

      <div className="grid gap-3 lg:grid-cols-3">
        {statusItems.map((item) => (
          <div
            key={item.label}
            className="rounded-lg border border-border bg-muted/40 p-3"
          >
            <p className="mb-2 text-xs text-muted-foreground">{item.label}</p>
            <div className="flex items-center gap-2">
              <code className="min-w-0 flex-1 truncate rounded border border-border/60 bg-background px-2 py-1.5 text-xs">
                {item.value || "-"}
              </code>
              <Button
                size="icon"
                variant="ghost"
                className="h-8 w-8"
                disabled={!item.value}
                onClick={() => void copyText(item.value)}
                title={t("common.copy", { defaultValue: "复制" })}
              >
                <Copy className="h-4 w-4" />
              </Button>
            </div>
          </div>
        ))}
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Field
          id="pxpipe-command"
          label={t("proxy.pxpipe.command", { defaultValue: "Command" })}
          value={form.command}
          onChange={(value) => setForm((prev) => ({ ...prev, command: value }))}
          placeholder="npx"
        />
        <ArgsField
          id="pxpipe-args"
          label={t("proxy.pxpipe.args", { defaultValue: "Args" })}
          value={argsText}
          onChange={setArgsText}
          placeholder={"pxpipe-proxy\n--some-flag=value"}
        />
        <Field
          id="pxpipe-host"
          label={t("proxy.pxpipe.listenHost", { defaultValue: "Listen host" })}
          value={form.listenHost}
          onChange={(value) =>
            setForm((prev) => ({ ...prev, listenHost: value }))
          }
          placeholder="127.0.0.1"
        />
        <Field
          id="pxpipe-port"
          label={t("proxy.pxpipe.listenPort", { defaultValue: "Listen port" })}
          value={portText}
          onChange={setPortText}
          placeholder="47821"
          type="number"
        />
        <Field
          id="pxpipe-models"
          label={t("proxy.pxpipe.models", {
            defaultValue: "PXPIPE_MODELS",
          })}
          value={form.models}
          onChange={(value) => setForm((prev) => ({ ...prev, models: value }))}
          placeholder="claude-fable-5,gpt-5.6"
        />
        <Field
          id="pxpipe-log-path"
          label={t("proxy.pxpipe.logPath", { defaultValue: "Log path" })}
          value={form.logPath ?? ""}
          onChange={(value) =>
            setForm((prev) => ({ ...prev, logPath: value }))
          }
          placeholder={t("common.optional", { defaultValue: "可选" })}
        />
      </div>

      <div className="rounded-lg border border-border bg-muted/40 p-3">
        <div className="mb-2 flex items-center justify-between gap-2">
          <p className="text-xs text-muted-foreground">
            {t("proxy.pxpipe.claudeHint", {
              defaultValue: "Claude Code command",
            })}
          </p>
          <Button
            size="sm"
            variant="outline"
            onClick={() => void copyText(claudeHint)}
          >
            <Copy className="mr-2 h-4 w-4" />
            {t("common.copy", { defaultValue: "复制" })}
          </Button>
        </div>
        <code className="block overflow-x-auto rounded border border-border/60 bg-background px-3 py-2 text-xs">
          {claudeHint}
        </code>
      </div>

      {status?.dashboardUrl && (
        <Button size="sm" variant="ghost" asChild>
          <a href={status.dashboardUrl} target="_blank" rel="noreferrer">
            <ExternalLink className="mr-2 h-4 w-4" />
            {t("proxy.pxpipe.openDashboard", {
              defaultValue: "Open dashboard",
            })}
          </a>
        </Button>
      )}
    </section>
  );
}

function ArgsField({
  id,
  label,
  value,
  onChange,
  placeholder,
}: Omit<FieldProps, "type">) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <Textarea
        id={id}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className="min-h-[88px] font-mono text-sm"
      />
    </div>
  );
}

interface FieldProps {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: string;
}

function Field({
  id,
  label,
  value,
  onChange,
  placeholder,
  type = "text",
}: FieldProps) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type={type}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className="font-mono text-sm"
      />
    </div>
  );
}
