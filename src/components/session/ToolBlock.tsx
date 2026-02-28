import { useState } from "react";
import { ChevronDown, ChevronRight, Terminal, FileText, FilePen, Search, Globe, Bot, AlertCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import type { ContentBlock } from "@/types";

interface ToolBlockProps {
  block: ContentBlock;
}

export function ToolBlock({ block }: ToolBlockProps) {
  const [expanded, setExpanded] = useState(false);

  if (block.type === "tool_use") {
    const { name, input } = block;
    const { icon, color, label } = toolMeta(name);

    const preview = getInputPreview(name, input);
    const body = JSON.stringify(input, null, 2);
    const isLong = body.split("\n").length > 20;

    return (
      <div className={cn("my-2 rounded border text-xs font-mono", color)}>
        <button
          onClick={() => setExpanded((e) => !e)}
          className="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
        >
          {expanded ? <ChevronDown size={11} /> : <ChevronRight size={11} />}
          <span className="shrink-0">{icon}</span>
          <span className="font-semibold">{label}</span>
          {preview && (
            <span className="ml-2 text-muted-foreground truncate font-normal">{preview}</span>
          )}
        </button>

        {expanded && (
          <div className="border-t border-border px-3 pb-2 pt-2">
            <ExpandableContent content={body} isLong={isLong} />
          </div>
        )}
      </div>
    );
  }

  if (block.type === "tool_result") {
    const content = typeof block.content === "string"
      ? block.content
      : JSON.stringify(block.content, null, 2);
    const lines = content.split("\n");
    const isLong = lines.length > 20;
    const isError = block.is_error;

    return (
      <div className={cn("my-1 rounded border text-xs font-mono", isError ? "border-destructive/40 bg-destructive/5" : "border-border bg-muted/30")}>
        <button
          onClick={() => setExpanded((e) => !e)}
          className="w-full flex items-center gap-2 px-3 py-1.5 text-left hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
        >
          {expanded ? <ChevronDown size={11} /> : <ChevronRight size={11} />}
          {isError && <AlertCircle size={11} className="text-destructive" />}
          <span className="text-muted-foreground font-normal">
            {isError ? "Error" : `Result`}
            {!expanded && ` · ${lines.length} line${lines.length !== 1 ? "s" : ""}`}
          </span>
        </button>

        {expanded && (
          <div className="border-t border-border px-3 pb-2 pt-2">
            <ExpandableContent content={content} isLong={isLong} />
          </div>
        )}
      </div>
    );
  }

  return null;
}

function ExpandableContent({ content, isLong }: { content: string; isLong: boolean }) {
  const [showAll, setShowAll] = useState(false);
  const lines = content.split("\n");
  const display = isLong && !showAll ? lines.slice(0, 20).join("\n") : content;

  return (
    <div>
      <pre className="whitespace-pre-wrap break-words text-[11px] leading-relaxed">{display}</pre>
      {isLong && (
        <button
          onClick={() => setShowAll((s) => !s)}
          className="mt-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
        >
          {showAll ? "Show less" : `Show all (${lines.length} lines)`}
        </button>
      )}
    </div>
  );
}

function toolMeta(name: string): { icon: React.ReactNode; color: string; label: string } {
  switch (name.toLowerCase()) {
    case "bash":
      return { icon: <Terminal size={11} />, color: "border-zinc-600/40 bg-zinc-950/80 text-zinc-100", label: "Bash" };
    case "read":
      return { icon: <FileText size={11} />, color: "border-blue-500/30 bg-blue-950/20 text-blue-100", label: "Read" };
    case "write":
    case "edit":
    case "multiedit":
      return { icon: <FilePen size={11} />, color: "border-amber-500/30 bg-amber-950/20 text-amber-100", label: name };
    case "glob":
    case "grep":
      return { icon: <Search size={11} />, color: "border-purple-500/30 bg-purple-950/20 text-purple-100", label: name };
    case "webfetch":
    case "websearch":
      return { icon: <Globe size={11} />, color: "border-teal-500/30 bg-teal-950/20 text-teal-100", label: name };
    case "task":
      return { icon: <Bot size={11} />, color: "border-emerald-500/30 bg-emerald-950/20 text-emerald-100", label: "Subagent Task" };
    default:
      return { icon: <Terminal size={11} />, color: "border-border bg-muted/40 text-foreground", label: name };
  }
}

function getInputPreview(name: string, input: Record<string, unknown>): string {
  switch (name.toLowerCase()) {
    case "bash": return String(input.command ?? "").split("\n")[0].slice(0, 80);
    case "read": return String(input.file_path ?? input.path ?? "");
    case "write": return String(input.file_path ?? input.path ?? "");
    case "edit": case "multiedit": return String(input.file_path ?? input.path ?? "");
    case "glob": return String(input.pattern ?? "");
    case "grep": return String(input.pattern ?? "");
    case "webfetch": return String(input.url ?? "").slice(0, 80);
    case "websearch": return String(input.query ?? "").slice(0, 80);
    default: return "";
  }
}
