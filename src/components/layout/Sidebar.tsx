import { useEffect, useState } from "react";
import { ChevronDown, ChevronRight, LayoutDashboard, FolderOpen, MessageSquare, RefreshCw } from "lucide-react";
import { formatDistanceToNow } from "date-fns";
import { cn } from "@/lib/utils";
import { api } from "@/lib/api";
import type { ProjectSummary, SessionSummary } from "@/types";

interface SidebarProps {
  selectedProjectId: string | null;
  selectedSessionId: string | null;
  onSelectProject: (id: string) => void;
  onSelectSession: (id: string | null) => void;
}

export function Sidebar({
  selectedProjectId,
  selectedSessionId,
  onSelectProject,
  onSelectSession,
}: SidebarProps) {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());
  const [sessionsByProject, setSessionsByProject] = useState<Record<string, SessionSummary[]>>({});
  const [loadingSessions, setLoadingSessions] = useState<Set<string>>(new Set());

  useEffect(() => {
    api.listProjects().then(setProjects).catch(console.error);
  }, []);

  function toggleProject(projectId: string) {
    const next = new Set(expandedProjects);
    if (next.has(projectId)) {
      next.delete(projectId);
    } else {
      next.add(projectId);
      if (!sessionsByProject[projectId]) {
        loadSessions(projectId);
      }
    }
    setExpandedProjects(next);
    onSelectProject(projectId);
  }

  async function loadSessions(projectId: string) {
    setLoadingSessions((s) => new Set(s).add(projectId));
    try {
      const sessions = await api.listSessions(projectId);
      setSessionsByProject((prev) => ({ ...prev, [projectId]: sessions }));
    } catch (e) {
      console.error(e);
    } finally {
      setLoadingSessions((s) => {
        const next = new Set(s);
        next.delete(projectId);
        return next;
      });
    }
  }

  const isDashboard = selectedSessionId === null;

  return (
    <aside className="w-64 shrink-0 flex flex-col border-r border-border bg-card h-full">
      {/* Header */}
      <div className="px-4 py-3 border-b border-border">
        <span className="font-semibold text-sm tracking-wide text-foreground">ClaudeKit</span>
      </div>

      {/* Dashboard link */}
      <button
        onClick={() => onSelectSession(null)}
        className={cn(
          "flex items-center gap-2 px-4 py-2 text-sm hover:bg-accent transition-colors text-left",
          isDashboard ? "text-foreground font-medium" : "text-muted-foreground"
        )}
      >
        <LayoutDashboard size={14} className={isDashboard ? "text-primary" : ""} />
        Dashboard
      </button>

      <div className="px-4 pt-3 pb-1">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Projects</span>
      </div>

      {/* Project list */}
      <div className="flex-1 overflow-y-auto">
        {projects.length === 0 ? (
          <div className="px-4 py-3 text-xs text-muted-foreground">No sessions found</div>
        ) : (
          projects.map((project) => (
            <ProjectRow
              key={project.id}
              project={project}
              isExpanded={expandedProjects.has(project.id)}
              isSelected={project.id === selectedProjectId}
              sessions={sessionsByProject[project.id]}
              isLoadingSessions={loadingSessions.has(project.id)}
              selectedSessionId={selectedSessionId}
              onToggle={() => toggleProject(project.id)}
              onSelectSession={(sid) => onSelectSession(sid)}
            />
          ))
        )}
      </div>
    </aside>
  );
}

interface ProjectRowProps {
  project: ProjectSummary;
  isExpanded: boolean;
  isSelected: boolean;
  sessions?: SessionSummary[];
  isLoadingSessions: boolean;
  selectedSessionId: string | null;
  onToggle: () => void;
  onSelectSession: (id: string) => void;
}

function ProjectRow({
  project,
  isExpanded,
  sessions,
  isLoadingSessions,
  selectedSessionId,
  onToggle,
  onSelectSession,
}: ProjectRowProps) {
  return (
    <div>
      <button
        onClick={onToggle}
        className="w-full flex items-center gap-1.5 px-3 py-1.5 hover:bg-accent transition-colors text-left"
      >
        {isExpanded ? (
          <ChevronDown size={13} className="text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight size={13} className="text-muted-foreground shrink-0" />
        )}
        <FolderOpen size={13} className="text-muted-foreground shrink-0" />
        <span className="truncate text-foreground text-xs font-medium">{project.displayName}</span>
        <span className="ml-auto text-xs text-muted-foreground shrink-0">{project.sessionCount}</span>
      </button>

      {isExpanded && (
        <div className="pl-2">
          {isLoadingSessions ? (
            <div className="px-4 py-1.5 text-xs text-muted-foreground flex items-center gap-1.5">
              <RefreshCw size={11} className="animate-spin" /> Loading…
            </div>
          ) : sessions?.length === 0 ? (
            <div className="px-4 py-1.5 text-xs text-muted-foreground">No sessions</div>
          ) : (
            sessions?.map((session) => (
              <SessionRow
                key={session.id}
                session={session}
                isSelected={session.id === selectedSessionId}
                onSelect={() => onSelectSession(session.id)}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}

function SessionRow({
  session,
  isSelected,
  onSelect,
}: {
  session: SessionSummary;
  isSelected: boolean;
  onSelect: () => void;
}) {
  const title = session.title || "Untitled session";
  const ago = formatDistanceToNow(new Date(session.createdAt), { addSuffix: true });

  return (
    <button
      onClick={onSelect}
      className={cn(
        "w-full flex flex-col items-start px-4 py-2 text-left hover:bg-accent transition-colors border-l-2",
        isSelected ? "border-l-primary bg-accent" : "border-l-transparent"
      )}
    >
      <span className="text-xs text-foreground truncate w-full leading-snug">{title}</span>
      <div className="flex items-center gap-2 mt-0.5">
        <span className="text-[10px] text-muted-foreground">{ago}</span>
        <span className="text-[10px] text-muted-foreground flex items-center gap-0.5">
          <MessageSquare size={9} /> {session.messageCount}
        </span>
      </div>
    </button>
  );
}
