import { useEffect, useState } from "react";
import "./index.css";
import { api } from "@/lib/api";
import { AppShell } from "@/components/layout/AppShell";
import { ConversationView } from "@/components/session/ConversationView";
import { RefreshCw } from "lucide-react";

function App() {
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [indexing, setIndexing] = useState(true);
  const [indexStats, setIndexStats] = useState<string | null>(null);

  useEffect(() => {
    api
      .runIndex()
      .then((stats) => {
        setIndexStats(
          `Indexed ${stats.sessionsIndexed} sessions across ${stats.projectsIndexed} projects in ${stats.durationMs}ms`
        );
      })
      .catch((e) => console.error("Index failed:", e))
      .finally(() => setIndexing(false));
  }, []);

  return (
    <AppShell
      selectedProjectId={selectedProjectId}
      selectedSessionId={selectedSessionId}
      onSelectProject={setSelectedProjectId}
      onSelectSession={setSelectedSessionId}
    >
      {indexing ? (
        <div className="flex items-center justify-center h-full gap-2 text-muted-foreground">
          <RefreshCw size={14} className="animate-spin" />
          <span className="text-sm">Indexing sessions…</span>
        </div>
      ) : selectedSessionId ? (
        <ConversationView sessionId={selectedSessionId} />
      ) : (
        <div className="flex flex-col items-center justify-center h-full gap-3 text-center px-8">
          <p className="text-2xl font-semibold text-foreground">ClaudeKit</p>
          <p className="text-sm text-muted-foreground max-w-sm">
            Select a project and session from the sidebar to browse your Claude Code history.
          </p>
          {indexStats && (
            <p className="text-xs text-muted-foreground mt-2">{indexStats}</p>
          )}
        </div>
      )}
    </AppShell>
  );
}

export default App;
