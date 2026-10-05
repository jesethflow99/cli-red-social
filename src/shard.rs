//! ShardManager — mesh de archivos SQLite.
//!
//! Divide la base en N archivos `.db` (shards). Cada shard es un `Database`
//! con un `id_base = shard_idx * MAX_USER_ID_PER_SHARD`, de modo que el id
//! global de un usuario codifica su shard: `shard = user_id / MAX_USER_ID_PER_SHARD`.
//!
//! Las operaciones de un solo usuario van directo a su shard. Las que cruzan
//! usuarios (timeline, búsqueda, trending, conversaciones) hacen fan-out a todos
//! los shards y combinan los resultados.
//!
//! Activación: `AGORA_DB_SHARDS=N`. Si no se setea, el sistema usa un solo
//! archivo (sin ShardManager).

use anyhow::Result;

use crate::db::DatabaseOps;
use crate::models::{Comment, Message, MessagePreview, Notification, Post, User};

use crate::db::AuthResult;

const MAX_USER_ID_PER_SHARD: i64 = crate::db::MAX_USER_ID_PER_SHARD;

/// Abre la base (un solo archivo o un mesh de shards según `AGORA_DB_SHARDS`).
pub fn open_database(base_path: &str) -> Result<Box<dyn DatabaseOps>> {
    let n = std::env::var("AGORA_DB_SHARDS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    if n == 1 {
        Ok(Box::new(crate::db::Database::new(base_path)?))
    } else {
        Ok(Box::new(ShardManager::new(base_path, n)?))
    }
}

