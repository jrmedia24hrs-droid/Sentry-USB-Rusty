import { useEffect, useState } from "react"
import { useSearchParams } from "react-router-dom"
import {
  RefreshCw,
  Stethoscope,
  Gauge,
  Settings as SettingsIcon,
  Unplug,
  RotateCcw,
} from "lucide-react"
import { api } from "@/lib/api"
import { SetupWizard } from "@/components/setup/SetupWizard"
import { HeaderStrip } from "@/components/settings/HeaderStrip"
import { ActionsRail, type ActionChipProps } from "@/components/settings/ActionsRail"
import { TabBar } from "@/components/ui/TabBar"
import { RawConfigEditor, type RawConfigEntry } from "@/components/settings/sections/RawConfigEditor"
import { HealthCheckModal } from "@/components/settings/sections/HealthCheckModal"
import { SpeedTestModal } from "@/components/settings/sections/SpeedTestModal"
import { DeviceTab } from "@/pages/settings/DeviceTab"
import { NetworkTab } from "@/pages/settings/NetworkTab"
import { UpdatesTab } from "@/pages/settings/UpdatesTab"
import { BackupsTab } from "@/pages/settings/BackupsTab"
import { NotificationsTab } from "@/pages/settings/NotificationsTab"
import { AboutTab } from "@/pages/settings/AboutTab"
import type { PiStatus } from "@/lib/api"

const TABS = [
  "Device",
  "Network",
  "Updates",
  "Backups",
  "Notifications",
  "About",
] as const
type TabName = (typeof TABS)[number]

function isTab(s: string | null): s is TabName {
  return !!s && (TABS as readonly string[]).includes(s)
}

