import { useCallback, useEffect, useRef, useState } from "react";
import "./index.css";
import { api } from "@/lib/api";
import { AppShell } from "@/components/layout/AppShell";
import { ConversationView } from "@/components/session/ConversationView";
import { Dashboard } from "@/components/dashboard/Dashboard";
import { RefreshCw } from "lucide-react";

const SYNC_INTERVAL_MS = 30_000;

function App() {
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [indexing, setIndexing] = useState(true);
  const [lastSynced, setLastSynced] = useState(0);
  const syncingRef = useRef(false);

  const runSync = useCallback(async () => {
    if (syncingRef.current) return;
    syncingRef.current = true;
    try {
      await api.runIndex();
      setLastSynced(Date.now());
    } catch (e) {
      console.error("Index failed:", e);
    } finally {
      syncingRef.current = false;
    }
  }, []);

  // Initial index
  useEffect(() => {
    runSync().finally(() => setIndexing(false));
  }, [runSync]);

  // Background auto-sync every 30s
  useEffect(() => {
    const id = setInterval(runSync, SYNC_INTERVAL_MS);
    return () => clearInterval(id);
  }, [runSync]);

  return (
    <AppShell
      selectedProjectId={selectedProjectId}
      selectedSessionId={selectedSessionId}
      onSelectProject={setSelectedProjectId}
      onSelectSession={setSelectedSessionId}
      lastSynced={lastSynced}
      onSync={runSync}
    >
      {indexing ? (
        <div className="flex items-center justify-center h-full gap-2 text-muted-foreground">
          <RefreshCw size={14} className="animate-spin" />
          <span className="text-sm">Indexing sessions…</span>
        </div>
      ) : selectedSessionId ? (
        <ConversationView sessionId={selectedSessionId} />
      ) : (
        <Dashboard />
      )}
    </AppShell>
  );
}

export default App;
