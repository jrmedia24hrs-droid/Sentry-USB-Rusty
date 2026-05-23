import { useState, useEffect } from "react"
import { ShieldCheck, ShieldAlert } from "lucide-react"
import { PrefCard } from "@/components/settings/PrefCard"
import { Toggle } from "@/components/ui/Toggle"
import { Pill } from "@/components/ui/Pill"

/**
 * Master Tesla-BLE enable/disable toggle.
 *
 * Sits beside `BlePairButton` in the Device settings tab. Flipping
 * this off is the user's "kill switch" for the Pi-as-phone-key
 * proximity-unlock concern: with BLE disabled the keep-awake nudge,
 * telemetry sampler, and pairing handshake all refuse to run. The
 * iOS-app GATT daemon stays unaffected — this only governs Pi → car
 * commands.
 */
export function BleEnableToggle() {
  const [enabled, setEnabled] = useState<boolean | null>(null)
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)

  useEffect(() => {
    fetch("/api/system/ble-enabled")
      .then((r) => r.json())
      .then((d) => setEnabled(Boolean(d?.enabled)))
      .catch(() => setEnabled(false))
  }, [])

  async function handleToggle(next: boolean) {
    setBusy(true)
    setErr(null)
    try {
      const res = await fetch("/api/system/ble-enabled", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: next }),
      })
      if (!res.ok) {
        const data = await res.json().catch(() => ({}))
        throw new Error(data.error || "Failed to update BLE toggle")
      }
      setEnabled(next)
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Failed to update")
    } finally {
      setBusy(false)
    }
  }

  const isOn = enabled === true
  const icon = isOn ? (
    <ShieldCheck className="h-3.5 w-3.5" />
  ) : (
    <ShieldAlert className="h-3.5 w-3.5" />
  )

  return (
    <PrefCard
      icon={icon}
      halo={isOn ? "accent" : "amber"}
      title="Tesla BLE"
      badge={
        enabled === null ? null : (
          <Pill kind={isOn ? "accent" : "amber"}>{isOn ? "Enabled" : "Disabled"}</Pill>
        )
      }
    >
      <Toggle
        checked={isOn}
        disabled={busy || enabled === null}
        onChange={handleToggle}
        label="Allow Pi → car BLE commands"
        sub={
          isOn
            ? "Pairing, telemetry, and keep-awake nudges can talk to the car."
            : "Pi cannot send any BLE commands to the car. iOS-app pairing is unaffected."
        }
      />
      {err && <p className="text-xs text-red-400">{err}</p>}
    </PrefCard>
  )
}
