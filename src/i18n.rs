use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum Lang {
    Es,
    En,
}

impl Lang {
    pub fn from_env() -> Self {
        match std::env::var("LANG").unwrap_or_default().to_lowercase() {
            s if s.starts_with("es") => Lang::Es,
            _ => Lang::Es,
        }
    }
}

#[macro_export]
macro_rules! t {
    ($self:expr, $field:ident) => {
        match $self.lang {
            $crate::i18n::Lang::Es => $crate::i18n::S.es.$field,
            $crate::i18n::Lang::En => $crate::i18n::S.en.$field,
        }
    };
}

pub fn ago(lang: Lang, dt: &DateTime<Utc>, offset_min: i32) -> String {
    let now = Utc::now() + chrono::Duration::minutes(offset_min as i64);
    let target = *dt + chrono::Duration::minutes(offset_min as i64);
    let diff = now.signed_duration_since(target);
    let secs = diff.num_seconds().max(0);
    let (num, unit) = if secs < 60 {
        (0, s_ago(lang, "now"))
    } else if secs < 3600 {
        let m = secs / 60;
        (m, if m == 1 { s_ago(lang, "min") } else { s_ago(lang, "mins") })
    } else if secs < 86400 {
        let h = secs / 3600;
        (h, if h == 1 { s_ago(lang, "hour") } else { s_ago(lang, "hours") })
    } else {
        let d = secs / 86400;
        (d, if d == 1 { s_ago(lang, "day") } else { s_ago(lang, "days") })
    };
    if unit.contains("{}") { format(unit, num) } else { unit.to_string() }
}

fn s_ago(lang: Lang, key: &str) -> &'static str {
    match (lang, key) {
        (Lang::Es, "now") => "ahora",
        (Lang::En, "now") => "now",
        (Lang::Es, "min") => "hace 1 min",
        (Lang::En, "min") => "1 min ago",
        (Lang::Es, "mins") => "hace {} min",
        (Lang::En, "mins") => "{} min ago",
        (Lang::Es, "hour") => "hace 1 h",
        (Lang::En, "hour") => "1 hour ago",
        (Lang::Es, "hours") => "hace {} h",
        (Lang::En, "hours") => "{} hours ago",
        (Lang::Es, "day") => "hace 1 día",
        (Lang::En, "day") => "1 day ago",
        (Lang::Es, "days") => "hace {} días",
        (Lang::En, "days") => "{} days ago",
        _ => "",
    }
}

fn format(template: &str, n: i64) -> String {
    template.replace("{}", &n.to_string())
}

pub struct Strings {
    pub es: &'static LangStrings,
    pub en: &'static LangStrings,
}

impl Strings {
    pub const fn new(es: &'static LangStrings, en: &'static LangStrings) -> Self {
        Self { es, en }
    }
}

#[allow(dead_code)]
pub struct LangStrings {
    pub login_title: &'static str,
    pub login_help: &'static str,
    pub login_error_format: &'static str,
    pub login_error_invalid: &'static str,
    pub login_error_user_not_found: &'static str,
    pub login_error_wrong_password: &'static str,

    pub register_title: &'static str,
    pub register_help: &'static str,
    pub register_error_format: &'static str,
    pub register_error_username_empty: &'static str,
    pub register_error_password_short: &'static str,
    pub register_error_name_empty: &'static str,
    pub register_error_rate: &'static str,
    pub register_error_exists: &'static str,

    pub timeline_title: &'static str,
    pub timeline_help: &'static str,
    pub timeline_no_posts: &'static str,
    pub timeline_page: &'static str,

    pub create_post_title: &'static str,
    pub create_post_url_title: &'static str,
    pub create_post_help: &'static str,
    pub create_post_help_url: &'static str,
    pub create_post_published: &'static str,
    pub create_post_published_img: &'static str,
    pub create_post_attach_prompt: &'static str,
    pub create_post_invalid_url: &'static str,
    pub create_post_enter_url: &'static str,
    pub create_post_attach_image: &'static str,

    pub post_detail_comments: &'static str,
    pub post_detail_no_comments: &'static str,
    pub post_detail_help_view: &'static str,
    pub post_detail_help_edit: &'static str,
    pub post_detail_help_comment: &'static str,
    pub post_detail_edit_title: &'static str,
    pub post_detail_comment_title: &'static str,
    pub post_detail_edited: &'static str,
    pub post_detail_deleted: &'static str,
    pub post_detail_delete_confirm: &'static str,
    pub post_detail_comment_deleted: &'static str,
    pub post_detail_image_hint: &'static str,

