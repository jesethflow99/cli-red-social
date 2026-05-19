# Plan: Sistema de Foros / Hilos por Temática

## Problema actual

El timeline solo muestra posts de personas que sigues. No hay forma de descubrir
contenido por temática, unirse a discusiones grupales, o encontrar hilos de interés.
Esto funciona más como un canal de comunicación descentralizado que como una red social.

## Objetivo

Agregar un sistema de foros con categorías temáticas donde cualquier usuario pueda:
- Explorar temas por categoría
- Crear nuevos topics
- Responder a topics existentes
- Buscar contenido por temática (no solo por persona)

---

## 1. Modelo de datos

### Tablas nuevas

```sql
CREATE TABLE categories (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    icon TEXT NOT NULL DEFAULT '📁',
    "order" INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE topics (
    id BIGSERIAL PRIMARY KEY,
    category_id BIGINT NOT NULL REFERENCES categories(id),
    title TEXT NOT NULL,
    author_id BIGINT NOT NULL REFERENCES users(id),
    pinned BOOLEAN NOT NULL DEFAULT false,
    locked BOOLEAN NOT NULL DEFAULT false,
    views BIGINT NOT NULL DEFAULT 0,
    last_reply_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE forum_posts (
    id BIGSERIAL PRIMARY KEY,
    topic_id BIGINT NOT NULL REFERENCES topics(id),
    author_id BIGINT NOT NULL REFERENCES users(id),
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Índices
CREATE INDEX idx_topics_category ON topics(category_id);
CREATE INDEX idx_topics_author ON topics(author_id);
CREATE INDEX idx_forum_posts_topic ON forum_posts(topic_id);
CREATE INDEX idx_forum_posts_author ON forum_posts(author_id);
CREATE INDEX idx_topics_last_reply ON topics(last_reply_at DESC);
```

### Categorías iniciales (seed)

| Icon | Nombre | Slug | Descripción |
|------|--------|------|-------------|
| 💻 | Tecnología | tech | Programación, hardware, software, IA |
| 🎮 | Gaming | gaming | Videojuegos, esports, reviews |
| 🎨 | Arte y Diseño | art | Ilustración, diseño gráfico, fotografía |
| 📚 | Ciencia | science | Física, biología, matemáticas, espacio |
| 🎵 | Música | music | Géneros, producción, instrumentos |
| 📰 | Noticias | news | Actualidad, política, economía |
| 🌍 | Off-topic | off-topic | Todo lo demás, charla libre |
| 💡 | Proyectos | projects | Muestra tu trabajo, busca colaboradores |

---

## 2. Operaciones de base de datos (DatabaseOps)

### Nuevos métodos en el trait

```rust
fn create_category(&self, name: &str, slug: &str, description: &str, icon: &str) -> Result<Category>;
fn get_categories(&self) -> Result<Vec<Category>>;
fn create_topic(&self, category_id: i64, author_id: i64, title: &str, content: &str) -> Result<Topic>;
fn get_topics(&self, category_id: i64, offset: u64, limit: u64) -> Result<Vec<TopicWithStats>>;
fn get_topic_detail(&self, topic_id: i64) -> Result<Option<TopicWithStats>>;
fn get_topic_posts(&self, topic_id: i64, offset: u64, limit: u64) -> Result<Vec<ForumPost>>;
fn add_forum_post(&self, topic_id: i64, author_id: i64, content: &str) -> Result<ForumPost>;
fn search_topics(&self, query: &str, category_id: Option<i64>, offset: u64, limit: u64) -> Result<Vec<TopicWithStats>>;
fn pin_topic(&self, topic_id: i64, user_id: i64) -> Result<()>;
fn lock_topic(&self, topic_id: i64, user_id: i64) -> Result<()>;
fn delete_topic(&self, topic_id: i64, user_id: i64) -> Result<()>;
fn delete_forum_post(&self, post_id: i64, user_id: i64) -> Result<()>;
fn increment_topic_views(&self, topic_id: i64) -> Result<()>;
```

### Rate limits

| Acción | Límite | Ventana |
|--------|--------|---------|
| Crear topic | 3 | 60s |
| Responder topic | 10 | 60s |
| Límite contenido | 10,000 chars | por post |

---

## 3. Nuevos modelos (models.rs)