export default function Settings() {
  const [params, setParams] = useSearchParams()
  const activeTab: TabName = isTab(params.get("tab")) ? (params.get("tab") as TabName) : "Device"

  const [status, setStatus] = useState<PiStatus | null>(null)
  const [piConfig, setPiConfig] = useState<{
    uses_ble?: string
    SENTRYUSB_HOSTNAME?: string
  } | null>(null)
  const [confirmReboot, setConfirmReboot] = useState(false)
  const [drivesConnected, setDrivesConnected] = useState<boolean | null>(null)
  // Local 1s tick so the header strip uptime advances between status polls
  // (status itself only refreshes every 4s — too jumpy for a wall clock).
  const [tickOffset, setTickOffset] = useState(0)

  // Modal state
  const [wizardOpen, setWizardOpen] = useState(false)
  const [wizardInitialData, setWizardInitialData] = useState<
    Record<string, string> | undefined
  >(undefined)
  const [rawConfigOpen, setRawConfigOpen] = useState(false)
  const [rawConfig, setRawConfig] = useState<Record<string, RawConfigEntry> | null>(null)
  const [healthOpen, setHealthOpen] = useState(false)
  const [speedOpen, setSpeedOpen] = useState(false)

  // Status poll (drives the actions rail USB toggle + header strip uptime)
  useEffect(() => {
    let mounted = true
    async function poll() {
      try {
        const data = await api.getStatus()
        if (mounted) {
          setStatus(data)
          setDrivesConnected(data.drives_active === "yes")
          setTickOffset(0) // reset local tick when we get a fresh server value
        }
      } catch {
        /* ignore */
      }
    }
    poll()
    const id = setInterval(poll, 4000)
    const tickId = setInterval(() => setTickOffset((t) => t + 1), 1000)
    return () => {
      mounted = false
      clearInterval(id)
      clearInterval(tickId)
    }
  }, [])

  // Pi config (uses_ble, SENTRYUSB_HOSTNAME). SBC model now comes from the
  // status payload (which already detects Pi 5 reliably) — the previous
  // rtc-status flag only flipped to is_pi5=true when a battery was present,
  // so a Pi 5 without an RTC battery was mis-labelled as "Pi 4 / earlier".
  useEffect(() => {
    fetch("/api/config")
      .then((r) => r.json())
      .then((data) => setPiConfig(data))
      .catch(() => {})
  }, [])

  const sbc = status?.sbc_model || null
  // /api/config returns each key as { value, active } OR as a raw string,
  // depending on shape — handle both.
  const hostnameEntry = piConfig?.SENTRYUSB_HOSTNAME as
    | { value?: string; active?: boolean }
    | string
    | undefined
  const hostname =
    typeof hostnameEntry === "string"
      ? hostnameEntry
      : hostnameEntry?.active
      ? hostnameEntry.value || null
      : null

  function setTab(next: TabName) {
    const p = new URLSearchParams(params)
    p.set("tab", next)
    setParams(p, { replace: true })
  }

  async function handleReboot(): Promise<string | void> {
    // First press arms the confirm; the parent re-renders the chip's label
    // ("Restart Pi" → "Confirm Restart"). Returning "confirm" tells the chip
    // not to flash a success state — the label change is the feedback.
    if (!confirmReboot) {
      setConfirmReboot(true)
      setTimeout(() => setConfirmReboot(false), 10000)
      return "confirm"
    }
    const res = await fetch("/api/system/reboot", { method: "POST" })
    setConfirmReboot(false)
    if (!res.ok) throw new Error("Reboot failed")
    return "Rebooting…"
  }

  async function handleToggleDrives(): Promise<string> {
    const res = await fetch("/api/system/toggle-drives", { method: "POST" })
    if (!res.ok) throw new Error("Toggle failed")
    // Eagerly refresh status so the chip label updates to the new state
    // ("USB · Connected" / "USB · Disconnected") on the next render.
    try {
      const data = await api.getStatus()
      setDrivesConnected(data.drives_active === "yes")
    } catch {
      /* non-critical */
    }
    return "Toggled"
  }

  async function handleArchiveSync(): Promise<string> {
    const res = await fetch("/api/system/trigger-sync", { method: "POST" })
    if (!res.ok) throw new Error("Sync failed")
    return "Triggered"
  }

  async function handleOpenRawConfig() {
    try {
      const res = await fetch("/api/setup/config")
      if (!res.ok) return
      const data = await res.json()
      setRawConfig(data)
      setRawConfigOpen(true)
    } catch {
      /* ignore */
    }
  }

  async function handleOpenWizard() {
    try {
      const res = await fetch("/api/setup/config")
      if (res.ok) {
        const data = await res.json()
        const flat: Record<string, string> = {}
        for (const [k, v] of Object.entries(data)) {
          const entry = v as { value: string; active: boolean }
          if (entry.active) flat[k] = entry.value
        }
        setWizardInitialData(flat)
      }
    } catch {
      /* ignore */
    }
    setWizardOpen(true)
  }

  const actions: ActionChipProps[] = [
    {
      icon: RefreshCw,
      label: "Archive Sync",
      onClick: handleArchiveSync,
    },
    {
      icon: Stethoscope,
      label: "Health Check",
      onClick: () => setHealthOpen(true),
    },
    {
      icon: Gauge,
      label: "Speed Test",
      onClick: () => setSpeedOpen(true),
    },
    {
      icon: SettingsIcon,
      label: "Raw Config",
      onClick: handleOpenRawConfig,
    },
    {
      icon: Unplug,
      label:
        drivesConnected === null
          ? "Toggle USB"
          : drivesConnected
          ? "USB · Connected"
          : "USB · Disconnected",
      onClick: handleToggleDrives,
    },
  ]
  const dangerActions: ActionChipProps[] = [
    {
      icon: RotateCcw,
      label: confirmReboot ? "Confirm Restart" : "Restart Pi",
      variant: "danger",
      onClick: handleReboot,
    },
  ]

  const uptimeSec = status ? parseFloat(status.uptime) + tickOffset : null

  // ⚠️ Mobile / tab-bar — switch to scrollable variant under 640px.
  const [isMobile, setIsMobile] = useState(
    typeof window !== "undefined" && window.innerWidth < 640
  )
  useEffect(() => {
    const onResize = () => setIsMobile(window.innerWidth < 640)
    window.addEventListener("resize", onResize)
    return () => window.removeEventListener("resize", onResize)
  }, [])

  return (
    <div className="space-y-3">
      <HeaderStrip
        hostname={hostname}
        sbc={sbc}
        uptimeSec={uptimeSec}
        onOpenWizard={handleOpenWizard}
      />

      <ActionsRail actions={actions} danger={dangerActions} />

      <TabBar tabs={TABS} active={activeTab} onSelect={setTab} scrollable={isMobile} />

      {activeTab === "Device" && <DeviceTab />}
      {activeTab === "Network" && <NetworkTab status={status} />}
      {activeTab === "Updates" && <UpdatesTab />}
      {activeTab === "Backups" && <BackupsTab onOpenRawConfig={handleOpenRawConfig} />}
      {activeTab === "Notifications" && <NotificationsTab />}
      {activeTab === "About" && (
        <AboutTab
          status={status}
          sbc={sbc}
          hostname={hostname}
          uptimeSec={uptimeSec}
          onOpenWizard={handleOpenWizard}
        />
      )}

      {/* Modals */}
      {wizardOpen && (
        <SetupWizard
          initialData={wizardInitialData}
          onClose={() => {
            setWizardOpen(false)
            setWizardInitialData(undefined)
          }}
        />
      )}
      {rawConfigOpen && rawConfig && (
        <RawConfigEditor
          config={rawConfig}
          onClose={() => {
            setRawConfigOpen(false)
            setRawConfig(null)
          }}
        />
      )}
      {healthOpen && <HealthCheckModal onClose={() => setHealthOpen(false)} />}
      {speedOpen && <SpeedTestModal onClose={() => setSpeedOpen(false)} />}
    </div>
  )
}
