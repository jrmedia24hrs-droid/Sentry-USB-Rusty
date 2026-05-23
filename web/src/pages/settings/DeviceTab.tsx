import { PrefGrid } from "@/components/settings/PrefCard"
import { KeepAwakePreference } from "@/components/settings/sections/KeepAwakePreference"
import { AwayModeControl } from "@/components/settings/sections/AwayModeControl"
import { BleEnableToggle } from "@/components/settings/sections/BleEnableToggle"
import { BlePairButton } from "@/components/settings/sections/BlePairButton"
import { DisplayUnitsSection } from "@/components/settings/sections/DisplayUnitsSection"

export function DeviceTab() {
  return (
    <PrefGrid>
      <KeepAwakePreference />
      <AwayModeControl />
      <BleEnableToggle />
      <BlePairButton />
      <DisplayUnitsSection />
    </PrefGrid>
  )
}
