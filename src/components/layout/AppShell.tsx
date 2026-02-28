import { Sidebar } from "./Sidebar";

interface AppShellProps {
  selectedProjectId: string | null;
  selectedSessionId: string | null;
  onSelectProject: (id: string) => void;
  onSelectSession: (id: string | null) => void;
  lastSynced: number;
  onSync: () => Promise<void>;
  children: React.ReactNode;
}

export function AppShell({
  selectedProjectId,
  selectedSessionId,
  onSelectProject,
  onSelectSession,
  lastSynced,
  onSync,
  children,
}: AppShellProps) {
  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      <Sidebar
        selectedProjectId={selectedProjectId}
        selectedSessionId={selectedSessionId}
        onSelectProject={onSelectProject}
        onSelectSession={onSelectSession}
        lastSynced={lastSynced}
        onSync={onSync}
      />
      <main className="flex-1 overflow-hidden">{children}</main>
    </div>
  );
}
