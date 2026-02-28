import { useEffect, useState } from "react";
import { format } from "date-fns";
import { MessageSquare, FolderOpen, TrendingUp, TrendingDown, Minus, Hash, X } from "lucide-react";
import { api } from "@/lib/api";
import type { DashboardStats, HeatmapDay, DayDetail } from "@/types";
import { ActivityHeatmap } from "./ActivityHeatmap";

export function Dashboard() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [heatmap, setHeatmap] = useState<HeatmapDay[]>([]);
  const [year] = useState(new Date().getFullYear());
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
  const [dayDetail, setDayDetail] = useState<DayDetail | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);

  useEffect(() => {
    api.getDashboardStats().then(setStats).catch(console.error);
    api.getHeatmapData(year).then(setHeatmap).catch(console.error);
  }, [year]);

  async function handleDayClick(date: string) {
    if (selectedDate === date) {
      // toggle off
      setSelectedDate(null);
      setDayDetail(null);
      return;
    }
    setSelectedDate(date);
    setLoadingDetail(true);
    try {
      const detail = await api.getDayDetail(date);
      setDayDetail(detail);
    } catch (e) {
      console.error(e);
    } finally {
      setLoadingDetail(false);
    }
  }

  const weekDelta = stats ? stats.sessionsThisWeek - stats.sessionsLastWeek : 0;

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-4xl mx-auto px-8 py-8 space-y-6">
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
              sub={stats.mostActiveProject ? `${stats.mostActiveProject.sessionCount} sessions` : undefined}
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
                Messages sent per day · click a day for details
              </p>
            </div>
            <span className="text-xs text-muted-foreground">
              {heatmap.reduce((s, d) => s + d.count, 0).toLocaleString()} total
            </span>
          </div>
          <ActivityHeatmap
            data={heatmap}
            year={year}
            selectedDate={selectedDate}
            onDayClick={handleDayClick}
          />
        </div>

        {/* Day detail panel */}
        {(selectedDate || loadingDetail) && (
          <div className="animate-slide-down">
            <DayDetailPanel
              date={selectedDate}
              detail={loadingDetail ? null : dayDetail}
              loading={loadingDetail}
              onClose={() => { setSelectedDate(null); setDayDetail(null); }}
            />
          </div>
        )}
      </div>
    </div>
  );
}

function DayDetailPanel({
  date,
  detail,
  loading,
  onClose,
}: {
  date: string | null;
  detail: DayDetail | null;
  loading: boolean;
  onClose: () => void;
}) {
  const displayDate = date ? format(new Date(date + "T12:00:00"), "EEEE, MMMM d, yyyy") : "";

  return (
    <div className="bg-card border border-border rounded-lg p-5">
      <div className="flex items-start justify-between mb-4">
        <div>
          <h3 className="text-sm font-semibold text-foreground">{displayDate}</h3>
          {detail && (
            <p className="text-xs text-muted-foreground mt-0.5">
              {detail.totalMessages} message{detail.totalMessages !== 1 ? "s" : ""} across{" "}
              {detail.sessions.length} session{detail.sessions.length !== 1 ? "s" : ""}
            </p>
          )}
        </div>
        <button
          onClick={onClose}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <X size={14} />
        </button>
      </div>

      {loading && (
        <div className="text-xs text-muted-foreground py-2">Loading…</div>
      )}

      {!loading && detail && detail.sessions.length === 0 && (
        <div className="text-xs text-muted-foreground py-2">No activity recorded.</div>
      )}

      {!loading && detail && detail.sessions.length > 0 && (
        <div className="space-y-2">
          {detail.sessions.map((session) => (
            <div
              key={session.sessionId}
              className="flex items-center justify-between py-2 border-b border-border last:border-0"
            >
              <div className="min-w-0 flex-1">
                <p className="text-xs font-medium text-foreground truncate">
                  {session.title ?? "Untitled session"}
                </p>
                <p className="text-[11px] text-muted-foreground mt-0.5 flex items-center gap-1">
                  <FolderOpen size={10} />
                  {session.projectName}
                </p>
              </div>
              <div className="ml-4 shrink-0 text-right">
                <span className="text-xs text-muted-foreground">
                  {session.userMessageCount} msg{session.userMessageCount !== 1 ? "s" : ""}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function StatCard({
  label,
  value,
  sub,
  icon,
  delta,
  isText,
}: {
  label: string;
  value: number | string;
  sub?: string;
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
          <span className="text-xs mb-0.5 text-muted-foreground">
            <Minus size={11} />
          </span>
        )}
      </div>
      {sub && <p className="text-[11px] text-muted-foreground mt-0.5">{sub}</p>}
    </div>
  );
}