```rust
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub icon: String,
    pub order: i32,
    pub created_at: DateTime<Utc>,
}

pub struct Topic {
    pub id: i64,
    pub category_id: i64,
    pub title: String,
    pub author_id: i64,
    pub author_username: String,
    pub pinned: bool,
    pub locked: bool,
    pub views: i64,
    pub reply_count: i64,
    pub last_reply_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub struct ForumPost {
    pub id: i64,
    pub topic_id: i64,
    pub author_id: i64,
    pub author_username: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}
```

---

## 4. Nuevas pantallas TUI

### Screen::ForumCategories
- Lista de categorías con icono, nombre, descripción y conteo de topics
- Navegación con j/k
- Enter → Screen::ForumTopics(category_id)
- `/` → buscar topics globalmente

```
┌── Foros ──────────────────────────────────────────────────────────────────┐
│ 💻  Tecnología              [128 topics]  Programación, hardware, IA       │
│ 🎮  Gaming                  [85 topics]   Videojuegos, esports, reviews    │
│ 🎨  Arte y Diseño           [42 topics]   Ilustración, diseño, fotografía  │
│ 📚  Ciencia                 [67 topics]   Física, biología, matemáticas    │
│ 🎵  Música                  [31 topics]   Géneros, producción, instrumentos│
│ 📰  Noticias                [53 topics]   Actualidad, política, economía   │
│ 🌍  Off-topic               [210 topics]  Todo lo demás, charla libre      │
│ 💡  Proyectos               [19 topics]   Muestra tu trabajo               │
│                                                                         │
│ n: nuevo topic   /: buscar   q: volver                                    │
└───────────────────────────────────────────────────────────────────────────┘
```

### Screen::ForumTopics(category_id)
- Lista de topics en una categoría
- Topics pinned primero (marcados con 📌)
- Topics locked marcados con 🔒
- Muestra: título, autor, replies, views, última respuesta
- Enter → Screen::ForumThread(topic_id)
- `n` → crear nuevo topic
- `b` → volver a categorías

```
┌── 💻 Tecnología ──────────────────────────────────────────────────────────┐
│ 📌 ¿Cómo empezar con Rust en 2026?    @alice   45r  1.2k   hace 2h       │
│ 🔒 Debate: Linux vs Windows           @bob     128r 3.4k   hace 5h       │
│    Mejor framework web para CLI       @carol   12r  340   hace 1d        │
│    Mi setup de desarrollo             @dave    8r   210   hace 2d        │
│                                                                         │
│ 8 topics — Página 1                                                         │
│ n: nuevo topic   b: volver   j/k: navegar   Enter: abrir                  │
└───────────────────────────────────────────────────────────────────────────┘
```

### Screen::ForumThread(topic_id)
- Vista del hilo completo con posts
- Primer post es el OP (original post)
- Respuestas debajo en orden cronológico
- Paginación si hay muchos posts
- `c` → responder
- `e` → editar (si es tu post)
- `d` → borrar (si es tu post o eres mod)
- `b` → volver a topics

```
┌── ¿Cómo empezar con Rust en 2026? — por @alice ───────────────────────────┐
│ ┌───────────────────────────────────────────────────────────────────────┐ │
│ │ @alice · hace 3 días                                                   │ │
│ │                                                                        │ │
│ │ Hola! Quiero aprender Rust pero no sé por dónde empezar.               │ │
│ │ ¿Alguien tiene recomendaciones?                                        │ │
│ └───────────────────────────────────────────────────────────────────────┘ │
│ ┌───────────────────────────────────────────────────────────────────────┐ │
│ │ @bob · hace 2 días                                                     │ │
│ │                                                                        │ │
│ │ Te recomiendo "The Rust Book" y luego hacer proyectos pequeños.        │ │
│ └───────────────────────────────────────────────────────────────────────┘ │
│ ┌───────────────────────────────────────────────────────────────────────┐ │
│ │ @carol · hace 1 día                                                    │ │
│ │                                                                        │ │
│ │ También mira el crate tokio para async.                                │ │
│ └───────────────────────────────────────────────────────────────────────┘ │
│                                                                          │
│ 3 respuestas — Página 1                                                   │
│ c: responder   b: volver   j/k: navegar                                   │
└───────────────────────────────────────────────────────────────────────────┘
```