pub fn shard_count() -> usize {
    std::env::var("AGORA_DB_SHARDS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1)
}

/// Devuelve el índice del shard que posee a un usuario según su id global.
pub fn shard_for_user(user_id: i64) -> usize {
    (user_id / MAX_USER_ID_PER_SHARD).max(0) as usize
}

/// Devuelve el índice del shard que posee a un usuario según su nombre
/// (usado en registro/autenticación, donde todavía no hay id).
fn shard_for_username(username: &str, n: usize) -> usize {
    let mut hash: u64 = 5381;
    for b in username.to_lowercase().bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    (hash % n as u64) as usize
}

pub struct ShardManager {
    shards: Vec<crate::db::Database>,
    paths: Vec<String>,
    base_path: String,
}

impl ShardManager {
    /// Abre N shards. `base_path` es el archivo base (ej: `agora.db`);
    /// los shards adicionales se llaman `agora-1.db`, `agora-2.db`, ...
    pub fn new(base_path: &str, n: usize) -> Result<Self> {
        let n = n.max(1);
        let (dir, stem) = match base_path.rfind('/') {
            Some(i) => (&base_path[..i], &base_path[i + 1..]),
            None => ("", base_path),
        };
        let mut shards = Vec::with_capacity(n);
        let mut paths = Vec::with_capacity(n);
        for i in 0..n {
            let path = if i == 0 {
                base_path.to_string()
            } else {
                let dot = stem.rfind('.').unwrap_or(stem.len());
                let name = format!("{}-{}{}", &stem[..dot], i, &stem[dot..]);
                if dir.is_empty() {
                    name
                } else {
                    format!("{}/{}", dir, name)
                }
            };
            let id_base = i as i64 * MAX_USER_ID_PER_SHARD;
            let db = crate::db::Database::with_id_base(&path, id_base)?;
            shards.push(db);
            paths.push(path);
        }
        Ok(Self {
            shards,
            paths,
            base_path: base_path.to_string(),
        })
    }

    #[allow(dead_code)]
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Condensa el mesh en un único archivo principal: fusiona todos los
    /// shards en `base_path` y elimina los archivos secundarios. Es una
    /// operación de mantenimiento para cuando baja la concurrencia.
    pub fn condense(&self) -> Result<String> {
        if self.shards.len() == 1 {
            return Ok(self.base_path.clone());
        }
        // Archivo temporal para no pisar el shard 0 mientras se lee.
        let tmp = format!("{}.condensed", self.base_path);
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(format!("{}-wal", tmp));
        let _ = std::fs::remove_file(format!("{}-shm", tmp));

        let target = crate::db::Database::new(&tmp)?;
        // Copia cada shard (incluido el 0) al archivo temporal.
        for path in &self.paths {
            target.merge_from_file(path)?;
        }
        target.finalize_merge()?;
        drop(target);

        // Reemplaza el archivo base por el condensado y borra los secundarios.
        std::fs::rename(&tmp, &self.base_path)?;
        for path in self.paths.iter().skip(1) {
            let _ = std::fs::remove_file(path);
            let _ = std::fs::remove_file(format!("{}-wal", path));
            let _ = std::fs::remove_file(format!("{}-shm", path));
        }
        Ok(self.base_path.clone())
    }

    fn shard(&self, idx: usize) -> &crate::db::Database {
        &self.shards[idx.min(self.shards.len() - 1)]
    }

    fn shard_for_id(&self, user_id: i64) -> &crate::db::Database {
        let idx = shard_for_user(user_id);
        self.shard(idx)
    }

    fn shard_for_name(&self, username: &str) -> &crate::db::Database {
        let idx = shard_for_username(username, self.shards.len());
        self.shard(idx)
    }

    /// Suma de funciones de los shards para operaciones que tocan a todos.
    fn fan_out<T, F>(&self, mut f: F) -> Result<Vec<T>>
    where
        F: FnMut(&crate::db::Database) -> Result<Vec<T>>,
    {
        let mut out = Vec::new();
        for shard in &self.shards {
            out.extend(f(shard)?);
        }
        Ok(out)
    }
}

impl DatabaseOps for ShardManager {
    fn register_user(
        &self,
        username: &str,
        password: &str,
        display_name: &str,
        invite_code: Option<&str>,
    ) -> Result<User> {
        // Verifica globalmente que el username no exista en ningún shard.
        for shard in &self.shards {
            if shard.user_exists(username)? {
                anyhow::bail!("El usuario ya existe");
            }
        }
        // En modo invite, el código vive en el shard 0 (creado con
        // `--invite-create`); registrar ahí mantiene consistente el routing
        // por id (el id global codifica el shard 0).
        let mode = std::env::var("REGISTRATION_MODE")
            .unwrap_or_else(|_| "open".to_string())
            .to_lowercase();
        if mode == "invite" {
            self.shard(0).register_user(username, password, display_name, invite_code)
        } else {
            let target = self.shard_for_name(username);
            target.register_user(username, password, display_name, invite_code)
        }
    }

    fn check_register_rate_limit(&self) -> Result<()> {
        self.shard(0).check_register_rate_limit()
    }

    fn authenticate(&self, username: &str, password: &str) -> Result<AuthResult> {
        let target = self.shard_for_name(username);
        match target.authenticate(username, password)? {
            AuthResult::UserNotFound => {
                // Reintenta en todos los shards por si el routing por nombre
                // no coincide con el routing por id del registro original.
                for shard in &self.shards {
                    match shard.authenticate(username, password)? {
                        AuthResult::UserNotFound => continue,
                        other => return Ok(other),
                    }
                }
                Ok(AuthResult::UserNotFound)
            }
            other => Ok(other),
        }
    }

    fn get_user_by_id(&self, id: i64) -> Result<Option<User>> {
        self.shard_for_id(id).get_user_by_id(id)
    }

    fn search_users(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<User>> {
        let mut all = self.fan_out(|s| s.search_users(query, 0, limit + offset))?;
        all.sort_by(|a, b| a.username.cmp(&b.username));
        all.dedup_by(|a, b| a.id == b.id);
        Ok(all.into_iter().skip(offset as usize).take(limit as usize).collect())
    }

    fn create_post(&self, user_id: i64, content: &str, image_path: Option<&str>) -> Result<Post> {
        self.shard_for_id(user_id).create_post(user_id, content, image_path)
    }

    fn get_timeline(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>> {
        // Usuarios que sigo (viven en mi shard o en otros).
        let following_ids = self.shard_for_id(user_id).get_following_ids(user_id)?;
        let mut ids: Vec<i64> = following_ids;
        ids.push(user_id);
        // Agrupa los ids por shard y consulta cada uno.
        let mut posts = Vec::new();
        for shard in &self.shards {
            let base = shard.id_base();
            let local: Vec<i64> = ids
                .iter()
                .copied()
                .filter(|id| id / MAX_USER_ID_PER_SHARD == base / MAX_USER_ID_PER_SHARD)
                .collect();
            posts.extend(shard.get_posts_by_users(&local, 0, limit + offset)?);
        }
        posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        posts.dedup_by(|a, b| a.id == b.id);
        Ok(posts.into_iter().skip(offset as usize).take(limit as usize).collect())
    }

    fn follow_user(&self, follower_id: i64, following_id: i64) -> Result<()> {
        self.shard_for_id(follower_id)
            .follow_user(follower_id, following_id)
    }

    fn unfollow_user(&self, follower_id: i64, following_id: i64) -> Result<()> {
        self.shard_for_id(follower_id)
            .unfollow_user(follower_id, following_id)
    }

    fn is_following(&self, follower_id: i64, following_id: i64) -> Result<bool> {
        self.shard_for_id(follower_id)
            .is_following(follower_id, following_id)
    }

    fn get_followers(&self, user_id: i64) -> Result<Vec<User>> {
        let mut ids = self.fan_out(|s| s.get_follower_ids(user_id))?;
        ids.sort();
        ids.dedup();
        let mut users = Vec::new();
        for id in ids {
            if let Some(u) = self.get_user_by_id(id)? {
                users.push(u);
            }
        }
        users.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(users)
    }

    fn get_following(&self, user_id: i64) -> Result<Vec<User>> {
        let ids = self.shard_for_id(user_id).get_following_ids(user_id)?;
        let mut users = Vec::new();
        for id in ids {
            if let Some(u) = self.get_user_by_id(id)? {
                users.push(u);
            }
        }
        users.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(users)
    }

    fn get_posts_by_user(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>> {
        self.shard_for_id(user_id).get_posts_by_user(user_id, offset, limit)
    }

    fn get_post_by_id(&self, post_id: i64) -> Result<Option<Post>> {
        // El post lo crea un usuario; su id global lo ubica en el shard del autor.
        let author_shard = shard_for_user(post_id);
        for shard in &self.shards {
            let idx = shard.id_base() / MAX_USER_ID_PER_SHARD;
            if idx as usize == author_shard {
                return shard.get_post_by_id(post_id);
            }
        }
        self.shard(0).get_post_by_id(post_id)
    }

    fn add_comment(
        &self,
        post_id: i64,
        user_id: i64,
        content: &str,
        parent_id: Option<i64>,
    ) -> Result<Comment> {
        // El comentario vive en el shard del autor del comentario.
        self.shard_for_id(user_id).add_comment(post_id, user_id, content, parent_id)
    }

    fn get_comments(&self, post_id: i64) -> Result<Vec<Comment>> {
        let mut all = self.fan_out(|s| s.get_comments(post_id))?;
        all.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        all.dedup_by(|a, b| a.id == b.id);
        Ok(all)
    }

    fn update_post(&self, post_id: i64, user_id: i64, content: &str) -> Result<()> {
        self.shard_for_id(user_id).update_post(post_id, user_id, content)
    }

    fn delete_post(&self, post_id: i64, user_id: i64) -> Result<()> {
        self.shard_for_id(user_id).delete_post(post_id, user_id)
    }

    fn delete_comment(&self, comment_id: i64, user_id: i64) -> Result<()> {
        self.shard_for_id(user_id).delete_comment(comment_id, user_id)
    }

    fn delete_user(&self, user_id: i64) -> Result<()> {
        // Un usuario puede tener datos (mensajes, follows) en cualquier shard.
        for shard in &self.shards {
            shard.delete_user(user_id)?;
        }
        Ok(())
    }

    fn send_message(
        &self,
        sender_id: i64,
        receiver_id: i64,
        content: &str,
        encrypted: bool,
    ) -> Result<Message> {
        // El mensaje lo crea el sender en su shard; el receiver lo lee
        // consultando el shard del sender.
        self.shard_for_id(sender_id)
            .send_message(sender_id, receiver_id, content, encrypted)
    }

    fn get_conversations(&self, user_id: i64) -> Result<Vec<User>> {
        let mut ids = self.fan_out(|s| s.get_conversation_partner_ids(user_id))?;
        ids.sort();
        ids.dedup();
        let mut users = Vec::new();
        for id in ids {
            if let Some(u) = self.get_user_by_id(id)? {
                users.push(u);
            }
        }
        users.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(users)
    }

    fn get_messages(&self, user_id: i64, other_id: i64) -> Result<Vec<Message>> {
        // Los mensajes de una conversación pueden estar en el shard del
        // sender de cada uno; consultamos ambos.
        let mut all = Vec::new();
        for shard in &self.shards {
            all.extend(shard.get_messages(user_id, other_id)?);
        }
        all.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        all.dedup_by(|a, b| a.id == b.id);
        for m in &mut all {
            if let Some(u) = self.get_user_by_id(m.sender_id)? {
                m.sender_username = u.username;
            }
        }
        Ok(all)
    }

    fn get_unread_count(&self, user_id: i64) -> Result<i64> {
        let mut total = 0i64;
        for shard in &self.shards {
            total += shard.get_unread_count(user_id)?;
        }
        Ok(total)
    }

    fn get_recent_message_previews(&self, user_id: i64, limit: i64) -> Result<Vec<MessagePreview>> {
        let mut all = self.fan_out(|s| s.get_recent_message_previews(user_id, limit * 4))?;
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all.dedup_by(|a, b| a.sender_id == b.sender_id);
        let mut taken: Vec<MessagePreview> = all.into_iter().take(limit as usize).collect();
        for m in &mut taken {
            if let Some(u) = self.get_user_by_id(m.sender_id)? {
                m.sender_username = u.username;
            }
        }
        Ok(taken)
    }

    fn mark_messages_read(&self, user_id: i64, other_id: i64) -> Result<()> {
        for shard in &self.shards {
            shard.mark_messages_read(user_id, other_id)?;
        }
        Ok(())
    }

    fn update_profile(
        &self,
        user_id: i64,
        display_name: &str,
        bio: &str,
        utc_offset: i32,
    ) -> Result<()> {
        self.shard_for_id(user_id)
            .update_profile(user_id, display_name, bio, utc_offset)
    }

    fn update_timezone(&self, user_id: i64, utc_offset: i32) -> Result<()> {
        self.shard_for_id(user_id).update_timezone(user_id, utc_offset)
    }

    fn add_notification(
        &self,
        user_id: i64,
        from_user_id: i64,
        notif_type: &str,
        related_id: Option<i64>,
    ) -> Result<()> {
        // La notificación va al shard del receptor.
        self.shard_for_id(user_id)
            .add_notification(user_id, from_user_id, notif_type, related_id)
    }

    fn get_notifications(
        &self,
        user_id: i64,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Notification>> {
        let mut notifications =
            self.shard_for_id(user_id).get_notifications(user_id, offset, limit)?;
        for n in &mut notifications {
            if let Some(u) = self.get_user_by_id(n.from_user_id)? {
                n.from_username = u.username;
            }
        }
        Ok(notifications)
    }

    fn get_unread_notifications_count(&self, user_id: i64) -> Result<i64> {
        self.shard_for_id(user_id).get_unread_notifications_count(user_id)
    }

    fn mark_notifications_read(&self, user_id: i64) -> Result<()> {
        self.shard_for_id(user_id).mark_notifications_read(user_id)
    }

    fn search_posts(
        &self,
        query: &str,
        time_filter: &str,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Post>> {
        let mut all = self.fan_out(|s| s.search_posts(query, time_filter, 0, limit + offset))?;
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all.dedup_by(|a, b| a.id == b.id);
        Ok(all.into_iter().skip(offset as usize).take(limit as usize).collect())
    }

    fn search_posts_by_user(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        let mut all = self.fan_out(|s| s.search_posts_by_user(query, 0, limit + offset))?;
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all.dedup_by(|a, b| a.id == b.id);
        Ok(all.into_iter().skip(offset as usize).take(limit as usize).collect())
    }

    fn search_posts_by_date(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        let mut all = self.fan_out(|s| s.search_posts_by_date(query, 0, limit + offset))?;
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all.dedup_by(|a, b| a.id == b.id);
        Ok(all.into_iter().skip(offset as usize).take(limit as usize).collect())
    }

    fn check_rate_limit(
        &self,
        user_id: i64,
        action: &str,
        max: usize,
        window_secs: u64,
    ) -> Result<()> {
        self.shard_for_id(user_id)
            .check_rate_limit(user_id, action, max, window_secs)
    }

    fn cleanup_old_data(&self, days: i64) -> Result<(u64, u64)> {
        let mut msgs = 0u64;
        let mut notifs = 0u64;
        for shard in &self.shards {
            let (m, n) = shard.cleanup_old_data(days)?;
            msgs += m;
            notifs += n;
        }
        Ok((msgs, notifs))
    }

    fn cleanup_inactive_users(&self, days: i64) -> Result<u64> {
        let mut total = 0u64;
        for shard in &self.shards {
            total += shard.cleanup_inactive_users(days)?;
        }
        Ok(total)
    }

    fn get_posts_by_hashtag(&self, tag: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        let mut all = self.fan_out(|s| s.get_posts_by_hashtag(tag, 0, limit + offset))?;
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        all.dedup_by(|a, b| a.id == b.id);
        Ok(all.into_iter().skip(offset as usize).take(limit as usize).collect())
    }

    fn get_trending_hashtags(&self, limit: u64) -> Result<Vec<(String, i64)>> {
        let all = self.fan_out(|s| s.get_trending_hashtags(limit * 4))?;
        let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for (tag, cnt) in all {
            *counts.entry(tag).or_insert(0) += cnt;
        }
        let mut v: Vec<(String, i64)> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.truncate(limit as usize);
        Ok(v)
    }

    fn export_user_data(&self, username: &str) -> Result<String> {
        for shard in &self.shards {
            if shard.user_exists(username)? {
                return shard.export_user_data(username);
            }
        }
        anyhow::bail!("Usuario '{}' no encontrado", username)
    }

    fn clear_image_from_posts(&self, path: &str) -> Result<u64> {
        let mut total = 0u64;
        for shard in &self.shards {
            total += shard.clear_image_from_posts(path)?;
        }
        Ok(total)
    }

    fn get_public_key(&self, user_id: i64) -> Result<Option<String>> {
        self.shard_for_id(user_id).get_public_key(user_id)
    }

    fn create_invitation(&self, valid_days: i64) -> Result<String> {
        // Se crea en el shard 0; el registro en modo invite busca en su shard
        // primero y luego en el 0 (ver `register_user`).
        self.shard(0).create_invitation(valid_days)
    }

    fn list_invitations(&self) -> Result<Vec<(i64, String, String, bool, Option<String>)>> {
        let mut all = self.fan_out(|s| s.list_invitations())?;
        all.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(all)
    }

    fn revoke_invitation(&self, code: &str) -> Result<bool> {
        for shard in &self.shards {
            if shard.revoke_invitation(code)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn seed_data(&self) -> Result<()> {
        // Si hay un solo shard, usa el seed normal.
        if self.shards.len() == 1 {
            return self.shard(0).seed_data();
        }
        // Con varios shards, se siembra cada shard con el mismo esquema pero
        // solo los usuarios rutean a su shard (el seed local inserta en el
        // shard 0 y luego re-mapea). Para no romper la coherencia, en modo
        // mesh el seed se limita a los usuarios base en el shard 0.
        self.shard(0).seed_data()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    fn tmp_dir(tag: &str) -> String {
        let dir = std::env::temp_dir().join(format!("agora_shard_{}", tag));
        std::fs::create_dir_all(&dir).ok();
        dir.to_str().unwrap().to_string()
    }

    fn cleanup(tag: &str) {
        let dir = tmp_dir(tag);
        for entry in std::fs::read_dir(&dir).unwrap() {
            let p = entry.unwrap().path();
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn test_shard_routing_by_id() {
        assert_eq!(shard_for_user(1), 0);
        assert_eq!(shard_for_user(10_000_001), 1);
        assert_eq!(shard_for_user(30_000_007), 3);
    }

    #[test]
    fn test_shard_cross_user_flow() {
        let _g = LOCK.lock().unwrap();
        let tag = "flow";
        cleanup(tag);
        let base = format!("{}/agora.db", tmp_dir(tag));

        // 2 shards
        let sm = ShardManager::new(&base, 2).unwrap();
        assert_eq!(sm.shard_count(), 2);

        // Registra varios usuarios; deberían repartirse entre shards.
        let u1 = sm.register_user("alice", "pass1234", "Alice", None).unwrap();
        let u2 = sm.register_user("bob", "pass1234", "Bob", None).unwrap();
        let u3 = sm.register_user("carol", "pass1234", "Carol", None).unwrap();
        let u4 = sm.register_user("dave", "pass1234", "Dave", None).unwrap();

        let shards = vec![
            shard_for_user(u1.id),
            shard_for_user(u2.id),
            shard_for_user(u3.id),
            shard_for_user(u4.id),
        ];
        assert!(shards.contains(&0) && shards.contains(&1), "debería repartirse: {:?}", shards);

        // Post de alice.
        let post = sm.create_post(u1.id, "Hola #rust", None).unwrap();
        assert!(post.id > 0);

        // bob sigue a alice (cross-shard probable).
        sm.follow_user(u2.id, u1.id).unwrap();
        assert!(sm.is_following(u2.id, u1.id).unwrap());

        // Timeline de bob debería incluir el post de alice.
        let tl = sm.get_timeline(u2.id, 0, 50).unwrap();
        assert!(
            tl.iter().any(|p| p.id == post.id),
            "timeline de bob debería tener el post de alice: {:?}",
            tl.iter().map(|p| p.content.clone()).collect::<Vec<_>>()
        );

        // Mensaje cross-shard entre dave y carol.
        sm.send_message(u3.id, u4.id, "hola dave", false).unwrap();
        let msgs = sm.get_messages(u3.id, u4.id).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "hola dave");

        // Notificación cross-shard.
        sm.add_notification(u4.id, u3.id, "follow", None).unwrap();
        let notifs = sm.get_notifications(u4.id, 0, 10).unwrap();
        assert_eq!(notifs.len(), 1);
        assert_eq!(notifs[0].from_user_id, u3.id);

        // Búsqueda global.
        let found = sm.search_users("ali", 0, 10).unwrap();
        assert!(found.iter().any(|u| u.username == "alice"));

        // Auth en el shard correcto.
        match sm.authenticate("dave", "pass1234").unwrap() {
            AuthResult::Success(u) => assert_eq!(u.id, u4.id),
            _ => panic!("dave debería autenticarse"),
        }

        // Export del usuario en shard no-0.
        let fname = sm.export_user_data("dave").unwrap();
        assert!(fname.contains("dave"));

        cleanup(tag);
    }
}

#[cfg(test)]
mod condense_tests {
    use super::*;

    fn tmp_base(tag: &str) -> String {
        let dir = std::env::temp_dir().join(format!("agora_cond_{}", tag));
        std::fs::create_dir_all(&dir).ok();
        format!("{}/agora.db", dir.to_str().unwrap())
    }

    fn wipe(base: &str) {
        let dir = std::path::Path::new(base).parent().unwrap();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    #[test]
    fn test_condense_merges_all_shards() {
        let base = tmp_base("merge");
        wipe(&base);

        // Crea usuarios hasta tener al menos uno en cada uno de 2 shards.
        let sm = ShardManager::new(&base, 2).unwrap();
        let mut ids = Vec::new();
        for name in ["u1", "u2", "u3", "u4", "u5", "u6", "u7", "u8"] {
            let u = sm.register_user(name, "pass1234", name, None).unwrap();
            ids.push(u.id);
        }
        // Al menos uno en cada shard.
        let shards: Vec<usize> = ids.iter().map(|id| shard_for_user(*id)).collect();
        assert!(shards.contains(&0) && shards.contains(&1), "ids: {:?}", ids);

        // Posts en distintos shards.
        let mut post_ids = Vec::new();
        for id in &ids {
            let p = sm.create_post(*id, &format!("post de {}", id), None).unwrap();
            post_ids.push(p.id);
        }
        drop(sm);

        // Condensa.
        let sm = ShardManager::new(&base, 2).unwrap();
        sm.condense().unwrap();
        drop(sm);

        // Verifica que los shards secundarios desaparecieron.
        assert!(!std::path::Path::new(&format!("{}-1.db", &base[..base.len() - 3])).exists());

        // Abre como un único archivo y comprueba que todo está.
        let db = crate::db::Database::new(&base).unwrap();
        for id in &ids {
            assert!(db.get_user_by_id(*id).unwrap().is_some(), "falta usuario {}", id);
        }
        for pid in &post_ids {
            assert!(db.get_post_by_id(*pid).unwrap().is_some(), "falta post {}", pid);
        }

        wipe(&base);
    }
}

#[cfg(test)]
mod seed_sync_tests {
    use super::*;

    #[test]
    fn test_seed_then_new_ops_no_id_collision() {
        let dir = std::env::temp_dir().join("agora_seed_sync");
        std::fs::create_dir_all(&dir).ok();
        let base = format!("{}/agora.db", dir.to_str().unwrap());
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let _ = std::fs::remove_file(entry.path());
        }

        // Abre un solo Database (sin mesh) y siembra.
        let db = crate::db::Database::new(&base).unwrap();
        db.seed_data().unwrap();

        // Obtiene dos usuarios del seed.
        let alice = db.get_user_by_id(1).unwrap().unwrap();
        let bob = db.get_user_by_id(2).unwrap().unwrap();

        // Nuevas operaciones: deben obtener ids SIN colisionar con el seed.
        let post = db.create_post(alice.id, "post nuevo", None).unwrap();
        assert!(post.id > 250, "post id {} deberia ser > 250 (max del seed)", post.id);

        let msg = db
            .send_message(alice.id, bob.id, "hola bob", false)
            .unwrap();
        assert!(msg.id > 250, "msg id {} deberia ser > 250", msg.id);

        let comment = db
            .add_comment(post.id, bob.id, "buen post", None)
            .unwrap();
        assert!(comment.id > 250, "comment id {} deberia ser > 250", comment.id);

        crate::db::Database::add_notification(&db, bob.id, alice.id, "mention", Some(post.id)).unwrap();

        // Verifica que todos los ids son únicos en sus tablas.
        assert_eq!(db.get_post_by_id(post.id).unwrap().unwrap().id, post.id);
        let msgs = db.get_messages(alice.id, bob.id).unwrap();
        assert!(msgs.iter().any(|m| m.id == msg.id));
    }
}
