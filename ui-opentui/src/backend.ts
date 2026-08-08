import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process"
import { dirname, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { createInterface } from "node:readline"

export type User = {
  id: number; username: string; display_name: string; bio: string; utc_offset: number;
  created_at: string; public_key?: string | null
}

export type Post = {
  id: number; user_id: number; username: string; content: string;
  image_path?: string | null; created_at: string
}

export type Message = {
  id: number; sender_id: number; receiver_id: number; sender_username: string;
  content: string; created_at: string; read: boolean; encrypted: boolean
}

export type Notification = {
  id: number; from_user_id: number; from_username: string; notif_type: string;
  created_at: string; read: boolean; related_id?: number | null
}

export type MessagePreview = {
  sender_id: number; sender_username: string; content: string; created_at: string; unread: boolean
}

export type Comment = {
  id: number; post_id: number; user_id: number; username: string; content: string;
  created_at: string; parent_comment_id?: number | null
}

export type UploadFile = { name: string; path: string; size?: string }
export type RadioItem = { tag: string; count: number; post: Post }

type RpcResponse = { id?: number; ok: boolean; data?: unknown; error?: string }

export class AgoraBackend {
  private child: ChildProcessWithoutNullStreams
  private nextId = 1
  private pending = new Map<number, { resolve: (value: unknown) => void; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> }>()
  private stderr = ""

  constructor() {
    const here = dirname(fileURLToPath(import.meta.url))
    const defaultBinary = resolve(here, "../../target/debug/agora")
    const binary = process.env.AGORA_BACKEND_BIN || defaultBinary
    const args = ["--rpc"]
    if (process.env.DATABASE_URL) args.push("--db", process.env.DATABASE_URL)
    this.child = spawn(binary, args, { stdio: ["pipe", "pipe", "pipe"], env: process.env })

    createInterface({ input: this.child.stdout }).on("line", line => {
      let response: RpcResponse
      try { response = JSON.parse(line) as RpcResponse } catch { return }
      if (response.id === undefined) return
      const request = this.pending.get(response.id)
      if (!request) return
      this.pending.delete(response.id)
      clearTimeout(request.timer)
      response.ok ? request.resolve(response.data) : request.reject(new Error(response.error || "Error desconocido"))
    })
    this.child.stderr.on("data", data => { this.stderr = `${this.stderr}${String(data)}`.slice(-2000) })
    this.child.on("error", error => {
      for (const request of this.pending.values()) { clearTimeout(request.timer); request.reject(error) }
      this.pending.clear()
    })
    this.child.on("exit", () => {
      const detail = this.stderr.trim().split("\n").at(-1)
      const error = new Error(detail ? `El backend se cerró: ${detail}` : "El backend de AGORA se cerró.")
      for (const request of this.pending.values()) { clearTimeout(request.timer); request.reject(error) }
      this.pending.clear()
    })
  }

  request<T>(action: string, payload: Record<string, unknown> = {}): Promise<T> {
    const id = this.nextId++
    return new Promise<T>((resolvePromise, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        const detail = this.stderr.trim().split("\n").at(-1)
        reject(new Error(detail || "AGORA no respondió. Comprueba que PostgreSQL esté activo y que DATABASE_URL sea correcta."))
      }, 12_000)
      this.pending.set(id, { resolve: value => resolvePromise(value as T), reject, timer })
      this.child.stdin.write(`${JSON.stringify({ id, action, ...payload })}\n`)
    })
  }

  close() {
    for (const request of this.pending.values()) clearTimeout(request.timer)
    this.pending.clear()
    this.child.stdin.end()
    this.child.kill("SIGTERM")
  }
}