    pub profile_header_fmt: &'static str,
    pub profile_followers_count: &'static str,
    pub profile_following_count: &'static str,
    pub profile_header_posts: &'static str,
    pub profile_following: &'static str,
    pub profile_not_following: &'static str,
    pub profile_you: &'static str,
    pub profile_help: &'static str,
    pub profile_help_follow: &'static str,
    pub profile_help_unfollow: &'static str,
    pub profile_followed: &'static str,
    pub profile_unfollowed: &'static str,
    pub profile_delete_confirm: &'static str,
    pub profile_wrong_password: &'static str,
    pub profile_updated: &'static str,
    pub profile_empty_name: &'static str,
    pub profile_title_posts: &'static str,
    pub profile_title_followers: &'static str,
    pub profile_title_following: &'static str,
    pub profile_help_follow_list: &'static str,

    pub search_title: &'static str,
    pub search_results: &'static str,
    pub search_no_results: &'static str,
    pub search_help: &'static str,

    pub post_search_title: &'static str,
    pub post_search_filter_all: &'static str,
    pub post_search_filter_24h: &'static str,
    pub post_search_filter_7d: &'static str,
    pub post_search_filter_30d: &'static str,
    pub post_search_filter_title: &'static str,
    pub post_search_help: &'static str,
    pub post_search_results: &'static str,

    pub hashtag_title: &'static str,
    pub hashtag_no_posts: &'static str,
    pub hashtag_help: &'static str,
    pub hashtag_trending: &'static str,

    pub messages_title: &'static str,
    pub messages_empty: &'static str,
    pub messages_conversations: &'static str,
    pub messages_help: &'static str,

    pub chat_header: &'static str,
    pub chat_input_title: &'static str,

    pub notifications_title: &'static str,
    pub notifications_empty: &'static str,

    pub edit_profile_title: &'static str,
    pub edit_profile_name: &'static str,
    pub edit_profile_bio: &'static str,
    pub edit_profile_tz: &'static str,
    pub edit_profile_help: &'static str,

    pub image_downloading: &'static str,
    pub image_no_viewer: &'static str,
    pub image_press_enter: &'static str,
    pub image_press_q: &'static str,
    pub image_download_prompt: &'static str,
    pub image_download_header: &'static str,
    pub image_download_cmd: &'static str,
    pub image_download_scp: &'static str,
    pub image_download_info: &'static str,
    pub image_download_error: &'static str,
    pub image_download_instructions: &'static str,
    pub image_open_browser: &'static str,
    pub image_view: &'static str,

    pub status_bar_timeline: &'static str,
    pub status_bar_new_post: &'static str,
    pub status_bar_post: &'static str,
    pub status_bar_profile: &'static str,
    pub status_bar_search: &'static str,
    pub status_bar_messages: &'static str,
    pub status_bar_chat: &'static str,
    pub status_bar_edit: &'static str,
    pub status_bar_notifications: &'static str,
    pub status_bar_post_search: &'static str,

    pub follow_notif: &'static str,
    pub mention_notif: &'static str,
    pub page: &'static str,
    pub error: &'static str,
}

pub static S: Strings = Strings::new(&ES, &EN);

