import { useEffect, useMemo, useState } from "react"
import { Disc, Loader2 } from "lucide-react"
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ReferenceArea,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts"

// Tire-pressure zones — labels match the Tessie convention. The
// "Optimal" band intentionally has no explicit numeric label inside
// the chart (the surrounding bands' edges already imply the range).
// Colour intent:
//   red      → unsafe (top & bottom)
//   amber    → harsher ride / wear (high end of safe)
//   green    → optimal
//   orange   → reduced handling / efficiency (low end of safe)
const ZONES = [
  { y1: 50, y2: 60, color: "rgba(239, 68, 68, 0.18)", label: ">50 PSI • UNSAFE" },
  {
    y1: 45,
    y2: 50,
    color: "rgba(251, 191, 36, 0.16)",
    label: ">45 PSI • HARSHER RIDE & WEAR",
  },
  { y1: 36, y2: 45, color: "rgba(52, 211, 153, 0.14)", label: "OPTIMAL" },
  {
    y1: 28,
    y2: 36,
    color: "rgba(249, 115, 22, 0.16)",
    label: "<36 PSI • REDUCED HANDLING & EFFICIENCY",
  },
  { y1: 15, y2: 28, color: "rgba(239, 68, 68, 0.18)", label: "<28 PSI • UNSAFE" },
] as const

const Y_DOMAIN: [number, number] = [25, 55]

// Per-tire line colours — distinct hues so all four read on top of
// the coloured zone bands. Picked to avoid the band colours (red,
// amber, green, orange) so the lines never blend into a backdrop.
const TIRE_COLORS = {
  fl: "#60a5fa", // sky-400  — front-left
  fr: "#a78bfa", // violet-400 — front-right
  rl: "#22d3ee", // cyan-400  — rear-left
  rr: "#f472b6", // pink-400  — rear-right
} as const

interface TirePoint {
  ts: number
  fl?: number
  fr?: number
  rl?: number
  rr?: number
}

interface TireHistoryResponse {
  points: TirePoint[]
  days: number
}

interface TirePressureCardProps {
  // Days of history to request from the backend. The card itself is
  // currently fixed at 30 days but the prop lets callers adjust without
  // touching this component, and matches the backend's `?days=` shape.
  days?: number
}

