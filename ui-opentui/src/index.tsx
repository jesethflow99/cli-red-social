import { createCliRenderer, type CliRenderer } from "@opentui/core"
import { createRoot, useKeyboard, useRenderer, useTerminalDimensions } from "@opentui/react"
import { spawn } from "node:child_process"
import { unlink, writeFile } from "node:fs/promises"
import { useEffect, useMemo, useState } from "react"
import { AgoraBackend, type Comment, type Message, type MessagePreview, type Notification, type Post, type RadioItem, type UploadFile, type User } from "./backend.js"

const C = {
  ink: "#060b14", navy: "#091525", panel: "#0e2032", raised: "#17334a", line: "#294c63",
  text: "#eef7fa", dim: "#849cab", cyan: "#38e8d1", violet: "#bd8cff", magenta: "#ff74bd",
  green: "#8bea9d", amber: "#ffd166", coral: "#ff7d73", blue: "#70b7ff", white: "#ffffff",
}
const SECTION_COLORS = [C.cyan, C.violet, C.blue, C.amber, C.green]
const CARD_COLORS = [C.cyan, C.violet, C.magenta, C.green, C.amber, C.blue]
const sections = [
  ["INICIO", "Actividad cronológica de tu equipo"], ["EXPLORAR", "Busca publicaciones y temas"],
  ["MENSAJES", "Conversaciones privadas"], ["ALERTAS", "Menciones y actividad"],
  ["PERFIL", "Tu identidad en AGORA"],
]
const backend = new AgoraBackend()
const ESCAPE_KEY = String.fromCharCode(27)
const CTRL_C = String.fromCharCode(3)
const CLEAR_SCREEN = `${ESCAPE_KEY}[2J${ESCAPE_KEY}[H`

