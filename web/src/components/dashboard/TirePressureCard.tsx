import { useEffect, useMemo, useState } from "react"
import { Disc, Loader2 } from "lucide-react"
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ReferenceArea,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts"

// Tire-pressure zones — labels and styling match Tessie's convention
// (see user's reference screenshot). Each band is a solid-feeling
// translucent block with the label centered vertically inside; the
// boundaries between bands are drawn separately as dashed ReferenceLines
// so the dividers read as a single line, not two stacked borders.
//
// Colour intent: red (unsafe top + bottom), amber (harsher ride near top
// of safe), green (optimal), orange (reduced handling near bottom of safe).
// Opacities are higher than the previous pass so the zones read as
// blocks rather than tints.
const ZONES = [
  {
    y1: 50,
    y2: 60,
    fill: "rgba(127, 29, 29, 0.55)",
    label: ">50 PSI • UNSAFE",
    labelColor: "#fca5a5",
  },
  {
    y1: 45,
    y2: 50,
    fill: "rgba(63, 98, 18, 0.55)",
    label: ">45 PSI • HARSHER RIDE & WEAR",
    labelColor: "#bef264",
  },
  {
    y1: 36,
    y2: 45,
    fill: "rgba(22, 78, 51, 0.55)",
    label: "OPTIMAL",
    labelColor: "rgba(167, 243, 208, 0.85)",
  },
  {
    y1: 28,
    y2: 36,
    fill: "rgba(124, 45, 18, 0.55)",
    label: "<36 PSI • REDUCED HANDLING & EFFICIENCY",
    labelColor: "#fcd34d",
  },
  {
    y1: 15,
    y2: 28,
    fill: "rgba(127, 29, 29, 0.55)",
    label: "<28 PSI • UNSAFE",
    labelColor: "#fca5a5",
  },
] as const

// Interior boundaries (dashed lines drawn between adjacent zones).
// Colour-coded to the warning band immediately above/below so the
// divider reads as a transition cue, not chrome.
const ZONE_BOUNDARIES = [
  { y: 50, color: "rgba(252, 165, 165, 0.7)" }, // red boundary above harsh
  { y: 45, color: "rgba(190, 242, 100, 0.7)" }, // amber/lime above optimal
  { y: 36, color: "rgba(252, 211, 77, 0.7)" }, // amber above reduced
  { y: 28, color: "rgba(252, 165, 165, 0.7)" }, // red above bottom-unsafe
] as const

// Y range chosen so the visible bottom "UNSAFE" band has the same
// presence Tessie gives it (~25-30% of the chart height). Going below
// 20 just wastes space — tires never read that low in practice.
const Y_DOMAIN: [number, number] = [20, 55]

// Per-tire line colours — green family to match Tessie's all-green
// tracings while keeping the four wheels distinguishable on hover.
// Picked far enough apart in lightness/hue that they don't melt into
// each other or into the green OPTIMAL band when stacked.
const TIRE_COLORS = {
  fl: "#34d399", // emerald-400  — front-left
  fr: "#a3e635", // lime-400     — front-right
  rl: "#5eead4", // teal-300     — rear-left
  rr: "#facc15", // yellow-400   — rear-right (warm contrast against the greens)
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
                  fill={z.fill}
                  stroke="transparent"
                  label={{
                    value: z.label,
                    position: "center",
                    fill: z.labelColor,
                    fontSize: 10,
                    fontWeight: 600,
                    letterSpacing: "0.08em",
                  }}
                  ifOverflow="hidden"
                />
              ))}
              {ZONE_BOUNDARIES.map((b) => (
                <ReferenceLine
                  key={b.y}
                  y={b.y}
                  stroke={b.color}
                  strokeWidth={1}
                  strokeDasharray="6 4"
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
