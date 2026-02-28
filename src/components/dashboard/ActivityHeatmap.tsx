import { useMemo, useState } from "react";
import { format, startOfYear, endOfYear, eachDayOfInterval, getDay } from "date-fns";
import type { HeatmapDay } from "@/types";

interface ActivityHeatmapProps {
  data: HeatmapDay[];
  year: number;
}

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

function heatLevel(count: number): number {
  if (count === 0) return 0;
  if (count <= 3) return 1;
  if (count <= 8) return 2;
  if (count <= 15) return 3;
  return 4;
}

const HEAT_COLORS: Record<number, string> = {
  0: "hsl(var(--heat-0))",
  1: "hsl(var(--heat-1))",
  2: "hsl(var(--heat-2))",
  3: "hsl(var(--heat-3))",
  4: "hsl(var(--heat-4))",
};

interface TooltipState {
  date: string;
  count: number;
  x: number;
  y: number;
}

export function ActivityHeatmap({ data, year }: ActivityHeatmapProps) {
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);

  const countByDate = useMemo(() => {
    const map: Record<string, number> = {};
    for (const d of data) map[d.date] = d.count;
    return map;
  }, [data]);

  // Build a 2D grid: weeks (cols) × days (rows)
  const { weeks, monthLabels } = useMemo(() => {
    const start = startOfYear(new Date(year, 0, 1));
    const end = endOfYear(new Date(year, 0, 1));
    const days = eachDayOfInterval({ start, end });

    // Pad start so first day lands on correct weekday column
    const firstDow = getDay(start); // 0=Sun
    const padded: (Date | null)[] = [...Array(firstDow).fill(null), ...days];

    // Chunk into weeks
    const weeks: (Date | null)[][] = [];
    for (let i = 0; i < padded.length; i += 7) {
      weeks.push(padded.slice(i, i + 7));
    }

    // Month label positions: find first week that contains the 1st of each month
    const monthLabels: { label: string; weekIndex: number }[] = [];
    let lastMonth = -1;
    weeks.forEach((week, wi) => {
      for (const d of week) {
        if (d && d.getMonth() !== lastMonth) {
          lastMonth = d.getMonth();
          monthLabels.push({ label: MONTHS[d.getMonth()], weekIndex: wi });
        }
      }
    });

    return { weeks, monthLabels };
  }, [year]);

  const cellSize = 12;
  const gap = 2;
  const step = cellSize + gap;

  return (
    <div className="relative select-none">
      {/* Month labels */}
      <div className="flex mb-1 ml-8 text-[10px] text-muted-foreground">
        {monthLabels.map(({ label, weekIndex }) => (
          <span
            key={label}
            className="absolute text-[10px] text-muted-foreground"
            style={{ left: 32 + weekIndex * step }}
          >
            {label}
          </span>
        ))}
      </div>

      <div className="flex mt-5">
        {/* Day labels */}
        <div className="flex flex-col mr-1.5" style={{ gap }}>
          {DAYS.map((d, i) => (
            <div
              key={d}
              className="text-[9px] text-muted-foreground flex items-center justify-end pr-1"
              style={{ height: cellSize, visibility: i % 2 === 1 ? "visible" : "hidden" }}
            >
              {d}
            </div>
          ))}
        </div>

        {/* Grid */}
        <div className="flex" style={{ gap }}>
          {weeks.map((week, wi) => (
            <div key={wi} className="flex flex-col" style={{ gap }}>
              {week.map((day, di) => {
                if (!day) {
                  return <div key={di} style={{ width: cellSize, height: cellSize }} />;
                }
                const dateStr = format(day, "yyyy-MM-dd");
                const count = countByDate[dateStr] ?? 0;
                const level = heatLevel(count);

                return (
                  <div
                    key={di}
                    className="rounded-sm cursor-default transition-opacity hover:opacity-80"
                    style={{
                      width: cellSize,
                      height: cellSize,
                      backgroundColor: HEAT_COLORS[level],
                    }}
                    onMouseEnter={(e) => {
                      const rect = e.currentTarget.getBoundingClientRect();
                      const parent = e.currentTarget.closest(".heatmap-root")?.getBoundingClientRect();
                      setTooltip({
                        date: format(day, "MMM d, yyyy"),
                        count,
                        x: rect.left - (parent?.left ?? 0) + cellSize / 2,
                        y: rect.top - (parent?.top ?? 0) - 36,
                      });
                    }}
                    onMouseLeave={() => setTooltip(null)}
                  />
                );
              })}
            </div>
          ))}
        </div>
      </div>

      {/* Legend */}
      <div className="flex items-center gap-1.5 mt-3 justify-end">
        <span className="text-[10px] text-muted-foreground">Less</span>
        {[0, 1, 2, 3, 4].map((l) => (
          <div
            key={l}
            className="rounded-sm"
            style={{ width: cellSize, height: cellSize, backgroundColor: HEAT_COLORS[l] }}
          />
        ))}
        <span className="text-[10px] text-muted-foreground">More</span>
      </div>

      {/* Tooltip */}
      {tooltip && (
        <div
          className="absolute z-10 pointer-events-none bg-popover border border-border rounded px-2 py-1 text-[11px] text-foreground shadow-md whitespace-nowrap"
          style={{ left: tooltip.x, top: tooltip.y, transform: "translateX(-50%)" }}
        >
          <span className="font-medium">{tooltip.count} message{tooltip.count !== 1 ? "s" : ""}</span>
          <span className="text-muted-foreground ml-1">on {tooltip.date}</span>
        </div>
      )}
    </div>
  );
}
