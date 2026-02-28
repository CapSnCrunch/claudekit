export interface ProjectSummary {
  id: string;
  decodedPath: string;
  displayName: string;
  sessionCount: number;
  lastActive: string | null;
}

export interface SessionSummary {
  id: string;
  projectId: string;
  title: string | null;
  messageCount: number;
  userMessageCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface MessageRecord {
  id: string;
  sessionId: string;
  role: string;
  isSummary: boolean;
  contentJson: string;
  timestamp: string;
  ordinal: number;
  model: string | null;
  inputTokens: number | null;
  outputTokens: number | null;
}

export interface DashboardStats {
  totalSessions: number;
  totalProjects: number;
  sessionsThisWeek: number;
  sessionsLastWeek: number;
  mostActiveProject: ProjectSummary | null;
}

export interface HeatmapDay {
  date: string;
  count: number;
}

export interface DaySession {
  sessionId: string;
  projectName: string;
  title: string | null;
  userMessageCount: number;
}

export interface DayDetail {
  date: string;
  totalMessages: number;
  sessions: DaySession[];
}

export interface IndexStats {
  projectsIndexed: number;
  sessionsIndexed: number;
  messagesIndexed: number;
  durationMs: number;
}

export interface SessionInfo {
  sessionId: string;
  title: string | null;
  projectId: string;
  projectDecodedPath: string;
}

// Anthropic content block types (parsed from MessageRecord.contentJson)
export type ContentBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; id: string; name: string; input: Record<string, unknown> }
  | { type: "tool_result"; tool_use_id: string; content: string | ContentBlock[]; is_error?: boolean }
  | { type: "image"; source: { type: "base64"; media_type: string; data: string } };