pub static ES: LangStrings = LangStrings {
    login_title: " Login (usuario:contraseña) ",
    login_help: "Tab: registrarse   Esc/Ctrl+q: salir",
    login_error_format: "Formato: usuario:contraseña",
    login_error_invalid: "Credenciales inválidas",
    login_error_user_not_found: "El usuario no existe. Regístrate primero (Tab)",
    login_error_wrong_password: "Contraseña incorrecta",

    register_title: " Registro (usuario:contraseña:nombre) ",
    register_help: "Tab: volver al login   Esc/Ctrl+q: salir",
    register_error_format: "Formato: usuario:contraseña:nombre",
    register_error_username_empty: "El usuario no puede estar vacío",
    register_error_password_short: "La contraseña debe tener al menos 4 caracteres",
    register_error_name_empty: "El nombre no puede estar vacío",
    register_error_rate: "Demasiados registros. Espera un minuto.",
    register_error_exists: "El usuario '@{}' ya existe",

    timeline_title: "📱 @{} — Timeline",
    timeline_help: "j/k: navegar   Enter: ver   /: buscar   #: trending   n: nuevo post   s: buscar usuarios   p: perfil   m: mensajes   Ctrl+n: notifs   i: imagen   q: salir",
    timeline_no_posts: "No hay posts en tu timeline",
    timeline_page: "Página {} — {} posts",

    create_post_title: " Nuevo Post ",
    create_post_url_title: " Pegar URL de imagen ",
    create_post_help: "Enter: publicar   u: subir imagen   Ctrl+U: desde URL   Esc: cancelar",
    create_post_help_url: "Enter: adjuntar imagen desde URL   Esc: cancelar",
    create_post_published: "Post publicado",
    create_post_published_img: "Post con imagen publicado",
    create_post_attach_prompt: "Pega la URL de la imagen y presiona Enter para adjuntarla",
    create_post_invalid_url: "URL no válida — solo jpg, png, gif, webp (Esc para cancelar)",
    create_post_enter_url: "Ingresa una URL o presiona Esc para cancelar",
    create_post_attach_image: "Ctrl+U: imagen desde URL   Esc: cancelar",

    post_detail_comments: " Comentarios ",
    post_detail_no_comments: "Sin comentarios",
    post_detail_help_view: "c: comentar   r: responder   i: imagen   ↑↓: navegar   d: eliminar comentario   e: editar   D: eliminar post   b: volver",
    post_detail_help_edit: "Enter: guardar   Esc: cancelar",
    post_detail_help_comment: "Enter: enviar   Esc: cancelar",
    post_detail_edit_title: " Editando post ",
    post_detail_comment_title: " Escribe un comentario ",
    post_detail_edited: "Post actualizado",
    post_detail_deleted: "Post eliminado",
    post_detail_delete_confirm: "¿Eliminar este post y todos sus comentarios? (y/n)",
    post_detail_comment_deleted: "Comentario eliminado",
    post_detail_image_hint: "Presiona i para ver la imagen",

    profile_header_fmt: "@{} — {}",
    profile_followers_count: "👥 {} seguidores",
    profile_following_count: "{} siguiendo",
    profile_header_posts: "{} posts",
    profile_following: "Siguiendo",
    profile_not_following: "No sigues",
    profile_you: "tu perfil",
    profile_help: "w: seguidores   g: siguiendo   f: follow/unfollow   e: editar perfil   x: borrar cuenta   m: mensaje   b: volver",
    profile_help_follow: "f: seguir",
    profile_help_unfollow: "f: dejar de seguir",
    profile_followed: "Siguiendo a @{}",
    profile_unfollowed: "Dejaste de seguir a @{}",
    profile_delete_confirm: "Escribe tu contraseña para borrar la cuenta (Esc para cancelar):",
    profile_wrong_password: "Contraseña incorrecta",
    profile_updated: "Perfil actualizado",
    profile_empty_name: "El nombre no puede estar vacío",
    profile_title_posts: " Posts ",
    profile_title_followers: " Seguidores ({}) ",
    profile_title_following: " Siguiendo ({}) ",
    profile_help_follow_list: "↑↓: navegar  Enter: ver perfil  Esc/b: volver",

    search_title: " Buscar usuarios ",
    search_results: " Resultados ",
    search_no_results: "Sin resultados",
    search_help: "Enter: buscar/seleccionar  Esc: volver",

    post_search_title: " Buscar posts [Filtro: {}] ",
    post_search_filter_all: "Todas las publicaciones",
    post_search_filter_24h: "Últimas 24 horas",
    post_search_filter_7d: "Últimos 7 días",
    post_search_filter_30d: "Últimos 30 días",
    post_search_filter_title: " Filtrar por fecha ",
    post_search_help: "Tab: cambiar filtro  Ctrl+f/p: cambiar página  Enter: buscar/seleccionar  Esc: volver",
    post_search_results: " Resultados ",

    hashtag_title: " #{} — Posts",
    hashtag_no_posts: "No hay posts con este hashtag",
    hashtag_help: "j/k: navegar   Enter: ver   b: volver   #: trending",
    hashtag_trending: " 🔥 Trending",

    messages_title: "📬 {} — Conversaciones",
    messages_empty: "No tienes conversaciones aún",
    messages_conversations: " Conversaciones ",
    messages_help: "j/k: navegar   Enter: abrir   b: volver   q: salir",

    chat_header: "💬 Chat con @{}  |  b: volver  Ctrl+q: salir",
    chat_input_title: " Mensaje (Enter: enviar) ",

    notifications_title: "🔔 Notificaciones",
    notifications_empty: "No tienes notificaciones",

    edit_profile_title: "✏️ Editando perfil — @{}",
    edit_profile_name: " Nombre ",
    edit_profile_bio: " Bio ",
    edit_profile_tz: " Zona horaria (UTC{:+d}) ",
    edit_profile_help: "Tab: cambiar campo   ↑↓: ajustar hora   Enter: guardar   Esc: cancelar",

    image_downloading: "⏳ Descargando imagen...",
    image_no_viewer: "No se encontró un visor de imágenes compatible.",
    image_press_enter: "Presiona Enter para volver...",
    image_press_q: "Presiona Enter, q o Esc para volver...",
    image_download_prompt: "d: descargar imagen  |  Enter/q/Esc: volver",
    image_download_header: "📥 Descargar imagen:",
    image_download_cmd: "Opción 1 — Copiar y pegar en tu terminal:",
    image_download_scp: "Opción 2 — Desde otra terminal con scp:",
    image_download_info: "Archivo:",
    image_download_error: "Error al descargar la imagen (timeout o URL inválida).",
    image_download_instructions: "📷 Imagen:",
    image_open_browser: "Ábrela en tu navegador para descargarla.",
    image_view: " Imagen ",

    status_bar_timeline: "Timeline",
    status_bar_new_post: "Nuevo Post",
    status_bar_post: "Post",
    status_bar_profile: "Perfil",
    status_bar_search: "Buscar",
    status_bar_messages: "Mensajes",
    status_bar_chat: "Chat",
    status_bar_edit: "Editar Perfil",
    status_bar_notifications: "Notificaciones",
    status_bar_post_search: "Buscar Posts",

    follow_notif: "@{} te ha seguido",
    mention_notif: "@{} te mencionó",
    page: "Página {}",
    error: "Error",
};

