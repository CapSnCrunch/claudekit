import { render, screen, act } from "@testing-library/react";
import { vi, describe, it, expect, beforeEach, afterEach } from "vitest";
import App from "@/App";

// Mock child components so App's sync logic can be tested in isolation
vi.mock("@/components/layout/AppShell", () => ({
  AppShell: ({
    children,
    onSync,
    lastSynced,
  }: {
    children: React.ReactNode;
    onSync: () => Promise<void>;
    lastSynced: number;
  }) => (
    <div>
      <button onClick={onSync} data-testid="sync-btn">sync</button>
      <span data-testid="last-synced">{lastSynced}</span>
      {children}
    </div>
  ),
}));
vi.mock("@/components/dashboard/Dashboard", () => ({
  Dashboard: () => <div>Dashboard</div>,
}));
vi.mock("@/components/session/ConversationView", () => ({
  ConversationView: () => <div>ConversationView</div>,
}));

const { mockRunIndex } = vi.hoisted(() => ({ mockRunIndex: vi.fn() }));
vi.mock("@/lib/api", () => ({
  api: { runIndex: mockRunIndex },
}));

const indexResult = {
  projectsIndexed: 1,
  sessionsIndexed: 2,
  messagesIndexed: 10,
  durationMs: 5,
};

describe("App sync logic", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockRunIndex.mockResolvedValue(indexResult);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it("calls runIndex once on mount and shows indexing spinner", async () => {
    let resolveIndex!: () => void;
    mockRunIndex.mockReturnValueOnce(
      new Promise<typeof indexResult>((res) => {
        resolveIndex = () => res(indexResult);
      })
    );

    render(<App />);

    expect(screen.getByText("Indexing sessions…")).toBeInTheDocument();
    expect(mockRunIndex).toHaveBeenCalledTimes(1);

    await act(async () => { resolveIndex(); });

    expect(screen.queryByText("Indexing sessions…")).not.toBeInTheDocument();
  });

  it("hides spinner and updates lastSynced after initial index completes", async () => {
    await act(async () => { render(<App />); });

    expect(screen.queryByText("Indexing sessions…")).not.toBeInTheDocument();
    expect(Number(screen.getByTestId("last-synced").textContent)).toBeGreaterThan(0);
  });

  it("calls runIndex again after 30s interval", async () => {
    await act(async () => { render(<App />); });
    expect(mockRunIndex).toHaveBeenCalledTimes(1);

    await act(async () => { vi.advanceTimersByTime(30_000); });

    expect(mockRunIndex).toHaveBeenCalledTimes(2);
  });

  it("calls runIndex a third time after 60s total", async () => {
    await act(async () => { render(<App />); });

    // Advance in 30s steps so each interval's promise can flush before the next fires
    await act(async () => { vi.advanceTimersByTime(30_000); });
    await act(async () => { vi.advanceTimersByTime(30_000); });

    expect(mockRunIndex).toHaveBeenCalledTimes(3);
  });

  it("does not run concurrent syncs if one is already in progress", async () => {
    // First call: slow index that won't resolve during the test
    let resolveFirst!: () => void;
    mockRunIndex
      .mockReturnValueOnce(new Promise<typeof indexResult>((res) => { resolveFirst = () => res(indexResult); }))
      .mockResolvedValue(indexResult);

    render(<App />);
    // Initial slow index is in progress — fire the 30s interval before it resolves
    await act(async () => { vi.advanceTimersByTime(30_000); });

    // syncingRef guard should have blocked the interval call
    expect(mockRunIndex).toHaveBeenCalledTimes(1);

    // Now let the first one finish and confirm the interval works afterwards
    await act(async () => { resolveFirst(); });
    await act(async () => { vi.advanceTimersByTime(30_000); });
    expect(mockRunIndex).toHaveBeenCalledTimes(2);
  });

  it("clears the interval on unmount", async () => {
    const { unmount } = await act(async () => render(<App />));
    expect(mockRunIndex).toHaveBeenCalledTimes(1);

    unmount();

    await act(async () => { vi.advanceTimersByTime(60_000); });
    // No more calls after unmount
    expect(mockRunIndex).toHaveBeenCalledTimes(1);
  });
});
