import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { MessageRecord, ContentBlock } from "@/types";
import { MessageBubble } from "./MessageBubble";
import { RefreshCw } from "lucide-react";

interface ConversationViewProps {
  sessionId: string;
}

export function ConversationView({ sessionId }: ConversationViewProps) {
  const [messages, setMessages] = useState<MessageRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    api
      .getSessionMessages(sessionId)
      .then((msgs) => setMessages(msgs.filter((m) => !m.isSummary)))
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
    <div className="h-full overflow-y-auto">
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
  );
}
