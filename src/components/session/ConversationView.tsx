import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { MessageRecord, ContentBlock, SessionInfo } from "@/types";
import { MessageBubble } from "./MessageBubble";
import { RefreshCw, Terminal, Monitor, Code2 } from "lucide-react";

interface ConversationViewProps {
  sessionId: string;
}

export function ConversationView({ sessionId }: ConversationViewProps) {
  const [messages, setMessages] = useState<MessageRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sessionInfo, setSessionInfo] = useState<SessionInfo | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    setSessionInfo(null);
    Promise.all([
      api.getSessionMessages(sessionId),
      api.getSessionInfo(sessionId),
    ])
      .then(([msgs, info]) => {
        setMessages(msgs.filter((m) => !m.isSummary));
        setSessionInfo(info);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [sessionId]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground gap-2">
        <RefreshCw size={14} className="animate-spin" />
        <span className="text-sm">Loading session…</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full text-destructive text-sm">
        Failed to load session: {error}
      </div>
    );
  }

  if (messages.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
        No messages in this session.
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col overflow-hidden">
      {/* Open-in toolbar */}
      {sessionInfo && (
        <OpenInBar sessionInfo={sessionInfo} />
      )}

      {/* Message thread */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-3xl mx-auto px-6 py-6 space-y-4">
          {messages.map((msg) => {
            let blocks: ContentBlock[] = [];
            try {
              const parsed = JSON.parse(msg.contentJson);
              if (Array.isArray(parsed)) {
                blocks = parsed;
              } else if (typeof parsed === "string") {
                blocks = [{ type: "text", text: parsed }];
              }
            } catch {
              blocks = [{ type: "text", text: msg.contentJson }];
            }
            return (
              <MessageBubble
                key={msg.id}
                role={msg.role}
                blocks={blocks}
                timestamp={msg.timestamp}
                model={msg.model}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}

// ── Open-in toolbar ────────────────────────────────────────────────────────────

function OpenInBar({ sessionInfo }: { sessionInfo: SessionInfo }) {
  const [launching, setLaunching] = useState<string | null>(null);
  const [launchError, setLaunchError] = useState<string | null>(null);

  async function launch(app: "claude_code" | "cursor" | "claude_desktop") {
    setLaunching(app);
    setLaunchError(null);
    try {
      await api.openInApp(app, sessionInfo.projectDecodedPath, sessionInfo.sessionId);
    } catch (e) {
      setLaunchError(String(e));
    } finally {
      setLaunching(null);
    }
  }

  return (
    <div className="shrink-0 border-b border-border bg-card px-6 py-2 flex items-center justify-between gap-4">
      <div className="min-w-0">
        <p className="text-xs font-medium text-foreground truncate">
          {sessionInfo.title ?? "Untitled session"}
        </p>
        <p className="text-[11px] text-muted-foreground truncate">{sessionInfo.projectDecodedPath}</p>
      </div>

      <div className="flex items-center gap-1.5 shrink-0">
        <OpenInButton
          label="Claude Code"
          icon={<Terminal size={12} />}
          loading={launching === "claude_code"}
          onClick={() => launch("claude_code")}
        />
        <OpenInButton
          label="Cursor"
          icon={<Code2 size={12} />}
          loading={launching === "cursor"}
          onClick={() => launch("cursor")}
        />
        <OpenInButton
          label="Claude Desktop"
          icon={<Monitor size={12} />}
          loading={launching === "claude_desktop"}
          onClick={() => launch("claude_desktop")}
        />
      </div>

      {launchError && (
        <p className="text-[11px] text-destructive shrink-0 max-w-[200px] truncate" title={launchError}>
          {launchError}
        </p>
      )}
    </div>
  );
}

function OpenInButton({
  label,
  icon,
  loading,
  onClick,
}: {
  label: string;
  icon: React.ReactNode;
  loading: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      disabled={loading}
      title={`Open in ${label}`}
      className="flex items-center gap-1.5 px-2.5 py-1 rounded text-xs text-muted-foreground
        border border-border hover:border-primary/50 hover:text-foreground
        transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {loading ? <RefreshCw size={12} className="animate-spin" /> : icon}
      {label}
    </button>
  );
}
