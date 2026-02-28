import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { cn } from "@/lib/utils";
import type { ContentBlock } from "@/types";
import { ToolBlock } from "./ToolBlock";
import "highlight.js/styles/github-dark.css";

interface MessageBubbleProps {
  role: string;
  blocks: ContentBlock[];
  timestamp: string;
  model: string | null;
}

export function MessageBubble({ role, blocks, timestamp, model }: MessageBubbleProps) {
  const isUser = role === "user";
  const isAssistant = role === "assistant";

  if (!isUser && !isAssistant) return null;

  // Separate pure tool_result blocks (they render inline with the previous tool_use)
  const hasOnlyToolResults = blocks.every((b) => b.type === "tool_result");
  if (hasOnlyToolResults) {
    return (
      <div className="space-y-2">
        {blocks.map((block, i) =>
          block.type === "tool_result" ? (
            <ToolBlock key={i} block={block} />
          ) : null
        )}
      </div>
    );
  }

  return (
    <div className={cn("flex flex-col gap-2", isUser ? "items-end" : "items-start")}>
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span className="font-medium">{isUser ? "You" : model ? `Claude (${model.split("-").slice(0,2).join("-")})` : "Claude"}</span>
        <span>{formatTime(timestamp)}</span>
      </div>

      <div
        className={cn(
          "max-w-[85%] rounded-lg px-4 py-3 text-sm",
          isUser
            ? "bg-primary text-primary-foreground"
            : "bg-card border border-border text-foreground"
        )}
      >
        {blocks.map((block, i) => {
          if (block.type === "text") {
            return (
              <div key={i} className={cn("prose prose-sm max-w-none", isUser && "prose-invert")}>
                <ReactMarkdown
                  remarkPlugins={[remarkGfm]}
                  rehypePlugins={[rehypeHighlight]}
                >
                  {block.text}
                </ReactMarkdown>
              </div>
            );
          }
          if (block.type === "tool_use") {
            return <ToolBlock key={i} block={block} />;
          }
          if (block.type === "tool_result") {
            return <ToolBlock key={i} block={block} />;
          }
          return null;
        })}
      </div>
    </div>
  );
}

function formatTime(timestamp: string): string {
  try {
    return new Date(timestamp).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return timestamp;
  }
}
