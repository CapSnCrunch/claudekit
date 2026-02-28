import { invoke } from "@tauri-apps/api/core";
import type {
  ProjectSummary,
  SessionSummary,
  MessageRecord,
  DashboardStats,
  HeatmapDay,
  DayDetail,
  IndexStats,
} from "@/types";

export const api = {
  runIndex: () =>
    invoke<IndexStats>("run_index"),

  listProjects: () =>
    invoke<ProjectSummary[]>("list_projects"),

  listSessions: (projectId: string, limit?: number, offset?: number) =>
    invoke<SessionSummary[]>("list_sessions", { projectId, limit, offset }),

  getSessionMessages: (sessionId: string) =>
    invoke<MessageRecord[]>("get_session_messages", { sessionId }),

  getDashboardStats: () =>
    invoke<DashboardStats>("get_dashboard_stats"),

  getHeatmapData: (year?: number) =>
    invoke<HeatmapDay[]>("get_heatmap_data", { year }),

  getDayDetail: (date: string) =>
    invoke<DayDetail>("get_day_detail", { date }),
};
