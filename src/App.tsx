import { useEffect, useState } from "react";
import "./index.css";
import { api } from "@/lib/api";
import { AppShell } from "@/components/layout/AppShell";
import { ConversationView } from "@/components/session/ConversationView";
import { Dashboard } from "@/components/dashboard/Dashboard";
import { RefreshCw } from "lucide-react";

function App() {
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [indexing, setIndexing] = useState(true);

  useEffect(() => {
    api
      .runIndex()
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
        <Dashboard onSelectSession={setSelectedSessionId} />
      )}
    </AppShell>
  );
}

export default App;