export function TirePressureCard({ days = 30 }: TirePressureCardProps) {
  const [data, setData] = useState<TireHistoryResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    setError(null)
    fetch(`/api/telemetry/tire-history?days=${days}`)
      .then((r) =>
        r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`)),
      )
      .then((d: TireHistoryResponse) => {
        if (cancelled) return
        setData(d)
      })
      .catch((e: unknown) => {
        if (cancelled) return
        setError(e instanceof Error ? e.message : String(e))
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [days])

  // Latest reading per tire for the header strip — same shape as the
  // four green-check tiles in Tessie's screenshot, but inline so the
  // card stays compact for the dashboard.
  const latest = useMemo(() => {
    const points = data?.points ?? []
    const out: Partial<Record<"fl" | "fr" | "rl" | "rr", number>> = {}
    for (let i = points.length - 1; i >= 0; i--) {
      const p = points[i]
      if (out.fl === undefined && p.fl !== undefined) out.fl = p.fl
      if (out.fr === undefined && p.fr !== undefined) out.fr = p.fr
      if (out.rl === undefined && p.rl !== undefined) out.rl = p.rl
      if (out.rr === undefined && p.rr !== undefined) out.rr = p.rr
      if (
        out.fl !== undefined &&
        out.fr !== undefined &&
        out.rl !== undefined &&
        out.rr !== undefined
      )
        break
    }
    return out
  }, [data])

  return (
    <div className="glass-card p-4">
      <div className="mb-3 flex flex-wrap items-center gap-3">
        <span className="tile-icon halo-blue">
          <Disc className="h-4 w-4" />
        </span>
        <div className="min-w-0">
          <div className="text-sm font-semibold text-slate-100">
            Tire pressure
          </div>
          <div className="text-[11px] uppercase tracking-wider text-slate-500">
            Last {days} days
          </div>
        </div>
        <div className="ml-auto flex flex-wrap gap-3 text-xs tabular-nums text-slate-300">
          <LatestChip label="FL" value={latest.fl} color={TIRE_COLORS.fl} />
          <LatestChip label="FR" value={latest.fr} color={TIRE_COLORS.fr} />
          <LatestChip label="RL" value={latest.rl} color={TIRE_COLORS.rl} />
          <LatestChip label="RR" value={latest.rr} color={TIRE_COLORS.rr} />
        </div>
      </div>

      {loading && (
        <div className="flex h-72 items-center justify-center gap-2 text-sm text-slate-500">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading tire history…
        </div>
      )}
      {!loading && error && (
        <div className="flex h-72 items-center justify-center text-sm text-rose-300">
          Failed to load tire history: {error}
        </div>
      )}
      {!loading && !error && (data?.points.length ?? 0) === 0 && (
        <div className="flex h-72 items-center justify-center text-sm text-slate-500">
          No tire-pressure samples in the last {days} days.
        </div>
      )}
      {!loading && !error && (data?.points.length ?? 0) > 0 && (
        <div className="h-72 w-full" aria-label="Tire pressure chart">
          <ResponsiveContainer>
            <LineChart
              data={data!.points}
              margin={{ top: 8, right: 20, bottom: 24, left: 0 }}
            >
              <CartesianGrid
                stroke="#1e242f"
                strokeDasharray="3 3"
                vertical={false}
              />
              {ZONES.map((z) => (
                <ReferenceArea
                  key={z.label}
                  y1={z.y1}
                  y2={z.y2}
                  fill={z.color}
                  stroke="transparent"
                  label={{
                    value: z.label,
                    position: "insideTop",
                    fill: "rgba(226,232,240,0.55)",
                    fontSize: 9,
                    fontWeight: 600,
                    letterSpacing: "0.08em",
                  }}
                  ifOverflow="hidden"
                />
              ))}
              <XAxis
                dataKey="ts"
                type="number"
                domain={["dataMin", "dataMax"]}
                tickFormatter={formatXTick}
                stroke="#475569"
                tick={{ fill: "#64748b", fontSize: 11 }}
                tickLine={false}
                axisLine={false}
                tickMargin={10}
                minTickGap={64}
              />
              <YAxis
                domain={Y_DOMAIN}
                stroke="#475569"
                tick={{ fill: "#64748b", fontSize: 11 }}
                tickFormatter={(n: number) => `${Math.round(n)}`}
                tickLine={false}
                axisLine={false}
                tickMargin={4}
                width={32}
              />
              <Tooltip
                content={({ active, payload }) => {
                  if (!active || !payload || payload.length === 0) return null
                  const p = payload[0].payload as TirePoint
                  return (
                    <div className="rounded-md border border-white/10 bg-slate-900/95 px-2 py-1.5 text-xs text-slate-200 shadow-xl">
                      <div className="mb-1 text-[10px] text-slate-500 tabular-nums">
                        {formatTooltipTime(p.ts)}
                      </div>
                      <TooltipRow label="FL" value={p.fl} color={TIRE_COLORS.fl} />
                      <TooltipRow label="FR" value={p.fr} color={TIRE_COLORS.fr} />
                      <TooltipRow label="RL" value={p.rl} color={TIRE_COLORS.rl} />
                      <TooltipRow label="RR" value={p.rr} color={TIRE_COLORS.rr} />
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
                name="Front L"
                dataKey="fl"
                stroke={TIRE_COLORS.fl}
                strokeWidth={1.75}
                dot={false}
                isAnimationActive={false}
                connectNulls
              />
              <Line
                type="monotone"
                name="Front R"
                dataKey="fr"
                stroke={TIRE_COLORS.fr}
                strokeWidth={1.75}
                dot={false}
                isAnimationActive={false}
                connectNulls
              />
              <Line
                type="monotone"
                name="Rear L"
                dataKey="rl"
                stroke={TIRE_COLORS.rl}
                strokeWidth={1.75}
                dot={false}
                isAnimationActive={false}
                connectNulls
              />
              <Line
                type="monotone"
                name="Rear R"
                dataKey="rr"
                stroke={TIRE_COLORS.rr}
                strokeWidth={1.75}
                dot={false}
                isAnimationActive={false}
                connectNulls
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  )
}

function LatestChip({
  label,
  value,
  color,
}: {
  label: string
  value: number | undefined
  color: string
}) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span
        className="inline-block h-2 w-2 rounded-full"
        style={{ background: color }}
        aria-hidden
      />
      <span className="text-slate-500">{label}</span>
      <span className="text-slate-100">
        {value !== undefined ? `${value.toFixed(1)} psi` : "—"}
      </span>
    </span>
  )
}

function TooltipRow({
  label,
  value,
  color,
}: {
  label: string
  value: number | undefined
  color: string
}) {
  return (
    <div className="flex items-center gap-2 tabular-nums">
      <span
        className="inline-block h-2 w-2 rounded-full"
        style={{ background: color }}
        aria-hidden
      />
      <span className="text-slate-400">{label}</span>
      <span className="ml-auto font-medium">
        {value !== undefined ? `${value.toFixed(1)} psi` : "—"}
      </span>
    </div>
  )
}

function formatXTick(ms: number): string {
  const t = new Date(ms)
  if (Number.isNaN(t.getTime())) return ""
  return t.toLocaleDateString([], { month: "short", day: "numeric" })
}

function formatTooltipTime(ms: number): string {
  const t = new Date(ms)
  if (Number.isNaN(t.getTime())) return ""
  return t.toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  })
}
