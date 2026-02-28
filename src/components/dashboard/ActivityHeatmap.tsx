import { useMemo, useState } from "react";
import { format, startOfYear, endOfYear, eachDayOfInterval, getDay } from "date-fns";
import type { HeatmapDay } from "@/types";

interface ActivityHeatmapProps {
  data: HeatmapDay[];
  year: number;
  selectedDate: string | null;
  onDayClick: (date: string) => void;
}

const DAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** Compute quartile thresholds from non-zero days — same approach as GitHub */
function computeThresholds(data: HeatmapDay[]): [number, number, number, number] {
  const nonZero = data
    .filter((d) => d.count > 0)
    .map((d) => d.count)
    .sort((a, b) => a - b);

  if (nonZero.length === 0) return [1, 2, 4, 8];

  const at = (p: number) => {
    const idx = Math.max(0, Math.floor((p / 100) * nonZero.length) - 1);
    return nonZero[idx];
  };

  return [at(25), at(50), at(75), nonZero[nonZero.length - 1]];
}

function heatLevel(count: number, thresholds: [number, number, number, number]): number {
  if (count === 0) return 0;
  if (count <= thresholds[0]) return 1;
  if (count <= thresholds[1]) return 2;
  if (count <= thresholds[2]) return 3;
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
  text: string;
  x: number;
  y: number;
}

export function ActivityHeatmap({ data, year, selectedDate, onDayClick }: ActivityHeatmapProps) {
  const [tooltip, setTooltip] = useState<TooltipState | null>(null);

  const countByDate = useMemo(() => {
    const map: Record<string, number> = {};
    for (const d of data) map[d.date] = d.count;
    return map;
  }, [data]);

  const thresholds = useMemo(() => computeThresholds(data), [data]);

  const { weeks, monthLabels } = useMemo(() => {
    const start = startOfYear(new Date(year, 0, 1));
    const end = endOfYear(new Date(year, 0, 1));
    const days = eachDayOfInterval({ start, end });

    const firstDow = getDay(start);
    const padded: (Date | null)[] = [...Array(firstDow).fill(null), ...days];

    const weeks: (Date | null)[][] = [];
    for (let i = 0; i < padded.length; i += 7) {
      weeks.push(padded.slice(i, i + 7));
    }

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
      <div className="relative h-4 ml-8 mb-0.5">
        {monthLabels.map(({ label, weekIndex }) => (
          <span
            key={label}
            className="absolute text-[10px] text-muted-foreground"
            style={{ left: weekIndex * step }}
          >
            {label}
          </span>
        ))}
      </div>

      <div className="flex">
        {/* Day-of-week labels */}
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
                const level = heatLevel(count, thresholds);
                const isSelected = dateStr === selectedDate;
                const isFuture = day > new Date();

                return (
                  <div
                    key={di}
                    className="rounded-sm transition-all cursor-pointer"
                    style={{
                      width: cellSize,
                      height: cellSize,
                      backgroundColor: HEAT_COLORS[level],
                      opacity: isFuture ? 0.3 : 1,
                      outline: isSelected ? "2px solid hsl(var(--primary))" : undefined,
                      outlineOffset: isSelected ? "1px" : undefined,
                    }}
                    onClick={() => !isFuture && count > 0 && onDayClick(dateStr)}
                    onMouseEnter={(e) => {
                      const rect = e.currentTarget.getBoundingClientRect();
                      const parent = e.currentTarget.closest(".heatmap-root")?.getBoundingClientRect();
                      setTooltip({
                        text: count === 0
                          ? `No activity on ${format(day, "MMM d")}`
                          : `${count} message${count !== 1 ? "s" : ""} on ${format(day, "MMM d, yyyy")}`,
                        x: rect.left - (parent?.left ?? 0) + cellSize / 2,
                        y: rect.top - (parent?.top ?? 0) - 34,
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

      {/* Hover tooltip */}
      {tooltip && (
        <div
          className="absolute z-10 pointer-events-none bg-popover border border-border rounded px-2 py-1 text-[11px] text-foreground shadow-md whitespace-nowrap"
          style={{ left: tooltip.x, top: tooltip.y, transform: "translateX(-50%)" }}
        >
          {tooltip.text}
        </div>
      )}
    </div>
  );
}
