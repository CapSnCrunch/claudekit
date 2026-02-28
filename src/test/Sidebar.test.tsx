import { render, screen, act, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import { Sidebar } from "@/components/layout/Sidebar";
import type { ProjectSummary, SessionSummary } from "@/types";

const { mockListProjects, mockListSessions } = vi.hoisted(() => ({
  mockListProjects: vi.fn(),
  mockListSessions: vi.fn(),
}));

vi.mock("@/lib/api", () => ({
  api: {
    listProjects: mockListProjects,
    listSessions: mockListSessions,
  },
}));

const project: ProjectSummary = {
  id: "proj-1",
  decodedPath: "/home/user/proj",
  displayName: "My Project",
  sessionCount: 2,
  lastActive: null,
};

const session: SessionSummary = {
  id: "sess-1",
  projectId: "proj-1",
  title: "Test session",
  messageCount: 4,
  userMessageCount: 2,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

function renderSidebar(overrides: Partial<Parameters<typeof Sidebar>[0]> = {}) {
  const props = {
    selectedProjectId: null,
    selectedSessionId: null,
    onSelectProject: vi.fn(),
    onSelectSession: vi.fn(),
    lastSynced: 0,
    onSync: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  return render(<Sidebar {...props} />);
}

describe("Sidebar sync behaviour", () => {
  beforeEach(() => {
    mockListProjects.mockResolvedValue([project]);
    mockListSessions.mockResolvedValue([session]);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("fetches projects on mount", async () => {
    await act(async () => { renderSidebar(); });
    expect(mockListProjects).toHaveBeenCalledTimes(1);
    expect(screen.getByText("My Project")).toBeInTheDocument();
  });

  it("does not re-fetch when lastSynced is 0 (initial render)", async () => {
    await act(async () => { renderSidebar({ lastSynced: 0 }); });
    // Only the mount effect fires
    expect(mockListProjects).toHaveBeenCalledTimes(1);
  });

  it("re-fetches projects when lastSynced changes to a non-zero value", async () => {
    const { rerender } = await act(async () => renderSidebar({ lastSynced: 0 }));
    expect(mockListProjects).toHaveBeenCalledTimes(1);

    await act(async () => {
      rerender(
        <Sidebar
          selectedProjectId={null}
          selectedSessionId={null}
          onSelectProject={vi.fn()}
          onSelectSession={vi.fn()}
          lastSynced={Date.now()}
          onSync={vi.fn().mockResolvedValue(undefined)}
        />
      );
    });

    expect(mockListProjects).toHaveBeenCalledTimes(2);
  });

  it("re-fetches sessions for expanded projects when lastSynced changes", async () => {
    const user = userEvent.setup();
    const { rerender } = await act(async () => renderSidebar({ lastSynced: 0 }));

    // Expand the project so its sessions are loaded
    await act(async () => { await user.click(screen.getByText("My Project")); });
    expect(mockListSessions).toHaveBeenCalledWith("proj-1");
    mockListSessions.mockClear();

    // Simulate a sync
    await act(async () => {
      rerender(
        <Sidebar
          selectedProjectId={null}
          selectedSessionId={null}
          onSelectProject={vi.fn()}
          onSelectSession={vi.fn()}
          lastSynced={Date.now()}
          onSync={vi.fn().mockResolvedValue(undefined)}
        />
      );
    });

    expect(mockListSessions).toHaveBeenCalledWith("proj-1");
  });

  it("does not re-fetch sessions for collapsed projects when lastSynced changes", async () => {
    const { rerender } = await act(async () => renderSidebar({ lastSynced: 0 }));
    // Project was never expanded, so no sessions were fetched
    mockListSessions.mockClear();

    await act(async () => {
      rerender(
        <Sidebar
          selectedProjectId={null}
          selectedSessionId={null}
          onSelectProject={vi.fn()}
          onSelectSession={vi.fn()}
          lastSynced={Date.now()}
          onSync={vi.fn().mockResolvedValue(undefined)}
        />
      );
    });

    expect(mockListSessions).not.toHaveBeenCalled();
  });

  it("calls onSync when the sync button is clicked", async () => {
    const user = userEvent.setup();
    const onSync = vi.fn().mockResolvedValue(undefined);
    await act(async () => { renderSidebar({ onSync }); });

    await user.click(screen.getByTitle("Sync sessions"));

    expect(onSync).toHaveBeenCalledTimes(1);
  });

  it("disables the sync button while syncing", async () => {
    const user = userEvent.setup();
    let resolveSync!: () => void;
    const onSync = vi.fn().mockReturnValue(
      new Promise<void>((res) => { resolveSync = res; })
    );
    await act(async () => { renderSidebar({ onSync }); });

    const btn = screen.getByTitle("Sync sessions");
    expect(btn).not.toBeDisabled();

    // Start sync but don't await — button should become disabled
    act(() => { user.click(btn); });
    await waitFor(() => expect(btn).toBeDisabled());

    // Resolve sync — button re-enables
    await act(async () => { resolveSync(); });
    expect(btn).not.toBeDisabled();
  });

  it("shows spinner animation on sync button while syncing", async () => {
    const user = userEvent.setup();
    let resolveSync!: () => void;
    const onSync = vi.fn().mockReturnValue(
      new Promise<void>((res) => { resolveSync = res; })
    );
    await act(async () => { renderSidebar({ onSync }); });

    const btn = screen.getByTitle("Sync sessions");
    const icon = btn.querySelector("svg")!;
    expect(icon).not.toHaveClass("animate-spin");

    act(() => { user.click(btn); });
    await waitFor(() => expect(icon).toHaveClass("animate-spin"));

    await act(async () => { resolveSync(); });
    expect(icon).not.toHaveClass("animate-spin");
  });
});