pub static EN: LangStrings = LangStrings {
    login_title: " Login (user:password) ",
    login_help: "Tab: register   Esc/Ctrl+q: exit",
    login_error_format: "Format: user:password",
    login_error_invalid: "Invalid credentials",
    login_error_user_not_found: "User does not exist. Register first (Tab)",
    login_error_wrong_password: "Wrong password",

    register_title: " Register (user:password:name) ",
    register_help: "Tab: back to login   Esc/Ctrl+q: exit",
    register_error_format: "Format: user:password:name",
    register_error_username_empty: "Username cannot be empty",
    register_error_password_short: "Password must be at least 4 characters",
    register_error_name_empty: "Name cannot be empty",
    register_error_rate: "Too many registrations. Wait a minute.",
    register_error_exists: "User '@{}' already exists",

    timeline_title: "📱 @{} — Timeline",
    timeline_help: "j/k: navigate   Enter: view   /: search   #: trending   n: new post   s: search users   p: profile   m: messages   Ctrl+n: notifs   i: image   q: quit",
    timeline_no_posts: "No posts in your timeline",
    timeline_page: "Page {} — {} posts",

    create_post_title: " New Post ",
    create_post_url_title: " Paste image URL ",
    create_post_help: "Enter: publish   u: upload image   Ctrl+U: from URL   Esc: cancel",
    create_post_help_url: "Enter: attach image from URL   Esc: cancel",
    create_post_published: "Post published",
    create_post_published_img: "Post with image published",
    create_post_attach_prompt: "Paste the image URL and press Enter to attach it",
    create_post_invalid_url: "Invalid URL — only jpg, png, gif, webp (Esc to cancel)",
    create_post_enter_url: "Enter a URL or press Esc to cancel",
    create_post_attach_image: "Ctrl+U: image from URL   Esc: cancel",

    post_detail_comments: " Comments ",
    post_detail_no_comments: "No comments",
    post_detail_help_view: "c: comment   r: reply   i: image   ↑↓: navigate   d: delete comment   e: edit   D: delete post   b: back",
    post_detail_help_edit: "Enter: save   Esc: cancel",
    post_detail_help_comment: "Enter: send   Esc: cancel",
    post_detail_edit_title: " Editing post ",
    post_detail_comment_title: " Write a comment ",
    post_detail_edited: "Post updated",
    post_detail_deleted: "Post deleted",
    post_detail_delete_confirm: "Delete this post and all its comments? (y/n)",
    post_detail_comment_deleted: "Comment deleted",
    post_detail_image_hint: "Press i to view the image",

    profile_header_fmt: "@{} — {}",
    profile_followers_count: "👥 {} followers",
    profile_following_count: "{} following",
    profile_header_posts: "{} posts",
    profile_following: "Following",
    profile_not_following: "Not following",
    profile_you: "your profile",
    profile_help: "w: followers   g: following   f: follow/unfollow   e: edit profile   x: delete account   m: message   b: back",
    profile_help_follow: "f: follow",
    profile_help_unfollow: "f: unfollow",
    profile_followed: "Following @{}",
    profile_unfollowed: "Unfollowed @{}",
    profile_delete_confirm: "Enter your password to delete the account (Esc to cancel):",
    profile_wrong_password: "Wrong password",
    profile_updated: "Profile updated",
    profile_empty_name: "Name cannot be empty",
    profile_title_posts: " Posts ",
    profile_title_followers: " Followers ({}) ",
    profile_title_following: " Following ({}) ",
    profile_help_follow_list: "↑↓: navigate  Enter: view profile  Esc/b: back",

    search_title: " Search users ",
    search_results: " Results ",
    search_no_results: "No results",
    search_help: "Enter: search/select  Esc: back",

    post_search_title: " Search posts [Filter: {}] ",
    post_search_filter_all: "All posts",
    post_search_filter_24h: "Last 24 hours",
    post_search_filter_7d: "Last 7 days",
    post_search_filter_30d: "Last 30 days",
    post_search_filter_title: " Filter by date ",
    post_search_help: "Tab: change filter  Ctrl+f/p: change page  Enter: search/select  Esc: back",
    post_search_results: " Results ",

    hashtag_title: " #{} — Posts",
    hashtag_no_posts: "No posts with this hashtag",
    hashtag_help: "j/k: navigate   Enter: view   b: back   #: trending",
    hashtag_trending: " 🔥 Trending",

    messages_title: "📬 {} — Conversations",
    messages_empty: "No conversations yet",
    messages_conversations: " Conversations ",
    messages_help: "j/k: navigate   Enter: open   b: back   q: quit",

    chat_header: "💬 Chat with @{}  |  b: back  Ctrl+q: quit",
    chat_input_title: " Message (Enter: send) ",

    notifications_title: "🔔 Notifications",
    notifications_empty: "No notifications",

    edit_profile_title: "✏️ Editing profile — @{}",
    edit_profile_name: " Name ",
    edit_profile_bio: " Bio ",
    edit_profile_tz: " Timezone (UTC{:+d}) ",
    edit_profile_help: "Tab: switch field   ↑↓: adjust time   Enter: save   Esc: cancel",

    image_downloading: "⏳ Downloading image...",
    image_no_viewer: "No compatible image viewer found.",
    image_press_enter: "Press Enter to return...",
    image_press_q: "Press Enter, q or Esc to return...",
    image_download_prompt: "d: download image  |  Enter/q/Esc: back",
    image_download_header: "📥 Download image:",
    image_download_cmd: "Option 1 — Copy and paste in your terminal:",
    image_download_scp: "Option 2 — From another terminal with scp:",
    image_download_info: "File:",
    image_download_error: "Error downloading image (timeout or invalid URL).",
    image_download_instructions: "📷 Image:",
    image_open_browser: "Open in your browser to download.",
    image_view: " Image ",

    status_bar_timeline: "Timeline",
    status_bar_new_post: "New Post",
    status_bar_post: "Post",
    status_bar_profile: "Profile",
    status_bar_search: "Search",
    status_bar_messages: "Messages",
    status_bar_chat: "Chat",
    status_bar_edit: "Edit Profile",
    status_bar_notifications: "Notifications",
    status_bar_post_search: "Search Posts",

    follow_notif: "@{} followed you",
    mention_notif: "@{} mentioned you",
    page: "Page {}",
    error: "Error",
};
