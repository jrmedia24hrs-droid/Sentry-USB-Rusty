import { useMemo } from "react"
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts"

export interface TemperaturePoint {
  // Unix ms — backend (drives_handler::temperature_series) emits the
  // sample's `ts` already multiplied by 1000 so JS Date and recharts
  // (which both work in ms) consume it directly.
  ts: number
  // Either or both may be undefined when the underlying telemetry row
  // had NULL for that column. recharts skips undefined values in line
  // series so a gap renders as a discontinuity, not a drop to zero.
  interiorC?: number
  exteriorC?: number
}

interface TemperatureChartProps {
  points: TemperaturePoint[]
  // True when DRIVE_MAP_UNIT === "km" (so the user prefers metric —
  // we mirror that for temperature with °C). False → °F.
  metric: boolean
}

const INTERIOR_COLOR = "#f97316" // orange — warm = inside
const EXTERIOR_COLOR = "#38bdf8" // sky — cool = outside (sky-500-ish)

export default function TemperatureChart({ points, metric }: TemperatureChartProps) {
  // Convert °C → °F once at the boundary so the chart's data, axis,
  // and tooltip all speak the same unit. Skipping with `undefined`
  // preserves the gap-handling behaviour from the raw payload.
  const converted = useMemo(() => {
    if (metric) return points
    return points.map((p) => ({
      ts: p.ts,
      interiorC: p.interiorC === undefined ? undefined : (p.interiorC * 9) / 5 + 32,
      exteriorC: p.exteriorC === undefined ? undefined : (p.exteriorC * 9) / 5 + 32,
    }))
  }, [points, metric])

  const unit = metric ? "°C" : "°F"

  return (
    <div className="h-56 w-full" aria-label="Temperature chart">
      <ResponsiveContainer>
        <LineChart
          data={converted}
          margin={{ top: 10, right: 16, bottom: 24, left: 4 }}
        >
          <CartesianGrid stroke="#1e242f" strokeDasharray="3 3" vertical={false} />
          <XAxis
            dataKey="ts"
            type="number"
            domain={["dataMin", "dataMax"]}
            tickFormatter={formatTick}
            stroke="#475569"
            tick={{ fill: "#64748b", fontSize: 11 }}
            tickLine={false}
            axisLine={false}
            tickMargin={10}
            minTickGap={56}
            padding={{ left: 10, right: 4 }}
          />
          <YAxis
            stroke="#475569"
            tick={{ fill: "#64748b", fontSize: 11 }}
            tickFormatter={(n: number) => `${Math.round(n)}${unit}`}
            tickLine={false}
            axisLine={false}
            tickMargin={4}
            width={44}
            domain={["dataMin - 2", "dataMax + 2"]}
          />
          <Tooltip
            content={({ active, payload }) => {
              if (!active || !payload || payload.length === 0) return null
              const p = payload[0].payload as TemperaturePoint
              return (
                <div className="rounded-md border border-white/10 bg-slate-900/95 px-2 py-1.5 text-xs text-slate-200 shadow-xl">
                  <div className="mb-1 text-[10px] text-slate-500 tabular-nums">
                    {formatTooltipTime(p.ts)}
                  </div>
                  {p.interiorC !== undefined && (
                    <div className="flex items-center gap-2 tabular-nums">
                      <span
                        className="inline-block h-2 w-2 rounded-full"
                        style={{ background: INTERIOR_COLOR }}
                        aria-hidden
                      />
                      <span className="text-slate-400">Interior</span>
                      <span className="ml-auto font-medium">
                        {Math.round(p.interiorC)}
                        {unit}
                      </span>
                    </div>
                  )}
                  {p.exteriorC !== undefined && (
                    <div className="flex items-center gap-2 tabular-nums">
                      <span
                        className="inline-block h-2 w-2 rounded-full"
                        style={{ background: EXTERIOR_COLOR }}
                        aria-hidden
                      />
                      <span className="text-slate-400">Exterior</span>
                      <span className="ml-auto font-medium">
                        {Math.round(p.exteriorC)}
                        {unit}
                      </span>
                    </div>
                  )}
                </div>
              )
            }}
          />
          <Legend
            verticalAlign="bottom"
            height={20}
            iconType="line"
            wrapperStyle={{ fontSize: 11, color: "#94a3b8" }}
          />
          <Line
            type="monotone"
            name="Interior"
            dataKey="interiorC"
            stroke={INTERIOR_COLOR}
            strokeWidth={2}
            dot={false}
            isAnimationActive={false}
            connectNulls
          />
          <Line
            type="monotone"
            name="Exterior"
            dataKey="exteriorC"
            stroke={EXTERIOR_COLOR}
            strokeWidth={2}
            dot={false}
            isAnimationActive={false}
            connectNulls
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  )
}

function formatTick(ms: number): string {
  const t = new Date(ms)
  if (Number.isNaN(t.getTime())) return ""
  return t.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" })
}

function formatTooltipTime(ms: number): string {
  const t = new Date(ms)
  if (Number.isNaN(t.getTime())) return ""
  return t.toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
    second: "2-digit",
  })
}