### Screen::ForumCreateTopic
- Input para título
- Input para contenido del primer post
- Selección de categoría (si no viene de una categoría específica)

### Screen::ForumCreatePost
- Input para contenido de la respuesta
- Similar al create post actual pero sin imagen

---

## 5. Navegación y teclas

### Teclas globales (agregar a la barra de status)
- `F` (shift+f) → ir a Foros desde cualquier pantalla

### Flujo de navegación
```
Timeline → F → ForumCategories
ForumCategories → Enter → ForumTopics(cat_id)
ForumTopics → Enter → ForumThread(topic_id)
ForumThread → b → ForumTopics
ForumTopics → b → ForumCategories
Cualquiera → b → pantalla anterior
```

---

## 6. i18n strings nuevos

### Español
```rust
forum_title: "📋 Foros",
forum_categories: "Categorías",
forum_topics: "Topics",
forum_replies: "respuestas",
forum_views: "vistas",
forum_new_topic: "Nuevo topic",
forum_new_topic_title: " Título del topic ",
forum_new_topic_content: " Contenido ",
forum_new_topic_category: " Categoría ",
forum_topic_pinned: "Fijado",
forum_topic_locked: "Cerrado",
forum_topic_created: "Topic creado",
forum_reply_title: " Responder ",
forum_reply_sent: "Respuesta enviada",
forum_no_topics: "No hay topics en esta categoría",
forum_no_posts: "Sin respuestas aún. ¡Sé el primero!",
forum_search_title: " Buscar topics ",
forum_delete_topic: "¿Eliminar este topic? (D para confirmar)",
forum_delete_post: "¿Eliminar este post? (D para confirmar)",
forum_topic_deleted: "Topic eliminado",
forum_post_deleted: "Respuesta eliminada",
forum_pin_toggle: "Topic fijado/desfijado",
forum_lock_toggle: "Topic cerrado/abierto",
forum_cannot_reply_locked: "Este topic está cerrado",
forum_help_categories: "j/k: navegar  Enter: abrir  n: nuevo topic  /: buscar  F: foros  b: volver",
forum_help_topics: "j/k: navegar  Enter: abrir  n: nuevo topic  b: volver",
forum_help_thread: "j/k: navegar  c: responder  e: editar  d: borrar  b: volver",
```

### English (equivalentes)

---

## 7. Implementación paso a paso

### Fase 1: Backend (DB + modelos)
1. Agregar tablas en `init_schema()` con migración
2. Crear structs en `models.rs` (Category, Topic, ForumPost)
3. Implementar métodos en `DatabaseOps`
4. Seed de categorías iniciales
5. Tests unitarios

### Fase 2: TUI básico
1. Agregar `Screen::ForumCategories`, `ForumTopics`, `ForumThread`
2. Render de lista de categorías
3. Render de lista de topics
4. Render de hilo con posts
5. Navegación básica (j/k, Enter, b)

### Fase 3: Creación y edición
1. Crear nuevo topic
2. Responder a topic
3. Editar/borrar posts propios
4. Validación de contenido (max 10k chars)
5. Rate limits

### Fase 4: Búsqueda y paginación
1. Buscar topics por título/contenido
2. Filtrar por categoría
3. Paginación de topics y posts
4. Contador de views

### Fase 5: Moderación
1. Pin/unpin topics
2. Lock/unlock topics
3. Borrar cualquier post (solo admin)
4. Indicadores visuales 📌 🔒

---

## 8. Consideraciones de rendimiento

- **Índices** en todas las foreign keys y columnas de búsqueda
- **Paginación** obligatoria en topics y posts (nunca traer todos)
- **Contador de replies** como columna denormalizada en `topics` para no hacer COUNT cada vez
- **Views** con UPDATE incremental (no necesita ser exacto en tiempo real)
- **Rate limits en DB** (ya implementados con la tabla `rate_limits`)

---

## 9. Archivos a modificar

| Archivo | Cambios |
|---------|---------|
| `src/db.rs` | +3 tablas, +13 métodos, seed categories |
| `src/models.rs` | +3 structs, +1 Screen variant |
| `src/app.rs` | +5 pantallas, +5 handlers de teclas, +5 renders |
| `src/i18n.rs` | +30 strings (es + en) |
