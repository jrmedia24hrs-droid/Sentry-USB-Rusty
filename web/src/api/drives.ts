import type { DriveDetail, DriveSummary, RouteOverview } from "@/types/drives"

export async function fetchDrives(): Promise<DriveSummary[]> {
  const res = await fetch("/api/drives")
  if (!res.ok) throw new Error(`drives: ${res.status}`)
  return res.json()
}

export async function fetchDriveDetail(id: string | number): Promise<DriveDetail> {
  const res = await fetch(`/api/drives/${id}`)
  if (!res.ok) throw new Error(`drive ${id}: ${res.status}`)
  return res.json()
}

export async function fetchRouteOverviews(maxPoints = 20): Promise<RouteOverview[]> {
  const res = await fetch(`/api/drives/routes?max_points=${maxPoints}`)
  if (!res.ok) throw new Error(`routes: ${res.status}`)
  return res.json()
}

export async function setDriveTags(id: string | number, tags: string[]): Promise<void> {
  const res = await fetch(`/api/drives/${id}/tags`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ tags }),
  })
  if (!res.ok) throw new Error(`set tags ${id}: ${res.status}`)
}

export async function fetchTags(): Promise<string[]> {
  const res = await fetch("/api/drives/tags")
  if (!res.ok) throw new Error(`tags: ${res.status}`)
  return res.json()
}

export async function triggerProcessNew(): Promise<void> {
  const res = await fetch("/api/drives/process", { method: "POST" })
  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body.error || `process: ${res.status}`)
  }
}

export async function triggerReprocessAll(): Promise<void> {
  const res = await fetch("/api/drives/reprocess", { method: "POST" })
  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body.error || `reprocess: ${res.status}`)
  }
}

export async function uploadDriveData(file: File): Promise<{ imported: number }> {
  const res = await fetch("/api/drives/data/upload", {
    method: "POST",
    body: file,
  })
  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body.error || `upload: ${res.status}`)
  }
  return res.json()
}

export async function deleteAllDrives(): Promise<void> {
  const res = await fetch("/api/drives/data", { method: "DELETE" })
  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body.error || `delete: ${res.status}`)
  }
}

export async function fetchProcessingStatus(): Promise<{ running: boolean; importing: boolean }> {
  const res = await fetch("/api/drives/status")
  if (!res.ok) throw new Error(`status: ${res.status}`)
  return res.json()
}
