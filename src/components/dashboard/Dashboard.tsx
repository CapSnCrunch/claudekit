import { useEffect, useState } from "react";
import { formatDistanceToNow } from "date-fns";
import { MessageSquare, FolderOpen, TrendingUp, TrendingDown, Minus, Hash } from "lucide-react";
import { api } from "@/lib/api";
import type { DashboardStats, HeatmapDay } from "@/types";
import { ActivityHeatmap } from "./ActivityHeatmap";

export function Dashboard() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [heatmap, setHeatmap] = useState<HeatmapDay[]>([]);
  const [year] = useState(new Date().getFullYear());

  useEffect(() => {
    api.getDashboardStats().then(setStats).catch(console.error);
    api.getHeatmapData(year).then(setHeatmap).catch(console.error);
  }, [year]);

  const weekDelta = stats
    ? stats.sessionsThisWeek - stats.sessionsLastWeek
    : 0;

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-4xl mx-auto px-8 py-8 space-y-8">
        <div>
          <h1 className="text-xl font-semibold text-foreground">Dashboard</h1>
          <p className="text-sm text-muted-foreground mt-0.5">Your Claude Code activity</p>
        </div>

        {/* Stats row */}
        {stats && (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <StatCard
              label="Total sessions"
              value={stats.totalSessions}
              icon={<MessageSquare size={14} />}
            />
            <StatCard
              label="Projects"
              value={stats.totalProjects}
              icon={<FolderOpen size={14} />}
            />
            <StatCard
              label="This week"
              value={stats.sessionsThisWeek}
              icon={<Hash size={14} />}
              delta={weekDelta}
            />
            <StatCard
              label="Most active"
              value={stats.mostActiveProject?.displayName ?? "—"}
              icon={<FolderOpen size={14} />}
              isText
            />
          </div>
        )}

        {/* Heatmap */}
        <div className="bg-card border border-border rounded-lg p-5 heatmap-root relative">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h2 className="text-sm font-medium text-foreground">Activity — {year}</h2>
              <p className="text-xs text-muted-foreground mt-0.5">
                Messages sent per day
              </p>
            </div>
            <span className="text-xs text-muted-foreground">
              {heatmap.reduce((s, d) => s + d.count, 0).toLocaleString()} total messages
            </span>
          </div>
          <ActivityHeatmap data={heatmap} year={year} />
        </div>

        {/* Bottom row */}
        {stats && (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            {stats.longestSession && (
              <div className="bg-card border border-border rounded-lg p-4">
                <p className="text-xs text-muted-foreground mb-1">Longest session</p>
                <p className="text-sm font-medium text-foreground truncate">
                  {stats.longestSession.title ?? "Untitled"}
                </p>
                <p className="text-xs text-muted-foreground mt-1">
                  {stats.longestSession.messageCount} messages ·{" "}
                  {formatDistanceToNow(new Date(stats.longestSession.createdAt), { addSuffix: true })}
                </p>
              </div>
            )}
            {stats.mostActiveProject && (
              <div className="bg-card border border-border rounded-lg p-4">
                <p className="text-xs text-muted-foreground mb-1">Most active project</p>
                <p className="text-sm font-medium text-foreground truncate">
                  {stats.mostActiveProject.displayName}
                </p>
                <p className="text-xs text-muted-foreground mt-1">
                  {stats.mostActiveProject.sessionCount} sessions ·{" "}
                  {stats.mostActiveProject.decodedPath}
                </p>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  icon,
  delta,
  isText,
}: {
  label: string;
  value: number | string;
  icon: React.ReactNode;
  delta?: number;
  isText?: boolean;
}) {
  return (
    <div className="bg-card border border-border rounded-lg px-4 py-3">
      <div className="flex items-center justify-between text-muted-foreground mb-1.5">
        <span className="text-xs">{label}</span>
        {icon}
      </div>
      <div className="flex items-end gap-2">
        <span className={`font-semibold text-foreground ${isText ? "text-sm truncate" : "text-2xl"}`}>
          {value}
        </span>
        {delta !== undefined && delta !== 0 && (
          <span className={`text-xs mb-0.5 flex items-center gap-0.5 ${delta > 0 ? "text-emerald-400" : "text-red-400"}`}>
            {delta > 0 ? <TrendingUp size={11} /> : <TrendingDown size={11} />}
            {Math.abs(delta)}
          </span>
        )}
        {delta === 0 && (
          <span className="text-xs mb-0.5 text-muted-foreground flex items-center">
            <Minus size={11} />
          </span>
        )}
      </div>
    </div>
  );
}