function relativeTime(value: string) {
  const seconds = Math.max(0, Math.floor((Date.now() - new Date(value).getTime()) / 1000))
  if (seconds < 60) return "ahora"
  if (seconds < 3600) return `${Math.floor(seconds / 60)} min`
  if (seconds < 86400) return `${Math.floor(seconds / 3600)} h`
  return `${Math.floor(seconds / 86400)} d`
}
function initials(value: string) { return value.replace(/^@/, "").slice(0, 2).toUpperCase().padEnd(2, "·") }
function truncate(value: string, max: number) { return value.length > max ? `${value.slice(0, max - 1)}…` : value }
function errorText(error: unknown) { return error instanceof Error ? error.message : String(error) }
function copyRemoteClipboard(value: string) {
  // OSC 52 permite copiar al portapapeles del cliente incluso a través de SSH.
  // Las terminales que lo deshabilitan simplemente ignoran la secuencia.
  process.stdout.write(`\u001b]52;c;${Buffer.from(value).toString("base64")}\u0007`)
}
async function viewImageInTerminal(renderer: CliRenderer, rawPath: string, setNotice: (value: string) => void) {
  // No hay widget de imagen en OpenTUI: se cede la terminal (igual que la TUI
  // nativa) para que chafa dibuje directo, y se recupera el control al volver.
  let localPath = rawPath
  let tempFile: string | null = null
  if (/^https?:\/\//i.test(rawPath)) {
    setNotice("Descargando imagen...")
    try {
      const response = await fetch(rawPath)
      const contentType = response.headers.get("content-type") || ""
      if (!response.ok || !contentType.startsWith("image/")) throw new Error("La URL no devolvió una imagen válida.")
      const buffer = Buffer.from(await response.arrayBuffer())
      const ext = (rawPath.split(/[?#]/)[0].split(".").pop() || "jpg").replace(/[^a-zA-Z0-9]/g, "").slice(0, 5) || "jpg"
      tempFile = `/tmp/agora_opentui_${Date.now()}.${ext}`
      await writeFile(tempFile, buffer)
      localPath = tempFile
    } catch (cause) { setNotice(errorText(cause)); return }
  }
  const download = tempFile ? null : `scp -P ${process.env.AGORA_PUBLIC_SSH_PORT || "2222"} localhost:${rawPath.split("/").filter(Boolean).slice(-2).join("/")} .`
  setNotice("")
  renderer.suspend()
  process.stdout.write(CLEAR_SCREEN)
  await new Promise<void>(resolve => {
    const cols = process.stdout.columns || 80, rows = Math.max(10, (process.stdout.rows || 24) - 4)
    const finish = () => {
      process.stdout.write(`\n${download ? `  D  descargar: ${download}\n` : ""}  ENTER / Q / ESC  volver\n`)
      process.stdin.setRawMode?.(true); process.stdin.resume()
      const onData = (data: Buffer) => {
        const key = data.toString()
        if (download && key.toLowerCase() === "d") { copyRemoteClipboard(download); process.stdout.write("  (comando copiado)\n") }
        else if (key === "\r" || key === "\n" || key.toLowerCase() === "q" || key === ESCAPE_KEY || key === CTRL_C) {
          process.stdin.off("data", onData); process.stdin.pause(); resolve()
        }
      }
      process.stdin.on("data", onData)
    }
    const chafa = spawn("chafa", ["--size", `${cols}x${rows}`, localPath], { stdio: ["ignore", "inherit", "inherit"] })
    chafa.on("error", () => { process.stdout.write("\n  chafa no está disponible en este servidor.\n"); finish() })
    chafa.on("close", finish)
  })
  renderer.resume()
  if (tempFile) { try { await unlink(tempFile) } catch {} }
}
function RichContent({ value, color }: { value: string; color: string }) {
  const parts = value.split(/(@[\p{L}\p{N}_-]+|#[\p{L}\p{N}_-]+)/gu)
  return <text fg={C.text} selectable>{parts.map((part, index) => part.startsWith("@")
    ? <span key={index} fg={C.cyan}><strong>{part}</strong></span>
    : part.startsWith("#") ? <span key={index} fg={color}>{part}</span> : part)}</text>
}

function AuthScreen({ onAuthenticated }: { onAuthenticated: (user: User) => void }) {
  const renderer = useRenderer()
  const [mode, setMode] = useState<"login" | "register">("login")
  const [focused, setFocused] = useState(0)
  const [username, setUsername] = useState("")
  const [password, setPassword] = useState("")
  const [displayName, setDisplayName] = useState("")
  const [inviteCode, setInviteCode] = useState("")
  const [busy, setBusy] = useState(false), [error, setError] = useState("")
  const fieldCount = mode === "login" ? 2 : 4
  const changeMode = () => {
    setMode(value => value === "login" ? "register" : "login")
    setFocused(0); setError("")
  }
  useKeyboard(key => {
    if (key.name === "tab") setFocused(value => (value + (key.shift ? fieldCount - 1 : 1)) % fieldCount)
    else if (key.ctrl && key.name === "r") changeMode()
    else if (key.name === "escape" || (key.ctrl && key.name === "q")) { backend.close(); renderer.destroy() }
  })
  const submit = async () => {
    if (busy) return
    const cleanUsername = username.trim(), cleanName = displayName.trim()
    if (!cleanUsername || !password || (mode === "register" && !cleanName)) {
      setError(mode === "login" ? "Escribe usuario y contraseña." : "Usuario, contraseña y nombre son obligatorios."); return
    }
    setBusy(true); setError("")
    try {
      const user = mode === "login"
        ? await backend.request<User>("login", { username: cleanUsername, password })
        : await backend.request<User>("register", { username: cleanUsername, password, display_name: cleanName, invite_code: inviteCode.trim() || null })
      onAuthenticated(user)
    } catch (cause) { setError(errorText(cause)) } finally { setBusy(false) }
  }
  const advance = (index: number) => { if (index === fieldCount - 1) void submit(); else setFocused(index + 1) }
  const inputBox = (index: number, title: string, value: string, placeholder: string, onInput: (value: string) => void) =>
    <box title={` ${title} `} titleColor={focused === index ? (mode === "login" ? C.green : C.amber) : C.dim} onMouseDown={() => setFocused(index)} style={{ height: 3, border: true, borderStyle: "rounded", borderColor: focused === index ? (mode === "login" ? C.green : C.amber) : C.line }}>
      <input value={value} focused={focused === index} placeholder={placeholder} onInput={onInput} onSubmit={() => advance(index)} />
    </box>
  return <box style={{ width: "100%", height: "100%", backgroundColor: C.ink, justifyContent: "center", alignItems: "center" }}>
    <box title=" AGORA / ACCESO " titleColor={C.cyan} style={{ width: "70%", maxWidth: 72, minHeight: mode === "login" ? 21 : 29, border: true, borderStyle: "double", borderColor: C.violet, backgroundColor: C.navy, padding: 2, flexDirection: "column", gap: 1 }}>
      <text fg={C.cyan}><strong>◈ AGORA</strong> <span fg={C.violet}>COMUNICACIÓN PRIVADA PARA EQUIPOS</span></text>
      <text fg={C.dim}>SSH protege el acceso. Tu identidad se autentica aquí, dentro de la TUI.</text>
      <text fg={mode === "login" ? C.green : C.amber}><strong>{mode === "login" ? "INICIAR SESIÓN" : "CREAR CUENTA"}</strong></text>
      {inputBox(0, "USUARIO", username, "tu_usuario", setUsername)}
      {inputBox(1, "CONTRASEÑA", password, "escribe tu contraseña", setPassword)}
      {mode === "register" ? <>{inputBox(2, "NOMBRE VISIBLE", displayName, "Cómo quieres aparecer", setDisplayName)}{inputBox(3, "INVITACIÓN (OPCIONAL)", inviteCode, "vacío si el registro está abierto", setInviteCode)}</> : null}
      <text fg={error ? C.coral : C.dim}>{busy ? "Conectando con AGORA..." : error || "ENTER avanzar/confirmar · TAB cambiar campo · Ctrl+R cambiar modo · ESC salir"}</text>
    </box>
  </box>
}

function Sidebar({ section, user, onSection, accent, badges }: { section: number; user: User; onSection: (value: number) => void; accent: string; badges: Record<number, number> }) {
  return <box style={{ width: 25, flexDirection: "column", border: ["right"], borderColor: C.line, paddingRight: 1 }}>
    <box style={{ height: 6, flexDirection: "column", paddingLeft: 1 }}><text fg={C.cyan}><strong>◈ AGORA</strong></text><text fg={C.violet}>ESPACIO DEL EQUIPO</text><text fg={C.green}>● EN LÍNEA</text></box>
    <text fg={C.dim}>  NAVEGACIÓN</text>
    <box style={{ flexDirection: "column", gap: 1, marginTop: 1 }}>
      {sections.map(([name], index) => <box key={name} onMouseDown={() => onSection(index)} style={{ height: 2, paddingLeft: 1, flexDirection: "row", alignItems: "center", backgroundColor: section === index ? C.raised : C.ink, border: section === index ? ["left"] : false, borderColor: SECTION_COLORS[index] }}>
        <text fg={section === index ? C.text : C.dim}><span fg={SECTION_COLORS[index]}><strong>{index + 1}</strong></span>  <strong>{name}</strong></text>
        <box style={{ flexGrow: 1 }} />
        {badges[index] ? <box style={{ backgroundColor: C.coral }}><text fg={C.ink}><strong>{badges[index]}</strong></text></box> : null}
      </box>)}
    </box>
    <box style={{ flexGrow: 1 }} />
    <box title=" SESIÓN " titleColor={accent} style={{ height: 7, border: true, borderStyle: "rounded", borderColor: C.line, padding: 1, flexDirection: "column" }}>
      <text fg={C.white}><strong>{user.display_name}</strong> <span fg={C.green}>●</span></text><text fg={C.dim}>@{user.username}</text><text fg={C.blue}>sesión privada</text>
    </box>
  </box>
}

function PostCard({ post, active, color, onPick }: { post: Post; active: boolean; color: string; onPick: () => void }) {
  const tags = post.content.match(/#[\p{L}\p{N}_-]+/gu)?.join("  ") || ""
  return <box onMouseDown={onPick} style={{ width: "100%", flexDirection: "column", padding: 1, marginBottom: 1, backgroundColor: active ? C.raised : C.panel, border: active ? ["left"] : false, borderColor: color }}>
    <box style={{ height: 2, flexDirection: "row" }}><box style={{ width: 5, height: 1, justifyContent: "center", backgroundColor: color }}><text fg={C.ink}><strong>{initials(post.username)}</strong></text></box><text fg={C.text}> <strong>@{post.username}</strong> <span fg={C.dim}>· {relativeTime(post.created_at)}</span></text><box style={{ flexGrow: 1 }} /><text fg={C.dim}>ID {post.id}</text></box>
    <RichContent value={post.content} color={color} />{tags ? <text fg={color}>{tags}</text> : null}
    {post.image_path ? <text fg={C.amber}>▧ imagen adjunta · [I] ver</text> : null}
    <text fg={active ? color : C.dim}>{active ? "ENTER abrir publicación · P perfil" : ""}</text>
  </box>
}

function ListPanel({ title, lines, selected, accent, empty }: { title: string; lines: Array<{ primary: string; secondary: string }>; selected: number; accent: string; empty: string }) {
  return <box style={{ flexGrow: 1, flexDirection: "column", paddingTop: 1 }}>
    <text fg={accent}><strong>{title}</strong></text>
    {lines.length === 0 ? <box style={{ flexGrow: 1, justifyContent: "center", alignItems: "center" }}><text fg={C.dim}>{empty}</text></box> : lines.map((line, index) => <box key={`${line.primary}-${index}`} style={{ height: 3, paddingLeft: 1, flexDirection: "column", backgroundColor: index === selected ? C.raised : index % 2 === 0 ? C.panel : C.navy, border: index === selected ? ["left"] : false, borderColor: accent }}><text fg={index === selected ? C.white : C.text}><strong>{line.primary}</strong></text><text fg={C.dim}>{line.secondary}</text></box>)}
  </box>
}

type ModalMode = "compose" | "image_url" | "search" | "message" | "comment" | "reply" | "help"
function Modal({ mode, accent, close, submit, attachment, initialValue = "", onChange }: { mode: ModalMode; accent: string; close: () => void; submit: (value: string) => void; attachment?: string; initialValue?: string; onChange?: (value: string) => void }) {
  const [value, setValue] = useState(initialValue)
  const title = { compose: "NUEVA PUBLICACIÓN", image_url: "IMAGEN POR URL", search: "BUSCAR", message: "NUEVO MENSAJE", comment: "COMENTAR", reply: "RESPONDER", help: "AYUDA" }[mode]
  return <box title={` ${title} `} titleColor={accent} style={{ position: "absolute", left: "15%", top: "22%", width: "70%", height: mode === "help" ? 16 : 11, zIndex: 20, border: true, borderStyle: "double", borderColor: accent, backgroundColor: C.raised, padding: 1, flexDirection: "column", gap: 1 }}>
    {mode === "help" ? <><text fg={C.cyan}><strong>ENTER</strong> abrir publicación/perfil/chat</text><text fg={C.violet}><strong>P</strong> perfil del autor · <strong>M</strong> mensaje</text><text fg={C.green}><strong>N</strong> abrir compositor de publicación</text><text fg={C.blue}><strong>C</strong> comentar · <strong>R</strong> responder/Radio</text><text fg={C.amber}><strong>/</strong> búsqueda combinada</text><text fg={C.dim}>J/K mover · ESC cerrar · Q salir</text></> : <><text fg={C.dim}>{mode === "compose" ? `Texto de la publicación.${attachment ? ` Adjunta: ${attachment}` : " Sin imagen."}` : mode === "image_url" ? "Introduce una URL http(s) de imagen. ESC vuelve al compositor." : mode === "message" ? "Escribe un mensaje privado." : mode === "comment" ? "Comenta esta publicación; @usuario genera una mención." : mode === "reply" ? "Responde al comentario seleccionado." : "Busca perfiles, @usuarios, contenido y hashtags."}</text><box style={{ height: 3, border: true, borderStyle: "rounded", borderColor: accent }}><input focused placeholder={mode === "search" ? "nombre, @usuario o texto..." : mode === "image_url" ? "https://servidor/imagen.jpg" : "escribe aquí..."} onInput={next => { setValue(next); onChange?.(next) }} onSubmit={() => submit(value)} /></box><text fg={C.dim}>{mode === "compose" ? "ENTER publicar · Ctrl+P subir por SCP · Ctrl+U usar URL" : "ENTER confirmar · ESC cancelar"} · {value.length}</text></>}
  </box>
}

function PostOverlay({ post, comments, selected }: { post: Post; comments: Comment[]; selected: number }) {
  return <box title=" PUBLICACIÓN / HILO " titleColor={C.cyan} style={{ position: "absolute", left: "8%", top: "8%", width: "84%", height: "84%", zIndex: 15, border: true, borderStyle: "double", borderColor: C.cyan, backgroundColor: C.ink, padding: 2, flexDirection: "column" }}>
    <text fg={C.white}><strong>@{post.username}</strong> <span fg={C.dim}>· {relativeTime(post.created_at)}</span></text><RichContent value={post.content} color={C.magenta} />
    {post.image_path ? <text fg={C.amber}>▧ {post.image_path} · [I] ver</text> : null}<text fg={C.violet}><strong>COMENTARIOS · {comments.length}</strong></text>
    <scrollbox focused style={{ flexGrow: 1 }}>{comments.map((item, index) => <box key={item.id} style={{ minHeight: 3, paddingLeft: item.parent_comment_id ? 4 : 1, flexDirection: "column", backgroundColor: index === selected ? C.raised : C.panel, border: index === selected ? ["left"] : false, borderColor: item.parent_comment_id ? C.violet : C.blue }}><text fg={C.cyan}><strong>@{item.username}</strong>{item.parent_comment_id ? <span fg={C.dim}> ↳ respuesta</span> : null}</text><RichContent value={item.content} color={C.violet} /></box>)}</scrollbox>
    <text fg={C.dim}>C comentar · R responder · {post.image_path ? "I ver imagen · " : ""}P autor · J/K mover · ESC volver</text>
  </box>
}

function RadioOverlay({ items, index, paused }: { items: RadioItem[]; index: number; paused: boolean }) {
  const current = items[index]
  return <box title=" ◉ AGORA RADIO " titleColor={C.magenta} style={{ position: "absolute", left: "10%", top: "12%", width: "80%", height: "72%", zIndex: 16, border: true, borderStyle: "double", borderColor: C.magenta, backgroundColor: C.navy, padding: 2, flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 1 }}>
    {current ? <><text fg={C.amber}><strong>#{current.tag}</strong> · {current.count} señales</text><text fg={C.cyan}><strong>@{current.post.username}</strong></text><RichContent value={current.post.content} color={C.magenta} /><text fg={paused ? C.amber : C.green}>{paused ? "Ⅱ PAUSADO" : "● EN VIVO"}</text></> : <text fg={C.dim}>Aún no hay hashtags para transmitir.</text>}
    <text fg={C.dim}>←/→ cambiar · ESPACIO pausar · ENTER abrir · ESC salir</text>
  </box>
}

function UploadOverlay({ command, files, selected }: { command: string; files: UploadFile[]; selected: number }) {
  return <box title=" SUBIR IMAGEN / SCP " titleColor={C.amber} style={{ position: "absolute", left: "10%", top: "15%", width: "80%", height: "65%", zIndex: 17, border: true, borderStyle: "double", borderColor: C.amber, backgroundColor: C.navy, padding: 2, flexDirection: "column", gap: 1 }}>
    <text fg={C.text}>Comando copiado al portapapeles (token de un uso, válido 5 minutos):</text><text fg={C.cyan} selectable><strong>{command}</strong></text><text fg={C.green}>◌ Esperando archivo… aparecerá y se adjuntará automáticamente.</text>
    {files.map((file, index) => <box key={file.path} style={{ height: 2, paddingLeft: 1, backgroundColor: index === selected ? C.raised : C.panel, border: index === selected ? ["left"] : false, borderColor: C.amber }}><text fg={index === selected ? C.white : C.dim}>▧ {file.name}</text></box>)}
    <text fg={C.dim}>Y copiar otra vez · J/K elegir · ENTER adjuntar · U actualizar · ESC cancelar</text>
  </box>
}

function Dashboard({ initialUser }: { initialUser: User }) {
  const renderer = useRenderer(), { width, height } = useTerminalDimensions()
  const [user, setUser] = useState(initialUser), [section, setSection] = useState(0), [selected, setSelected] = useState(0)
  const [feed, setFeed] = useState<Post[]>([]), [searchResults, setSearchResults] = useState<Post[]>([]), [searchUsers, setSearchUsers] = useState<User[]>([])
  const [conversations, setConversations] = useState<User[]>([]), [notifications, setNotifications] = useState<Notification[]>([])
  const [profilePosts, setProfilePosts] = useState<Post[]>([]), [profileUser, setProfileUser] = useState<User>(initialUser), [messages, setMessages] = useState<Message[]>([]), [chatUser, setChatUser] = useState<User | null>(null)
  const [unreadMessages, setUnreadMessages] = useState(0), [messagePreviews, setMessagePreviews] = useState<MessagePreview[]>([])
  const [unreadNotifications, setUnreadNotifications] = useState(0), [notificationPreviews, setNotificationPreviews] = useState<Notification[]>([])
  const [modal, setModal] = useState<ModalMode | null>(null)
  const [detail, setDetail] = useState<Post | null>(null), [comments, setComments] = useState<Comment[]>([]), [commentSelected, setCommentSelected] = useState(0)
  const [radioItems, setRadioItems] = useState<RadioItem[]>([]), [radioIndex, setRadioIndex] = useState(0), [radioPaused, setRadioPaused] = useState(false), [radioOpen, setRadioOpen] = useState(false)
  const [uploadOpen, setUploadOpen] = useState(false), [uploadCommand, setUploadCommand] = useState(""), [uploadFiles, setUploadFiles] = useState<UploadFile[]>([]), [uploadSelected, setUploadSelected] = useState(0), [attachedImage, setAttachedImage] = useState<UploadFile | null>(null)
  const [composeDraft, setComposeDraft] = useState(""), [returnToCompose, setReturnToCompose] = useState(false)
  const [notice, setNotice] = useState("Cargando actividad..."), [busy, setBusy] = useState(false)
  const accent = SECTION_COLORS[section], compact = width < 112, narrow = width < 72
  const activePosts = section === 1 ? searchResults : feed

  const loadSection = async (next: number) => {
    setSection(next); setSelected(0); setBusy(true); setNotice(`Actualizando ${sections[next][0].toLowerCase()}...`)
    try {
      if (next === 0) setFeed(await backend.request<Post[]>("timeline"))
      else if (next === 2) setConversations(await backend.request<User[]>("conversations"))
      else if (next === 3) { setNotifications(await backend.request<Notification[]>("notifications")); setUnreadNotifications(0) }
      else if (next === 4) { const data = await backend.request<{ user: User; posts: Post[] }>("profile"); setUser(data.user); setProfileUser(data.user); setProfilePosts(data.posts) }
      setNotice(sections[next][1])
    } catch (cause) { setNotice(errorText(cause)) } finally { setBusy(false) }
  }
  useEffect(() => { void loadSection(0); return () => backend.close() }, [])
  useEffect(() => {
    let running = false
    const sync = async () => {
      if (running) return
      running = true
      try {
        if (section === 2) {
          if (chatUser) setMessages(await backend.request<Message[]>("messages", { other_id: chatUser.id }))
          else setConversations(await backend.request<User[]>("conversations"))
        }
        const [unread, previews, unreadNotifs, recentNotifs] = await Promise.all([
          backend.request<{ count: number }>("unread_messages"),
          backend.request<MessagePreview[]>("message_previews"),
          backend.request<{ count: number }>("unread_notifications"),
          backend.request<Notification[]>("notifications_peek"),
        ])
        setUnreadMessages(unread.count); setMessagePreviews(previews)
        setUnreadNotifications(unreadNotifs.count); setNotificationPreviews(recentNotifs)
      } catch (cause) { setNotice(errorText(cause)) } finally { running = false }
    }
    void sync()
    const timer = setInterval(() => { void sync() }, section === 2 ? 1500 : 5000)
    return () => clearInterval(timer)
  }, [section, chatUser?.id])
  useEffect(() => { if (!radioOpen || radioPaused || radioItems.length < 2) return; const timer = setInterval(() => setRadioIndex(value => (value + 1) % radioItems.length), 5000); return () => clearInterval(timer) }, [radioOpen, radioPaused, radioItems.length])
  useEffect(() => {
    if (!uploadOpen) return
    const known = new Set(uploadFiles.map(file => file.path))
    const timer = setInterval(() => { void backend.request<{ files: UploadFile[] }>("uploads").then(data => {
      const received = data.files.find(file => !known.has(file.path))
      if (received) { setUploadFiles(data.files); setAttachedImage(received); setUploadOpen(false); if (returnToCompose) setModal("compose"); setNotice(`✓ ${received.name} recibida y adjunta`) }
    }).catch(cause => setNotice(errorText(cause))) }, 1500)
    return () => clearInterval(timer)
  }, [uploadOpen, uploadFiles, returnToCompose])
  useKeyboard(key => {
    if (modal) {
      if (modal === "compose" && key.ctrl && key.name === "p") { setModal(null); setReturnToCompose(true); void refreshUploads(true) }
      else if (modal === "compose" && key.ctrl && key.name === "u") setModal("image_url")
      else if (key.name === "escape") { if (modal === "image_url") setModal("compose"); else setModal(null) }
      return
    }
    if (uploadOpen) {
      if (key.name === "escape") { setUploadOpen(false); if (returnToCompose) setModal("compose") }
      else if (key.name === "j" || key.name === "down") setUploadSelected(value => Math.min(uploadFiles.length - 1, value + 1))
      else if (key.name === "k" || key.name === "up") setUploadSelected(value => Math.max(0, value - 1))
      else if (key.name === "u") void refreshUploads(false)
      else if (key.name === "y") { copyRemoteClipboard(uploadCommand); setNotice("Comando SCP copiado") }
      else if (key.name === "return" && uploadFiles[uploadSelected]) { setAttachedImage(uploadFiles[uploadSelected]); setUploadOpen(false); if (returnToCompose) setModal("compose"); setNotice(`Imagen adjunta: ${uploadFiles[uploadSelected].name}`) }
      return
    }
    if (radioOpen) {
      if (key.name === "escape") setRadioOpen(false)
      else if (key.name === "space") setRadioPaused(value => !value)
      else if (key.name === "right" || key.name === "l") setRadioIndex(value => radioItems.length ? (value + 1) % radioItems.length : 0)
      else if (key.name === "left" || key.name === "h") setRadioIndex(value => radioItems.length ? (value - 1 + radioItems.length) % radioItems.length : 0)
      else if (key.name === "return" && radioItems[radioIndex]) { setRadioOpen(false); void openPost(radioItems[radioIndex].post.id) }
      return
    }
    if (detail) {
      if (key.name === "escape") setDetail(null)
      else if (key.name === "j" || key.name === "down") setCommentSelected(value => Math.min(comments.length - 1, value + 1))
      else if (key.name === "k" || key.name === "up") setCommentSelected(value => Math.max(0, value - 1))
      else if (key.name === "c") setModal("comment")
      else if (key.name === "r" && comments[commentSelected]) setModal("reply")
      else if (key.name === "p") { const id = detail.user_id; setDetail(null); void openProfile(id) }
      else if (key.name === "i" && detail.image_path) void viewImageInTerminal(renderer, detail.image_path, setNotice)
      return
    }
    if (key.name === "q" || (key.ctrl && key.name === "c")) { backend.close(); renderer.destroy() }
    else if (key.name === "escape" && section === 2 && chatUser) { setChatUser(null); setMessages([]); void loadSection(2) }
    else if (/^[1-5]$/.test(key.name)) void loadSection(Number(key.name) - 1)
    else if (key.name === "j" || key.name === "down") setSelected(index => Math.min(Math.max(0, currentLength() - 1), index + 1))
    else if (key.name === "k" || key.name === "up") setSelected(index => Math.max(0, index - 1))
    else if (key.name === "n" && section === 0) { setComposeDraft(""); setReturnToCompose(false); setModal("compose") }
    else if (key.name === "r") void openRadio()
    else if (key.name === "/") setModal("search")
    else if (key.name === "?") setModal("help")
    else if (key.name === "return" && section === 0 && feed[selected]) void openPost(feed[selected].id)
    else if (key.name === "p" && section === 0 && feed[selected]) void openProfile(feed[selected].user_id)
    else if (key.name === "i" && selectedPost()?.image_path) void viewImageInTerminal(renderer, selectedPost()!.image_path!, setNotice)
    else if (key.name === "return" && section === 1) {
      const target = selected < searchUsers.length ? searchUsers[selected] : searchResults[selected - searchUsers.length]
      if (target) "user_id" in target ? void openPost(target.id) : void openProfile(target.id)
    }
    else if (key.name === "return" && section === 2 && conversations[selected]) void openChat(conversations[selected])
    else if (key.name === "return" && section === 3 && notifications[selected]) { const item = notifications[selected]; item.related_id ? void openPost(item.related_id) : void openProfile(item.from_user_id) }
    else if (key.name === "return" && section === 4 && profilePosts[selected]) void openPost(profilePosts[selected].id)
    else if (key.name === "m" && section === 4 && profileUser.id !== user.id) void startChat(profileUser)
    else if (key.name === "m" && chatUser) setModal("message")
  })
  const currentLength = () => section === 0 ? feed.length : section === 1 ? searchUsers.length + searchResults.length : section === 2 ? conversations.length : section === 3 ? notifications.length : profilePosts.length
  const selectedPost = (): Post | undefined => section === 0 ? feed[selected] : section === 4 ? profilePosts[selected] : section === 1 && selected >= searchUsers.length ? searchResults[selected - searchUsers.length] : undefined
  const openProfile = async (userId: number) => { setBusy(true); try { const data = await backend.request<{ user: User; posts: Post[] }>("profile", { user_id: userId }); setProfileUser(data.user); setProfilePosts(data.posts); setSection(4); setSelected(0); setNotice(`Perfil de @${data.user.username}`) } catch (cause) { setNotice(errorText(cause)) } finally { setBusy(false) } }
  const openPost = async (postId: number) => { setBusy(true); try { const data = await backend.request<{ post: Post; comments: Comment[] }>("post_detail", { post_id: postId }); setDetail(data.post); setComments(data.comments); setCommentSelected(0); setNotice(`Hilo #${postId}`) } catch (cause) { setNotice(errorText(cause)) } finally { setBusy(false) } }
  const openChat = async (other: User) => { setBusy(true); try { setMessages(await backend.request<Message[]>("messages", { other_id: other.id })); setChatUser(other); setNotice(`Conversación con @${other.username}`) } catch (cause) { setNotice(errorText(cause)) } finally { setBusy(false) } }
  const startChat = async (other: User) => { setSection(2); setSelected(0); await openChat(other); setModal("message") }
  const refreshUploads = async (prepare: boolean) => { setBusy(true); try { const data = await backend.request<{ command?: string; files: UploadFile[] }>(prepare ? "upload_prepare" : "uploads"); const command = data.command || uploadCommand; setUploadCommand(command); setUploadFiles(data.files); setUploadSelected(0); setUploadOpen(true); if (prepare && command) copyRemoteClipboard(command); setNotice(prepare ? "Comando SCP copiado · esperando imagen" : "Galería actualizada") } catch (cause) { setNotice(errorText(cause)) } finally { setBusy(false) } }
  const openRadio = async () => { setBusy(true); try { const data = await backend.request<{ items: RadioItem[] }>("radio"); setRadioItems(data.items); setRadioIndex(0); setRadioPaused(false); setRadioOpen(true); setNotice("AGORA Radio") } catch (cause) { setNotice(errorText(cause)) } finally { setBusy(false) } }
  const submit = async (value: string) => {
    setModal(null); if (!value.trim()) return; setBusy(true)
    try {
      if (modal === "image_url") { if (!/^https?:\/\//i.test(value) || value.length > 2048) throw new Error("Usa una URL http(s) válida."); setAttachedImage({ name: value, path: value }); setModal("compose"); setNotice("Imagen por URL adjunta") }
      else if (modal === "comment" && detail) { await backend.request<Comment>("add_comment", { post_id: detail.id, content: value }); await openPost(detail.id); setNotice("Comentario publicado") }
      else if (modal === "reply" && detail && comments[commentSelected]) { await backend.request<Comment>("add_comment", { post_id: detail.id, content: value, parent_id: comments[commentSelected].id }); await openPost(detail.id); setNotice("Respuesta publicada") }
      else if (modal === "compose") { await backend.request<Post>("create_post", { content: value, image_path: attachedImage?.path }); setAttachedImage(null); setComposeDraft(""); setFeed(await backend.request<Post[]>("timeline")); setSection(0); setNotice("Publicación enviada") }
      else if (modal === "message" && chatUser) { await backend.request<Message>("send_message", { other_id: chatUser.id, content: value }); await openChat(chatUser); setNotice("Mensaje enviado") }
      else { const data = await backend.request<{ users: User[]; posts: Post[] }>("search_all", { query: value }); setSearchUsers(data.users); setSearchResults(data.posts); setSection(1); setSelected(0); setNotice(`${data.users.length} perfiles · ${data.posts.length} publicaciones`) }
    } catch (cause) { setNotice(errorText(cause)) } finally { setBusy(false) }
  }
  const conversationLines = chatUser ? messages.map(item => ({ primary: item.sender_id === user.id ? "Tú" : `@${item.sender_username}`, secondary: `${item.content} · ${relativeTime(item.created_at)}` })) : conversations.map(item => ({ primary: `${item.display_name}  @${item.username}`, secondary: item.bio || "Abrir conversación con ENTER" }))
  const notificationLines = notifications.map(item => ({ primary: `@${item.from_username}`, secondary: `${item.notif_type} · ${relativeTime(item.created_at)}` }))

  return <box style={{ width: "100%", height: "100%", backgroundColor: C.ink, flexDirection: "column" }}>
    <box style={{ height: 2, backgroundColor: accent, paddingLeft: 1, paddingRight: 1, flexDirection: "row" }}><text fg={C.ink}><strong> AGORA / {sections[section][0]} </strong></text><text fg={C.ink}> {width}×{height}</text><box style={{ flexGrow: 1 }} /><text fg={C.ink}><strong>{busy ? "SINCRONIZANDO..." : notice}  ●</strong></text></box>
    <box style={{ flexGrow: 1, padding: 1, flexDirection: "row" }}>
      {!narrow && <Sidebar section={section} user={user} onSection={value => void loadSection(value)} accent={accent} badges={{ 2: unreadMessages, 3: unreadNotifications }} />}
      <box style={{ flexGrow: 1, flexDirection: "column", paddingLeft: narrow ? 0 : 1 }}>
        <box style={{ height: 4, border: ["bottom"], borderColor: C.line, flexDirection: "column" }}><box style={{ flexDirection: "row" }}><text fg={accent}><strong>{sections[section][0]}</strong></text><box style={{ flexGrow: 1 }} /><text fg={C.dim}>cronológico · <span fg={C.green}>● conectado</span></text></box><text fg={C.dim}>{sections[section][1]}</text></box>
        {section === 0 ? <scrollbox focused style={{ flexGrow: 1, rootOptions: { backgroundColor: C.ink }, viewportOptions: { backgroundColor: C.ink }, contentOptions: { backgroundColor: C.ink }, scrollbarOptions: { showArrows: false, trackOptions: { foregroundColor: accent, backgroundColor: C.line } } }}>{feed.length ? feed.map((post, index) => <PostCard key={post.id} post={post} active={index === selected} color={CARD_COLORS[index % CARD_COLORS.length]} onPick={() => setSelected(index)} />) : <text fg={C.dim}>No hay publicaciones en tu timeline.</text>}</scrollbox>
          : section === 1 ? <scrollbox focused style={{ flexGrow: 1, rootOptions: { backgroundColor: C.ink }, viewportOptions: { backgroundColor: C.ink }, contentOptions: { backgroundColor: C.ink } }}><text fg={C.violet}><strong>PERFILES POR COINCIDENCIA</strong></text>{searchUsers.map((item, index) => <box key={item.id} onMouseDown={() => setSelected(index)} style={{ height: 3, paddingLeft: 1, flexDirection: "column", backgroundColor: selected === index ? C.raised : C.navy, border: selected === index ? ["left"] : false, borderColor: C.violet }}><text fg={C.white}><strong>{item.display_name}</strong>  <span fg={C.cyan}>@{item.username}</span></text><text fg={C.dim}>{item.bio || "ENTER para abrir el perfil"}</text></box>)}<text fg={C.magenta}><strong>PUBLICACIONES COINCIDENTES</strong></text>{searchResults.map((post, index) => <PostCard key={post.id} post={post} active={selected === searchUsers.length + index} color={CARD_COLORS[index % CARD_COLORS.length]} onPick={() => setSelected(searchUsers.length + index)} />)}{!searchUsers.length && !searchResults.length ? <text fg={C.dim}>Pulsa / para buscar perfiles, nombres, @usuarios o contenido.</text> : null}</scrollbox>
          : section === 2 ? <ListPanel title={chatUser ? `CHAT EN VIVO · @${chatUser.username}  [M] responder` : `CONVERSACIONES · ${unreadMessages} sin leer`} lines={conversationLines} selected={selected} accent={accent} empty="Todavía no hay conversaciones." />
          : section === 3 ? <ListPanel title="ACTIVIDAD RECIENTE" lines={notificationLines} selected={selected} accent={accent} empty="No tienes alertas pendientes." />
          : <scrollbox focused style={{ flexGrow: 1, padding: 1, rootOptions: { backgroundColor: C.ink }, viewportOptions: { backgroundColor: C.ink }, contentOptions: { backgroundColor: C.ink }, scrollbarOptions: { showArrows: true, trackOptions: { foregroundColor: C.green, backgroundColor: C.line } } }}><box title=" IDENTIDAD " titleColor={C.green} style={{ border: true, borderStyle: "rounded", borderColor: C.green, padding: 2, flexDirection: "column", height: 10 }}><text fg={C.white}><strong>{profileUser.display_name}</strong></text><text fg={C.cyan}>@{profileUser.username}</text><text fg={C.text}>{profileUser.bio || "Sin biografía todavía."}</text><text fg={C.violet}>{profilePosts.length} publicaciones recientes</text><text fg={C.blue}>{profileUser.id === user.id ? "Tu perfil" : "M enviar mensaje privado"}</text></box><text fg={C.green}><strong>PUBLICACIONES DE @{profileUser.username}</strong></text>{profilePosts.map((post, index) => <PostCard key={post.id} post={post} active={index === selected} color={CARD_COLORS[index % CARD_COLORS.length]} onPick={() => setSelected(index)} />)}{!profilePosts.length ? <text fg={C.dim}>Este perfil todavía no ha publicado.</text> : null}</scrollbox>}
      </box>
      {!compact && <box style={{ width: 27, border: ["left"], borderColor: C.line, paddingLeft: 1, flexDirection: "column", gap: 1 }}>
        <box title=" MENSAJES " titleColor={C.blue} onMouseDown={() => void loadSection(2)} style={{ minHeight: 6, border: true, borderStyle: "rounded", borderColor: C.line, padding: 1, flexDirection: "column" }}>
          {messagePreviews.length ? messagePreviews.map(item => <text key={item.sender_id} fg={item.unread ? C.white : C.dim}>{item.unread ? "● " : "○ "}<span fg={C.cyan}><strong>@{item.sender_username}</strong></span> {truncate(item.content, 20)}</text>) : <text fg={C.dim}>Sin mensajes todavía.</text>}
        </box>
        <box title=" ALERTAS " titleColor={C.amber} onMouseDown={() => void loadSection(3)} style={{ minHeight: 6, border: true, borderStyle: "rounded", borderColor: C.line, padding: 1, flexDirection: "column" }}>
          {notificationPreviews.length ? notificationPreviews.map(item => <text key={item.id} fg={item.read ? C.dim : C.white}>{item.read ? "○ " : "● "}<span fg={C.violet}><strong>@{item.from_username}</strong></span> {item.notif_type === "follow" ? "te siguió" : "te mencionó"} · {relativeTime(item.created_at)}</text>) : <text fg={C.dim}>Sin alertas todavía.</text>}
        </box>
      </box>}
    </box>
    <box style={{ height: 2, border: ["top"], borderColor: C.line, paddingLeft: 1, flexDirection: "row" }}><text fg={C.cyan}><strong>[1–5]</strong></text><text fg={C.dim}> SECCIONES  </text><text fg={C.green}><strong>[N]</strong></text><text fg={C.dim}> PUBLICAR  </text><text fg={C.magenta}><strong>[R]</strong></text><text fg={C.dim}> RADIO  </text><text fg={C.violet}><strong>[/]</strong></text><text fg={C.dim}> BUSCAR</text><box style={{ flexGrow: 1 }} /><text fg={C.dim}>[?] AYUDA [Q] SALIR </text></box>
    {detail && <PostOverlay post={detail} comments={comments} selected={commentSelected} />}
    {radioOpen && <RadioOverlay items={radioItems} index={radioIndex} paused={radioPaused} />}
    {uploadOpen && <UploadOverlay command={uploadCommand} files={uploadFiles} selected={uploadSelected} />}
    {modal && <Modal key={modal} mode={modal} accent={accent} close={() => setModal(null)} submit={submit} attachment={attachedImage?.name} initialValue={modal === "compose" ? composeDraft : ""} onChange={modal === "compose" ? setComposeDraft : undefined} />}
  </box>
}

function App() {
  const [user, setUser] = useState<User | null>(null)
  return user ? <Dashboard initialUser={user} /> : <AuthScreen onAuthenticated={setUser} />
}

const renderer = await createCliRenderer({ exitOnCtrlC: false, backgroundColor: C.ink })
createRoot(renderer).render(<App />)
