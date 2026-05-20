use anyhow::Result;
use chrono::{DateTime, Utc};
use postgres::NoTls;
use r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use crate::models::{Comment, Message, Notification, Post, User};

pub enum AuthResult {
    Success(User),
    UserNotFound,
    WrongPassword,
}

#[allow(dead_code)]
pub trait DatabaseOps: Send {
    fn register_user(&self, username: &str, password: &str, display_name: &str) -> Result<User>;
    fn check_register_rate_limit(&self) -> Result<()>;
    fn authenticate(&self, username: &str, password: &str) -> Result<AuthResult>;
    fn get_user_by_id(&self, id: i64) -> Result<Option<User>>;
    fn search_users(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<User>>;
    fn create_post(&self, user_id: i64, content: &str, image_path: Option<&str>) -> Result<Post>;
    fn get_timeline(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>>;
    fn follow_user(&self, follower_id: i64, following_id: i64) -> Result<()>;
    fn unfollow_user(&self, follower_id: i64, following_id: i64) -> Result<()>;
    fn is_following(&self, follower_id: i64, following_id: i64) -> Result<bool>;
    fn get_followers(&self, user_id: i64) -> Result<Vec<User>>;
    fn get_following(&self, user_id: i64) -> Result<Vec<User>>;
    fn get_posts_by_user(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>>;
    fn get_post_by_id(&self, post_id: i64) -> Result<Option<Post>>;
    fn add_comment(&self, post_id: i64, user_id: i64, content: &str, parent_id: Option<i64>) -> Result<Comment>;
    fn get_comments(&self, post_id: i64) -> Result<Vec<Comment>>;
    fn update_post(&self, post_id: i64, user_id: i64, content: &str) -> Result<()>;
    fn delete_post(&self, post_id: i64, user_id: i64) -> Result<()>;
    fn delete_comment(&self, comment_id: i64, user_id: i64) -> Result<()>;
    fn delete_user(&self, user_id: i64) -> Result<()>;
    fn send_message(&self, sender_id: i64, receiver_id: i64, content: &str) -> Result<Message>;
    fn get_conversations(&self, user_id: i64) -> Result<Vec<User>>;
    fn get_messages(&self, user_id: i64, other_id: i64) -> Result<Vec<Message>>;
    fn get_unread_count(&self, user_id: i64) -> Result<i64>;
    fn mark_messages_read(&self, user_id: i64, other_id: i64) -> Result<()>;
    fn update_profile(&self, user_id: i64, display_name: &str, bio: &str, utc_offset: i32) -> Result<()>;
    fn update_timezone(&self, user_id: i64, utc_offset: i32) -> Result<()>;
    fn add_notification(&self, user_id: i64, from_user_id: i64, notif_type: &str, related_id: Option<i64>) -> Result<()>;
    fn get_notifications(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Notification>>;
    fn get_unread_notifications_count(&self, user_id: i64) -> Result<i64>;
    fn mark_notifications_read(&self, user_id: i64) -> Result<()>;
    fn search_posts(&self, query: &str, time_filter: &str, offset: u64, limit: u64) -> Result<Vec<Post>>;
    fn search_posts_by_user(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<Post>>;
    fn search_posts_by_date(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<Post>>;
    fn check_rate_limit(&self, user_id: i64, action: &str, max: usize, window_secs: u64) -> Result<()>;
    fn cleanup_old_data(&self, days: i64) -> Result<(u64, u64)>;
    fn cleanup_inactive_users(&self, days: i64) -> Result<u64>;
    fn get_posts_by_hashtag(&self, tag: &str, offset: u64, limit: u64) -> Result<Vec<Post>>;
    fn get_trending_hashtags(&self, limit: u64) -> Result<Vec<(String, i64)>>;
    fn export_user_data(&self, username: &str) -> Result<String>;
    fn clear_image_from_posts(&self, path: &str) -> Result<u64>;
}

impl DatabaseOps for Database {
    fn check_register_rate_limit(&self) -> Result<()> {
        Database::check_register_rate_limit(self)
    }
    fn register_user(&self, username: &str, password: &str, display_name: &str) -> Result<User> {
        Database::register_user(self, username, password, display_name)
    }
    fn authenticate(&self, username: &str, password: &str) -> Result<AuthResult> {
        Database::authenticate(self, username, password)
    }
    fn get_user_by_id(&self, id: i64) -> Result<Option<User>> {
        Database::get_user_by_id(self, id)
    }
    fn search_users(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<User>> {
        Database::search_users(self, query, offset, limit)
    }
    fn create_post(&self, user_id: i64, content: &str, image_path: Option<&str>) -> Result<Post> {
        Database::create_post(self, user_id, content, image_path)
    }
    fn get_timeline(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>> {
        Database::get_timeline(self, user_id, offset, limit)
    }
    fn follow_user(&self, follower_id: i64, following_id: i64) -> Result<()> {
        Database::follow_user(self, follower_id, following_id)
    }
    fn unfollow_user(&self, follower_id: i64, following_id: i64) -> Result<()> {
        Database::unfollow_user(self, follower_id, following_id)
    }
    fn is_following(&self, follower_id: i64, following_id: i64) -> Result<bool> {
        Database::is_following(self, follower_id, following_id)
    }
    fn get_followers(&self, user_id: i64) -> Result<Vec<User>> {
        Database::get_followers(self, user_id)
    }
    fn get_following(&self, user_id: i64) -> Result<Vec<User>> {
        Database::get_following(self, user_id)
    }
    fn get_posts_by_user(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>> {
        Database::get_posts_by_user(self, user_id, offset, limit)
    }
    fn get_post_by_id(&self, post_id: i64) -> Result<Option<Post>> {
        Database::get_post_by_id(self, post_id)
    }
    fn add_comment(&self, post_id: i64, user_id: i64, content: &str, parent_id: Option<i64>) -> Result<Comment> {
        Database::add_comment(self, post_id, user_id, content, parent_id)
    }
    fn get_comments(&self, post_id: i64) -> Result<Vec<Comment>> {
        Database::get_comments(self, post_id)
    }
    fn update_post(&self, post_id: i64, user_id: i64, content: &str) -> Result<()> {
        Database::update_post(self, post_id, user_id, content)
    }
    fn delete_post(&self, post_id: i64, user_id: i64) -> Result<()> {
        Database::delete_post(self, post_id, user_id)
    }
    fn delete_comment(&self, comment_id: i64, user_id: i64) -> Result<()> {
        Database::delete_comment(self, comment_id, user_id)
    }
    fn delete_user(&self, user_id: i64) -> Result<()> {
        Database::delete_user(self, user_id)
    }
    fn send_message(&self, sender_id: i64, receiver_id: i64, content: &str) -> Result<Message> {
        Database::send_message(self, sender_id, receiver_id, content)
    }
    fn get_conversations(&self, user_id: i64) -> Result<Vec<User>> {
        Database::get_conversations(self, user_id)
    }
    fn get_messages(&self, user_id: i64, other_id: i64) -> Result<Vec<Message>> {
        Database::get_messages(self, user_id, other_id)
    }
    fn get_unread_count(&self, user_id: i64) -> Result<i64> {
        Database::get_unread_count(self, user_id)
    }
    fn mark_messages_read(&self, user_id: i64, other_id: i64) -> Result<()> {
        Database::mark_messages_read(self, user_id, other_id)
    }
    fn update_profile(&self, user_id: i64, display_name: &str, bio: &str, utc_offset: i32) -> Result<()> {
        Database::update_profile(self, user_id, display_name, bio, utc_offset)
    }
    fn update_timezone(&self, user_id: i64, utc_offset: i32) -> Result<()> {
        Database::update_timezone(self, user_id, utc_offset)
    }
    fn add_notification(&self, user_id: i64, from_user_id: i64, notif_type: &str, related_id: Option<i64>) -> Result<()> {
        Database::add_notification(self, user_id, from_user_id, notif_type, related_id)
    }
    fn get_notifications(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Notification>> {
        Database::get_notifications(self, user_id, offset, limit)
    }
    fn get_unread_notifications_count(&self, user_id: i64) -> Result<i64> {
        Database::get_unread_notifications_count(self, user_id)
    }
    fn mark_notifications_read(&self, user_id: i64) -> Result<()> {
        Database::mark_notifications_read(self, user_id)
    }
    fn search_posts(&self, query: &str, time_filter: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        Database::search_posts(self, query, time_filter, offset, limit)
    }
    fn search_posts_by_user(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        Database::search_posts_by_user(self, query, offset, limit)
    }
    fn search_posts_by_date(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        Database::search_posts_by_date(self, query, offset, limit)
    }
    fn check_rate_limit(&self, user_id: i64, action: &str, max: usize, window_secs: u64) -> Result<()> {
        Database::check_rate_limit(self, user_id, action, max, window_secs)
    }
    fn cleanup_old_data(&self, days: i64) -> Result<(u64, u64)> {
        Database::cleanup_old_data(self, days)
    }
    fn cleanup_inactive_users(&self, days: i64) -> Result<u64> {
        Database::cleanup_inactive_users(self, days)
    }
    fn get_posts_by_hashtag(&self, tag: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        Database::get_posts_by_hashtag(self, tag, offset, limit)
    }
    fn get_trending_hashtags(&self, limit: u64) -> Result<Vec<(String, i64)>> {
        Database::get_trending_hashtags(self, limit)
    }
    fn export_user_data(&self, username: &str) -> Result<String> {
        Database::export_user_data(self, username)
    }
    fn clear_image_from_posts(&self, path: &str) -> Result<u64> {
        Database::clear_image_from_posts(self, path)
    }
}

pub struct Database {
    pool: Pool<PostgresConnectionManager<NoTls>>,
    rate_limiter: Mutex<HashMap<String, Vec<Instant>>>,
}

impl Database {
    pub fn new(conn_str: &str) -> Result<Self> {
        let manager = PostgresConnectionManager::new(conn_str.parse()?, NoTls);
        let pool = Pool::builder()
            .max_size(25)
            .min_idle(Some(0))
            .connection_timeout(Duration::from_secs(3))
            .build(manager)?;
        let db = Self { pool, rate_limiter: Mutex::new(HashMap::new()) };
        let mut last_err = anyhow::anyhow!("could not connect to database");
        for i in 0..12 {
            match db.init_schema() {
                Ok(()) => return Ok(db),
                Err(e) => {
                    last_err = e;
                    eprintln!("[agora] DB connection attempt {} failed, retrying in {}s...", i + 1, i + 1);
                    std::thread::sleep(std::time::Duration::from_secs(i as u64 + 1));
                }
            }
        }
        Err(last_err)
    }

    pub fn check_register_rate_limit(&self) -> Result<()> {
        let key = "register:global".to_string();
        let now = Instant::now();
        let mut limiter = self.rate_limiter.lock().unwrap();
        let entries = limiter.entry(key).or_default();
        entries.retain(|t| now.duration_since(*t).as_secs() < 60);
        if entries.len() >= 3 {
            anyhow::bail!("Demasiados registros. Espera un minuto.");
        }
        entries.push(now);
        Ok(())
    }

    pub fn check_rate_limit(&self, user_id: i64, action: &str, max: usize, window_secs: u64) -> Result<()> {
        let mut conn = self.pool.get()?;
        let now = Utc::now();
        let window_start = now - chrono::Duration::seconds(window_secs as i64);
        let window_start_str = window_start.to_rfc3339();
        let now_str = now.to_rfc3339();

        let rows = conn.query(
            "SELECT count, banned_until FROM rate_limits WHERE user_id = $1 AND action = $2 AND window_start > $3 ORDER BY window_start DESC LIMIT 1",
            &[&user_id, &action, &window_start_str],
        )?;

        if let Some(row) = rows.into_iter().next() {
            let count: i32 = row.get(0);
            let banned_until: String = row.get(1);

            if !banned_until.is_empty() {
                if let Ok(ban_time) = banned_until.parse::<DateTime<Utc>>() {
                    if now < ban_time {
                        let remaining = ban_time - now;
                        anyhow::bail!(
                            "Cuenta baneada temporalmente. Espera {} segundos.",
                            remaining.num_seconds()
                        );
                    }
                }
            }

            if count as usize >= max {
                let ban_duration = if count as usize >= max * 3 {
                    window_secs * 4
                } else if count as usize >= max * 2 {
                    window_secs * 2
                } else {
                    window_secs
                };
                let ban_until = now + chrono::Duration::seconds(ban_duration as i64);
                let ban_until_str = ban_until.to_rfc3339();

                conn.execute(
                    "UPDATE rate_limits SET banned_until = $1 WHERE user_id = $2 AND action = $3 AND window_start > $4",
                    &[&ban_until_str, &user_id, &action, &window_start_str],
                )?;

                anyhow::bail!(
                    "Demasiadas solicitudes. Espera {} segundos.",
                    ban_duration
                );
            }

            conn.execute(
                "UPDATE rate_limits SET count = count + 1 WHERE user_id = $1 AND action = $2 AND window_start > $3",
                &[&user_id, &action, &window_start_str],
            )?;
        } else {
            conn.execute(
                "INSERT INTO rate_limits (user_id, action, window_start, count) VALUES ($1, $2, $3, 1)",
                &[&user_id, &action, &now_str],
            )?;
        }

        Ok(())
    }

    fn init_schema(&self) -> Result<()> {
        let mut conn = self.pool.get()?;
        conn.batch_execute(
            "SET client_min_messages TO warning;
            CREATE TABLE IF NOT EXISTS users (
                id BIGSERIAL PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                display_name TEXT NOT NULL DEFAULT '',
                bio TEXT NOT NULL DEFAULT '',
                utc_offset INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                last_login_at TEXT NOT NULL DEFAULT '',
                login_count INTEGER NOT NULL DEFAULT 0
            );
            ALTER TABLE users ADD COLUMN IF NOT EXISTS last_login_at TEXT NOT NULL DEFAULT '';
            ALTER TABLE users ADD COLUMN IF NOT EXISTS login_count INTEGER NOT NULL DEFAULT 0;
            CREATE TABLE IF NOT EXISTS posts (
                id BIGSERIAL PRIMARY KEY,
                user_id BIGINT NOT NULL REFERENCES users(id),
                content TEXT NOT NULL,
                image_path TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS follows (
                follower_id BIGINT NOT NULL REFERENCES users(id),
                following_id BIGINT NOT NULL REFERENCES users(id),
                PRIMARY KEY (follower_id, following_id)
            );
            CREATE TABLE IF NOT EXISTS comments (
                id BIGSERIAL PRIMARY KEY,
                post_id BIGINT NOT NULL REFERENCES posts(id),
                user_id BIGINT NOT NULL REFERENCES users(id),
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS messages (
                id BIGSERIAL PRIMARY KEY,
                sender_id BIGINT NOT NULL REFERENCES users(id),
                receiver_id BIGINT NOT NULL REFERENCES users(id),
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                read INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS notifications (
                id BIGSERIAL PRIMARY KEY,
                user_id BIGINT NOT NULL REFERENCES users(id),
                from_user_id BIGINT NOT NULL REFERENCES users(id),
                type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                read INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS rate_limits (
                user_id BIGINT NOT NULL,
                action TEXT NOT NULL,
                window_start TEXT NOT NULL,
                count INTEGER NOT NULL DEFAULT 1,
                banned_until TEXT NOT NULL DEFAULT '',
                PRIMARY KEY (user_id, action, window_start)
            );
            CREATE TABLE IF NOT EXISTS post_hashtags (
                post_id BIGINT NOT NULL REFERENCES posts(id),
                tag TEXT NOT NULL,
                PRIMARY KEY (post_id, tag)
            );
            CREATE INDEX IF NOT EXISTS idx_post_hashtags_tag ON post_hashtags(tag);",
        )?;
        // Migrate existing SERIAL/INTEGER columns to BIGINT if they exist
        conn.batch_execute(
            "ALTER TABLE users ALTER COLUMN id TYPE BIGINT;
             ALTER TABLE posts ALTER COLUMN id TYPE BIGINT;
             ALTER TABLE posts ALTER COLUMN user_id TYPE BIGINT;
             ALTER TABLE follows ALTER COLUMN follower_id TYPE BIGINT;
             ALTER TABLE follows ALTER COLUMN following_id TYPE BIGINT;
             ALTER TABLE comments ALTER COLUMN id TYPE BIGINT;
             ALTER TABLE comments ALTER COLUMN post_id TYPE BIGINT;
             ALTER TABLE comments ALTER COLUMN user_id TYPE BIGINT;
             ALTER TABLE messages ALTER COLUMN id TYPE BIGINT;
             ALTER TABLE messages ALTER COLUMN sender_id TYPE BIGINT;
             ALTER TABLE messages ALTER COLUMN receiver_id TYPE BIGINT;
             ALTER TABLE notifications ALTER COLUMN id TYPE BIGINT;
             ALTER TABLE notifications ALTER COLUMN user_id TYPE BIGINT;
             ALTER TABLE notifications ALTER COLUMN from_user_id TYPE BIGINT;
             ALTER TABLE notifications ADD COLUMN IF NOT EXISTS related_id BIGINT DEFAULT NULL;
             ALTER TABLE comments ADD COLUMN IF NOT EXISTS parent_comment_id BIGINT DEFAULT NULL;"
        ).ok();
        Ok(())
    }

    pub fn register_user(&self, username: &str, password: &str, display_name: &str) -> Result<User> {
        let mut conn = self.pool.get()?;
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        let now = Utc::now().to_rfc3339();
        let username_lower = username.trim().to_lowercase();
        let rows = conn.query(
            "INSERT INTO users (username, password_hash, display_name, utc_offset, created_at) VALUES ($1, $2, $3, 0, $4) RETURNING id",
            &[&username_lower, &hash, &display_name, &now],
        )?;
        let id: i64 = rows[0].get(0);
        Ok(User {
            id,
            username: username_lower,
            display_name: display_name.to_string(),
            bio: String::new(),
            utc_offset: 0,
            created_at: now.parse().unwrap(),
        })
    }

    pub fn authenticate(&self, username: &str, password: &str) -> Result<AuthResult> {
        let conn = self.pool.get().map_err(|e| anyhow::anyhow!("pool.get failed: {e}"))?;
        let mut conn = conn;
        let rows = conn.query(
            "SELECT id, username, display_name, bio, utc_offset, created_at, password_hash FROM users WHERE LOWER(username) = LOWER($1)",
            &[&username],
        ).map_err(|e| anyhow::anyhow!("query failed: {e}"))?;
        if let Some(row) = rows.into_iter().next() {
            let hash: String = row.get(6);
            if bcrypt::verify(password, &hash).map_err(|e| anyhow::anyhow!("bcrypt failed: {e}"))? {
                let id: i64 = row.get(0);
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE users SET last_login_at = $1, login_count = login_count + 1 WHERE id = $2",
                    &[&now, &id],
                ).map_err(|e| anyhow::anyhow!("update failed: {e}"))?;
                return Ok(AuthResult::Success(User {
                    id: row.get(0),
                    username: row.get(1),
                    display_name: row.get(2),
                    bio: row.get(3),
                    utc_offset: row.get(4),
                    created_at: row.get::<_, String>(5).parse().unwrap(),
                }));
            } else {
                return Ok(AuthResult::WrongPassword);
            }
        }
        Ok(AuthResult::UserNotFound)
    }

    pub fn get_user_by_id(&self, id: i64) -> Result<Option<User>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT id, username, display_name, bio, utc_offset, created_at FROM users WHERE id = $1",
            &[&id],
        )?;
        Ok(rows.into_iter().next().map(|row| User {
            id: row.get(0),
            username: row.get(1),
            display_name: row.get(2),
            bio: row.get(3),
            utc_offset: row.get(4),
            created_at: row.get::<_, String>(5).parse().unwrap(),
        }))
    }

    pub fn search_users(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<User>> {
        let mut conn = self.pool.get()?;
        let pattern = format!("%{}%", query);
        let rows = conn.query(
            "SELECT id, username, display_name, bio, utc_offset, created_at FROM users WHERE username ILIKE $1 OR display_name ILIKE $1 ORDER BY username LIMIT $2 OFFSET $3",
            &[&pattern, &(limit as i64), &(offset as i64)],
        )?;
        Ok(rows.iter().map(|row| User {
            id: row.get(0),
            username: row.get(1),
            display_name: row.get(2),
            bio: row.get(3),
            utc_offset: row.get(4),
            created_at: row.get::<_, String>(5).parse().unwrap(),
        }).collect())
    }

    pub fn create_post(&self, user_id: i64, content: &str, image_path: Option<&str>) -> Result<Post> {
        if content.len() > 5000 {
            anyhow::bail!("El post es demasiado largo (máximo 5000 caracteres)");
        }
        if content.trim().is_empty() {
            anyhow::bail!("El post no puede estar vacío");
        }
        self.check_rate_limit(user_id, "post", 5, 60)?;
        let mut conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();
        let img = image_path.unwrap_or("");
        let rows = conn.query(
            "INSERT INTO posts (user_id, content, image_path, created_at) VALUES ($1, $2, $3, $4) RETURNING id",
            &[&user_id, &content, &img, &now],
        )?;
        let id: i64 = rows[0].get(0);
        let username: String = conn.query_one(
            "SELECT username FROM users WHERE id = $1",
            &[&user_id],
        )?.get(0);

        let hashtags = Self::extract_hashtags(content);
        for tag in &hashtags {
            conn.execute(
                "INSERT INTO post_hashtags (post_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&id, &tag],
            )?;
        }

        let mentioned = Self::extract_mentions(content);
        for mentioned_username in &mentioned {
            let rows = conn.query(
                "SELECT id FROM users WHERE LOWER(username) = LOWER($1)",
                &[mentioned_username],
            )?;
            if let Some(row) = rows.into_iter().next() {
                let mentioned_id: i64 = row.get(0);
                if mentioned_id != user_id {
                    conn.execute(
                        "INSERT INTO notifications (user_id, from_user_id, type, created_at, related_id) VALUES ($1, $2, 'mention', $3, $4)",
                        &[&mentioned_id, &user_id, &now, &id],
                    ).ok();
                }
            }
        }

        Ok(Post {
            id,
            user_id,
            username,
            content: content.to_string(),
            image_path: image_path.map(|s| s.to_string()).filter(|s| !s.is_empty()),
            created_at: now.parse().unwrap(),
        })
    }

    pub fn get_posts_by_hashtag(&self, tag: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        let mut conn = self.pool.get()?;
        let tag_lower = tag.to_lowercase().trim_start_matches('#').to_string();
        let rows = conn.query(
            "SELECT p.id, p.user_id, u.username, p.content, p.image_path, p.created_at
             FROM posts p
             JOIN users u ON u.id = p.user_id
             JOIN post_hashtags ph ON ph.post_id = p.id
             WHERE LOWER(ph.tag) = $1
             ORDER BY p.created_at DESC
             LIMIT $2 OFFSET $3",
            &[&tag_lower, &(limit as i64), &(offset as i64)],
        )?;
        Ok(rows.iter().map(|row| {
            let img: String = row.get(4);
            Post {
                id: row.get(0),
                user_id: row.get(1),
                username: row.get(2),
                content: row.get(3),
                image_path: if img.is_empty() { None } else { Some(img) },
                created_at: row.get::<_, String>(5).parse().unwrap(),
            }
        }).collect())
    }

    pub fn get_trending_hashtags(&self, limit: u64) -> Result<Vec<(String, i64)>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT tag, COUNT(*) as cnt FROM post_hashtags
             GROUP BY tag
             ORDER BY cnt DESC
             LIMIT $1",
            &[&(limit as i64)],
        )?;
        Ok(rows.iter().map(|row| {
            (row.get(0), row.get(1))
        }).collect())
    }

    fn extract_hashtags(content: &str) -> Vec<String> {
        let mut tags = Vec::new();
        let mut in_hashtag = false;
        let mut current = String::new();

        for ch in content.chars() {
            if ch == '#' && !in_hashtag {
                in_hashtag = true;
                current.clear();
            } else if in_hashtag {
                if ch.is_alphanumeric() || ch == '_' {
                    current.push(ch);
                } else {
                    if !current.is_empty() {
                        tags.push(current.to_lowercase());
                    }
                    in_hashtag = false;
                    current.clear();
                }
            }
        }
        if in_hashtag && !current.is_empty() {
            tags.push(current.to_lowercase());
        }

        tags.sort();
        tags.dedup();
        tags
    }

    fn extract_mentions(content: &str) -> Vec<String> {
        let mut mentions = Vec::new();
        let mut in_mention = false;
        let mut current = String::new();

        for ch in content.chars() {
            if ch == '@' && !in_mention {
                in_mention = true;
                current.clear();
            } else if in_mention {
                if ch.is_alphanumeric() || ch == '_' {
                    current.push(ch);
                } else {
                    if !current.is_empty() {
                        mentions.push(current.to_lowercase());
                    }
                    in_mention = false;
                    current.clear();
                }
            }
        }
        if in_mention && !current.is_empty() {
            mentions.push(current.to_lowercase());
        }

        mentions.sort();
        mentions.dedup();
        mentions
    }

    pub fn get_timeline(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT p.id, p.user_id, u.username, p.content, p.image_path, p.created_at
             FROM posts p
             JOIN users u ON u.id = p.user_id
             LEFT JOIN follows f ON f.following_id = p.user_id AND f.follower_id = $1
             WHERE p.user_id = $1 OR f.follower_id = $1
             ORDER BY p.created_at DESC
             LIMIT $2 OFFSET $3",
            &[&user_id, &(limit as i64), &(offset as i64)],
        )?;
        Ok(rows.iter().map(|row| {
            let img: String = row.get(4);
            Post {
                id: row.get(0),
                user_id: row.get(1),
                username: row.get(2),
                content: row.get(3),
                image_path: if img.is_empty() { None } else { Some(img) },
                created_at: row.get::<_, String>(5).parse().unwrap(),
            }
        }).collect())
    }

    pub fn follow_user(&self, follower_id: i64, following_id: i64) -> Result<()> {
        self.check_rate_limit(follower_id, "follow", 10, 60)?;
        let mut conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO follows (follower_id, following_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            &[&follower_id, &following_id],
        )?;
        Ok(())
    }

    pub fn unfollow_user(&self, follower_id: i64, following_id: i64) -> Result<()> {
        let mut conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM follows WHERE follower_id = $1 AND following_id = $2",
            &[&follower_id, &following_id],
        )?;
        Ok(())
    }

    pub fn is_following(&self, follower_id: i64, following_id: i64) -> Result<bool> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT 1 FROM follows WHERE follower_id = $1 AND following_id = $2",
            &[&follower_id, &following_id],
        )?;
        Ok(!rows.is_empty())
    }

    pub fn get_followers(&self, user_id: i64) -> Result<Vec<User>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT u.id, u.username, u.display_name, u.bio, u.utc_offset, u.created_at
             FROM users u
             JOIN follows f ON f.follower_id = u.id
             WHERE f.following_id = $1",
            &[&user_id],
        )?;
        Ok(rows.iter().map(|row| User {
            id: row.get(0),
            username: row.get(1),
            display_name: row.get(2),
            bio: row.get(3),
            utc_offset: row.get(4),
            created_at: row.get::<_, String>(5).parse().unwrap(),
        }).collect())
    }

    pub fn get_posts_by_user(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT p.id, p.user_id, u.username, p.content, p.image_path, p.created_at
             FROM posts p JOIN users u ON u.id = p.user_id
             WHERE p.user_id = $1
             ORDER BY p.created_at DESC LIMIT $2 OFFSET $3",
            &[&user_id, &(limit as i64), &(offset as i64)],
        )?;
        Ok(rows.iter().map(|row| {
            let img: String = row.get(4);
            Post {
                id: row.get(0),
                user_id: row.get(1),
                username: row.get(2),
                content: row.get(3),
                image_path: if img.is_empty() { None } else { Some(img) },
                created_at: row.get::<_, String>(5).parse().unwrap(),
            }
        }).collect())
    }

    pub fn search_posts(&self, query: &str, time_filter: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        let mut conn = self.pool.get()?;
        let interval = match time_filter {
            "24h" => Some("24 hours"),
            "7d" => Some("7 days"),
            "30d" => Some("30 days"),
            _ => None,
        };
        let has_interval = interval.is_some();
        let sql = if has_interval {
            format!(
                "SELECT p.id, p.user_id, u.username, p.content, p.image_path, p.created_at
                 FROM posts p JOIN users u ON u.id = p.user_id
                 WHERE p.content ILIKE $1 AND p.created_at::timestamptz > NOW() - $2::interval
                 ORDER BY p.created_at DESC LIMIT $3 OFFSET $4"
            )
        } else {
            "SELECT p.id, p.user_id, u.username, p.content, p.image_path, p.created_at
             FROM posts p JOIN users u ON u.id = p.user_id
             WHERE p.content ILIKE $1
             ORDER BY p.created_at DESC LIMIT $2 OFFSET $3".to_string()
        };
        let pattern = format!("%{}%", query);
        let rows = if let Some(iv) = interval {
            conn.query(&sql, &[&pattern, &iv, &(limit as i64), &(offset as i64)])?
        } else {
            conn.query(&sql, &[&pattern, &(limit as i64), &(offset as i64)])?
        };
        Ok(rows.iter().map(|row| {
            let img: String = row.get(4);
            Post {
                id: row.get(0),
                user_id: row.get(1),
                username: row.get(2),
                content: row.get(3),
                image_path: if img.is_empty() { None } else { Some(img) },
                created_at: row.get::<_, String>(5).parse().unwrap(),
            }
        }).collect())
    }

    pub fn search_posts_by_user(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        let mut conn = self.pool.get()?;
        let pattern = format!("%{}%", query);
        let rows = conn.query(
            "SELECT p.id, p.user_id, u.username, p.content, p.image_path, p.created_at
             FROM posts p JOIN users u ON u.id = p.user_id
             WHERE LOWER(u.username) LIKE LOWER($1)
             ORDER BY p.created_at DESC LIMIT $2 OFFSET $3",
            &[&pattern, &(limit as i64), &(offset as i64)],
        )?;
        Ok(rows.iter().map(|row| {
            let img: String = row.get(4);
            Post {
                id: row.get(0),
                user_id: row.get(1),
                username: row.get(2),
                content: row.get(3),
                image_path: if img.is_empty() { None } else { Some(img) },
                created_at: row.get::<_, String>(5).parse().unwrap(),
            }
        }).collect())
    }

    pub fn search_posts_by_date(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
        let mut conn = self.pool.get()?;
        let pattern = format!("%{}%", query);
        let rows = conn.query(
            "SELECT p.id, p.user_id, u.username, p.content, p.image_path, p.created_at
             FROM posts p JOIN users u ON u.id = p.user_id
             WHERE p.created_at ILIKE $1
             ORDER BY p.created_at DESC LIMIT $2 OFFSET $3",
            &[&pattern, &(limit as i64), &(offset as i64)],
        )?;
        Ok(rows.iter().map(|row| {
            let img: String = row.get(4);
            Post {
                id: row.get(0),
                user_id: row.get(1),
                username: row.get(2),
                content: row.get(3),
                image_path: if img.is_empty() { None } else { Some(img) },
                created_at: row.get::<_, String>(5).parse().unwrap(),
            }
        }).collect())
    }

    pub fn get_post_by_id(&self, post_id: i64) -> Result<Option<Post>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT p.id, p.user_id, u.username, p.content, p.image_path, p.created_at
             FROM posts p JOIN users u ON u.id = p.user_id WHERE p.id = $1",
            &[&post_id],
        )?;
        Ok(rows.iter().next().map(|row| Post {
            id: row.get(0),
            user_id: row.get(1),
            username: row.get(2),
            content: row.get(3),
            image_path: if let Some(img) = row.get::<_, Option<String>>(4) {
                if img.is_empty() { None } else { Some(img) }
            } else { None },
            created_at: row.get::<_, String>(5).parse().unwrap(),
        }))
    }

    pub fn add_comment(&self, post_id: i64, user_id: i64, content: &str, parent_id: Option<i64>) -> Result<Comment> {
        self.check_rate_limit(user_id, "comment", 10, 60)?;
        let mut conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();
        let rows = conn.query(
            "INSERT INTO comments (post_id, user_id, content, created_at, parent_comment_id) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            &[&post_id, &user_id, &content, &now, &parent_id],
        )?;
        let id: i64 = rows[0].get(0);
        let username: String = conn.query_one(
            "SELECT username FROM users WHERE id = $1",
            &[&user_id],
        )?.get(0);

        let mentioned = Self::extract_mentions(content);
        for mentioned_username in &mentioned {
            let rows = conn.query(
                "SELECT id FROM users WHERE LOWER(username) = LOWER($1)",
                &[mentioned_username],
            )?;
            if let Some(row) = rows.into_iter().next() {
                let mentioned_id: i64 = row.get(0);
                if mentioned_id != user_id {
                    conn.execute(
                        "INSERT INTO notifications (user_id, from_user_id, type, created_at, related_id) VALUES ($1, $2, 'mention', $3, $4)",
                        &[&mentioned_id, &user_id, &now, &post_id],
                    ).ok();
                }
            }
        }

        Ok(Comment {
            id,
            post_id,
            user_id,
            username,
            content: content.to_string(),
            created_at: now.parse().unwrap(),
            parent_comment_id: parent_id,
        })
    }

    pub fn get_comments(&self, post_id: i64) -> Result<Vec<Comment>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT c.id, c.post_id, c.user_id, u.username, c.content, c.created_at, c.parent_comment_id
             FROM comments c
             JOIN users u ON u.id = c.user_id
             WHERE c.post_id = $1
             ORDER BY c.created_at ASC",
            &[&post_id],
        )?;
        Ok(rows.iter().map(|row| Comment {
            id: row.get(0),
            post_id: row.get(1),
            user_id: row.get(2),
            username: row.get(3),
            content: row.get(4),
            created_at: row.get::<_, String>(5).parse().unwrap(),
            parent_comment_id: row.get(6),
        }).collect())
    }

    pub fn update_post(&self, post_id: i64, user_id: i64, content: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let rows = conn.execute(
            "UPDATE posts SET content = $1 WHERE id = $2 AND user_id = $3",
            &[&content, &post_id, &user_id],
        )?;
        if rows == 0 {
            anyhow::bail!("No tienes permiso para editar este post o no existe");
        }
        Ok(())
    }

    pub fn delete_post(&self, post_id: i64, user_id: i64) -> Result<()> {
        let mut conn = self.pool.get()?;
        conn.execute("DELETE FROM comments WHERE post_id = $1", &[&post_id])?;
        let rows = conn.execute(
            "DELETE FROM posts WHERE id = $1 AND user_id = $2",
            &[&post_id, &user_id],
        )?;
        if rows == 0 {
            anyhow::bail!("No tienes permiso para eliminar este post o no existe");
        }
        Ok(())
    }

    pub fn delete_comment(&self, comment_id: i64, user_id: i64) -> Result<()> {
        let mut conn = self.pool.get()?;
        let rows = conn.execute(
            "DELETE FROM comments WHERE id = $1 AND user_id = $2",
            &[&comment_id, &user_id],
        )?;
        if rows == 0 {
            anyhow::bail!("No tienes permiso para eliminar este comentario o no existe");
        }
        Ok(())
    }

    pub fn delete_user(&self, user_id: i64) -> Result<()> {
        let mut conn = self.pool.get()?;
        conn.execute("DELETE FROM follows WHERE follower_id = $1 OR following_id = $1", &[&user_id])?;
        conn.execute(
            "DELETE FROM comments WHERE post_id IN (SELECT id FROM posts WHERE user_id = $1)",
            &[&user_id],
        )?;
        conn.execute("DELETE FROM posts WHERE user_id = $1", &[&user_id])?;
        conn.execute("DELETE FROM messages WHERE sender_id = $1 OR receiver_id = $1", &[&user_id])?;
        conn.execute("DELETE FROM notifications WHERE user_id = $1 OR from_user_id = $1", &[&user_id])?;
        conn.execute("DELETE FROM users WHERE id = $1", &[&user_id])?;
        Ok(())
    }

    pub fn get_following(&self, user_id: i64) -> Result<Vec<User>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT u.id, u.username, u.display_name, u.bio, u.utc_offset, u.created_at
             FROM users u
             JOIN follows f ON f.following_id = u.id
             WHERE f.follower_id = $1",
            &[&user_id],
        )?;
        Ok(rows.iter().map(|row| User {
            id: row.get(0),
            username: row.get(1),
            display_name: row.get(2),
            bio: row.get(3),
            utc_offset: row.get(4),
            created_at: row.get::<_, String>(5).parse().unwrap(),
        }).collect())
    }

    pub fn send_message(&self, sender_id: i64, receiver_id: i64, content: &str) -> Result<Message> {
        self.check_rate_limit(sender_id, "message", 10, 60)?;
        let mut conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();
        let rows = conn.query(
            "INSERT INTO messages (sender_id, receiver_id, content, created_at) VALUES ($1, $2, $3, $4) RETURNING id",
            &[&sender_id, &receiver_id, &content, &now],
        )?;
        let id: i64 = rows[0].get(0);
        let username: String = conn.query_one(
            "SELECT username FROM users WHERE id = $1",
            &[&sender_id],
        )?.get(0);
        Ok(Message {
            id,
            sender_id,
            receiver_id,
            sender_username: username,
            content: content.to_string(),
            created_at: now.parse().unwrap(),
            read: false,
        })
    }

    pub fn get_conversations(&self, user_id: i64) -> Result<Vec<User>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT u.id, u.username, u.display_name, u.bio, u.utc_offset, u.created_at
             FROM users u
             WHERE u.id IN (
                 SELECT DISTINCT CASE WHEN sender_id = $1 THEN receiver_id ELSE sender_id END
                 FROM messages
                 WHERE sender_id = $1 OR receiver_id = $1
             )
             ORDER BY u.username",
            &[&user_id],
        )?;
        Ok(rows.iter().map(|row| User {
            id: row.get(0),
            username: row.get(1),
            display_name: row.get(2),
            bio: row.get(3),
            utc_offset: row.get(4),
            created_at: row.get::<_, String>(5).parse().unwrap(),
        }).collect())
    }

    pub fn get_messages(&self, user_id: i64, other_id: i64) -> Result<Vec<Message>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT m.id, m.sender_id, m.receiver_id, u.username, m.content, m.created_at, m.read
             FROM messages m
             JOIN users u ON u.id = m.sender_id
             WHERE (m.sender_id = $1 AND m.receiver_id = $2) OR (m.sender_id = $2 AND m.receiver_id = $1)
             ORDER BY m.created_at ASC",
            &[&user_id, &other_id],
        )?;
        Ok(rows.iter().map(|row| Message {
            id: row.get(0),
            sender_id: row.get(1),
            receiver_id: row.get(2),
            sender_username: row.get(3),
            content: row.get(4),
            created_at: row.get::<_, String>(5).parse().unwrap(),
            read: row.get::<_, i32>(6) != 0,
        }).collect())
    }

    pub fn get_unread_count(&self, user_id: i64) -> Result<i64> {
        let mut conn = self.pool.get()?;
        let count: i64 = conn.query_one(
            "SELECT COUNT(*) FROM messages WHERE receiver_id = $1 AND read = 0",
            &[&user_id],
        )?.get(0);
        Ok(count)
    }

    pub fn mark_messages_read(&self, user_id: i64, other_id: i64) -> Result<()> {
        let mut conn = self.pool.get()?;
        conn.execute(
            "UPDATE messages SET read = 1 WHERE sender_id = $2 AND receiver_id = $1 AND read = 0",
            &[&user_id, &other_id],
        )?;
        Ok(())
    }

    pub fn update_profile(&self, user_id: i64, display_name: &str, bio: &str, utc_offset: i32) -> Result<()> {
        let mut conn = self.pool.get()?;
        conn.execute(
            "UPDATE users SET display_name = $1, bio = $2, utc_offset = $3 WHERE id = $4",
            &[&display_name, &bio, &utc_offset, &user_id],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_timezone(&self, user_id: i64, utc_offset: i32) -> Result<()> {
        let mut conn = self.pool.get()?;
        conn.execute(
            "UPDATE users SET utc_offset = $1 WHERE id = $2",
            &[&utc_offset, &user_id],
        )?;
        Ok(())
    }

    pub fn add_notification(&self, user_id: i64, from_user_id: i64, notif_type: &str, related_id: Option<i64>) -> Result<()> {
        let mut conn = self.pool.get()?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO notifications (user_id, from_user_id, type, created_at, related_id) VALUES ($1, $2, $3, $4, $5)",
            &[&user_id, &from_user_id, &notif_type, &now, &related_id],
        )?;
        Ok(())
    }

    pub fn get_notifications(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Notification>> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT n.id, n.user_id, n.from_user_id, u.username, n.type, n.created_at, n.read, n.related_id
             FROM notifications n
             JOIN users u ON u.id = n.from_user_id
             WHERE n.user_id = $1
             ORDER BY n.created_at DESC
             LIMIT $2 OFFSET $3",
            &[&user_id, &(limit as i64), &(offset as i64)],
        )?;
        Ok(rows.iter().map(|row| Notification {
            id: row.get(0),
            user_id: row.get(1),
            from_user_id: row.get(2),
            from_username: row.get(3),
            notif_type: row.get(4),
            created_at: row.get::<_, String>(5).parse().unwrap(),
            read: row.get::<_, i32>(6) != 0,
            related_id: row.get(7),
        }).collect())
    }

    pub fn get_unread_notifications_count(&self, user_id: i64) -> Result<i64> {
        let mut conn = self.pool.get()?;
        let count: i64 = conn.query_one(
            "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read = 0",
            &[&user_id],
        )?.get(0);
        Ok(count)
    }

    pub fn mark_notifications_read(&self, user_id: i64) -> Result<()> {
        let mut conn = self.pool.get()?;
        conn.execute(
            "UPDATE notifications SET read = 1 WHERE user_id = $1 AND read = 0",
            &[&user_id],
        )?;
        Ok(())
    }

    pub fn cleanup_old_data(&self, days: i64) -> Result<(u64, u64)> {
        let mut conn = self.pool.get()?;
        let msgs = conn.execute(
            "DELETE FROM messages WHERE created_at::timestamptz < NOW() - make_interval(days => $1)",
            &[&days],
        )?;
        let notifs = conn.execute(
            "DELETE FROM notifications WHERE created_at::timestamptz < NOW() - make_interval(days => $1)",
            &[&days],
        )?;
        let rl = conn.execute(
            "DELETE FROM rate_limits WHERE window_start::timestamptz < NOW() - make_interval(days => $1)",
            &[&days],
        )?;
        Ok((msgs + rl, notifs))
    }

    pub fn cleanup_inactive_users(&self, inactive_days: i64) -> Result<u64> {
        let mut conn = self.pool.get()?;
        let rows = conn.query(
            "SELECT id FROM users
             WHERE (login_count = 0 AND created_at::timestamptz < NOW() - make_interval(days => $1))
                OR (login_count > 0 AND last_login_at::timestamptz < NOW() - make_interval(days => $1))",
            &[&inactive_days],
        )?;
        let mut deleted = 0u64;
        for row in &rows {
            let user_id: i64 = row.get(0);
            conn.execute("DELETE FROM follows WHERE follower_id = $1 OR following_id = $1", &[&user_id])?;
            conn.execute("DELETE FROM comments WHERE post_id IN (SELECT id FROM posts WHERE user_id = $1)", &[&user_id])?;
            conn.execute("DELETE FROM comments WHERE user_id = $1", &[&user_id])?;
            conn.execute("DELETE FROM post_hashtags WHERE post_id IN (SELECT id FROM posts WHERE user_id = $1)", &[&user_id])?;
            conn.execute("DELETE FROM posts WHERE user_id = $1", &[&user_id])?;
            conn.execute("DELETE FROM messages WHERE sender_id = $1 OR receiver_id = $1", &[&user_id])?;
            conn.execute("DELETE FROM notifications WHERE user_id = $1 OR from_user_id = $1", &[&user_id])?;
            conn.execute("DELETE FROM rate_limits WHERE user_id = $1", &[&user_id])?;
            conn.execute("DELETE FROM users WHERE id = $1", &[&user_id])?;
            deleted += 1;
        }
        Ok(deleted)
    }

    pub fn clear_image_from_posts(&self, path: &str) -> Result<u64> {
        let mut conn = self.pool.get()?;
        let n = conn.execute(
            "UPDATE posts SET image_path = '' WHERE image_path = $1",
            &[&path],
        )?;
        Ok(n)
    }

    pub fn cleanup_rate_limits(&self) -> Result<u64> {
        let mut conn = self.pool.get()?;
        let deleted = conn.execute(
            "DELETE FROM rate_limits WHERE window_start::timestamptz < NOW() - interval '1 hour'",
            &[],
        )?;
        Ok(deleted)
    }

    pub fn export_user_data(&self, username: &str) -> Result<String> {
        let mut conn = self.pool.get()?;
        let username_lower = username.trim().to_lowercase();
        let user_row = conn.query_opt(
            "SELECT id, username, display_name, bio, utc_offset, created_at FROM users WHERE LOWER(username) = LOWER($1)",
            &[&username_lower],
        )?.ok_or_else(|| anyhow::anyhow!("Usuario '{}' no encontrado", username))?;

        let user_id: i64 = user_row.get(0);
        let user = serde_json::json!({
            "id": user_id,
            "username": user_row.get::<_, String>(1),
            "display_name": user_row.get::<_, String>(2),
            "bio": user_row.get::<_, String>(3),
            "utc_offset": user_row.get::<_, i32>(4),
            "created_at": user_row.get::<_, String>(5),
        });

        let posts_rows = conn.query(
            "SELECT id, content, image_path, created_at FROM posts WHERE user_id = $1 ORDER BY created_at DESC",
            &[&user_id],
        )?;
        let posts: Vec<serde_json::Value> = posts_rows.iter().map(|r| {
            serde_json::json!({
                "id": r.get::<_, i64>(0),
                "content": r.get::<_, String>(1),
                "image_path": r.get::<_, String>(2),
                "created_at": r.get::<_, String>(3),
            })
        }).collect();

        let comments_rows = conn.query(
            "SELECT id, post_id, content, created_at FROM comments WHERE user_id = $1 ORDER BY created_at DESC",
            &[&user_id],
        )?;
        let comments: Vec<serde_json::Value> = comments_rows.iter().map(|r| {
            serde_json::json!({
                "id": r.get::<_, i64>(0),
                "post_id": r.get::<_, i64>(1),
                "content": r.get::<_, String>(2),
                "created_at": r.get::<_, String>(3),
            })
        }).collect();

        let msgs_rows = conn.query(
            "SELECT m.id, m.sender_id, m.receiver_id, m.content, m.created_at, u.username as sender_name
             FROM messages m JOIN users u ON u.id = m.sender_id
             WHERE m.sender_id = $1 OR m.receiver_id = $1 ORDER BY m.created_at ASC",
            &[&user_id],
        )?;
        let messages: Vec<serde_json::Value> = msgs_rows.iter().map(|r| {
            serde_json::json!({
                "id": r.get::<_, i64>(0),
                "sender_id": r.get::<_, i64>(1),
                "receiver_id": r.get::<_, i64>(2),
                "sender_username": r.get::<_, String>(5),
                "content": r.get::<_, String>(3),
                "created_at": r.get::<_, String>(4),
            })
        }).collect();

        let followers_rows = conn.query(
            "SELECT u.username FROM users u JOIN follows f ON f.follower_id = u.id WHERE f.following_id = $1",
            &[&user_id],
        )?;
        let followers: Vec<String> = followers_rows.iter().map(|r| r.get(0)).collect();

        let following_rows = conn.query(
            "SELECT u.username FROM users u JOIN follows f ON f.following_id = u.id WHERE f.follower_id = $1",
            &[&user_id],
        )?;
        let following: Vec<String> = following_rows.iter().map(|r| r.get(0)).collect();

        let export = serde_json::json!({
            "exported_at": Utc::now().to_rfc3339(),
            "user": user,
            "posts": posts,
            "comments": comments,
            "messages": messages,
            "followers": followers,
            "following": following,
        });

        let json_str = serde_json::to_string_pretty(&export)?;
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("export_{}_{}.json", username_lower, timestamp);
        let path = format!("/data/uploads/{}", filename);
        std::fs::create_dir_all("/data/uploads").ok();
        std::fs::write(&path, &json_str)?;
        Ok(filename)
    }

    pub fn seed_data(&self) -> Result<()> {
        let mut conn = self.pool.get()?;
        let now = chrono::Utc::now();
        let ago = |mins: i64| (now - chrono::Duration::minutes(mins)).to_rfc3339();

        // Clean existing data
        conn.batch_execute("
            DELETE FROM notifications; DELETE FROM messages; DELETE FROM post_hashtags;
            DELETE FROM comments; DELETE FROM posts; DELETE FROM follows; DELETE FROM rate_limits; DELETE FROM users;
        ").map_err(|e| anyhow::anyhow!("Error limpiando datos: {}", e))?;

        // ── 52 Users ────────────────────────────────────────────────────
        let user_defs: [(&str, &str); 52] = [
            ("alice",     "Alice Rodríguez"),
            ("bob",       "Bob Martínez"),
            ("carol",     "Carolina López"),
            ("dave",      "David Chen"),
            ("eve",       "Eva García"),
            ("frank",     "Francisco Torres"),
            ("grace",     "Gabriela Ramírez"),
            ("hank",      "Héctor Vargas"),
            ("iris",      "Isabel Mendoza"),
            ("jack",      "Javier Castillo"),
            ("karen",     "Karen Herrera"),
            ("leo",       "Leonardo Rivas"),
            ("maria",     "María Fernández"),
            ("nacho",     "Ignacio Paredes"),
            ("olivia",    "Olivia Soto"),
            ("pablo",     "Pablo Núñez"),
            ("quinn",     "Quintín Delgado"),
            ("rosa",      "Rosa Guerrero"),
            ("sam",       "Samuel Ortega"),
            ("tina",      "Cristina Campos"),
            ("ulises",    "Ulises Medina"),
            ("vero",      "Verónica Rojas"),
            ("will",      "Wilson Aguilar"),
            ("xena",      "Ximena Peña"),
            ("yago",      "Yago Fuentes"),
            ("zoe",       "Zoe Santana"),
            ("coder42",   "Dev Master"),
            ("rustacean", "Rust Fanático"),
            ("linuxero",  "Tux Lover"),
            ("pythonista","Py Coder"),
            ("hackerx",   "Sec Ghost"),
            ("sysadmin",  "Root Admin"),
            ("neovimmer", "Vim Enjoyer"),
            ("frontend",  "CSS Fighter"),
            ("backend",   "API Builder"),
            ("fullstack", "Jack of All"),
            ("devops",    "Pipeline Runner"),
            ("datawiz",   "Data Wizard"),
            ("mlguru",    "ML Engineer"),
            ("designer",  "UI Pixel"),
            ("scifi_fan", "Scifi Reader"),
            ("gamer",     "Night Owl"),
            ("musico",    "Bass Player"),
            ("fotografo", "Lens Hunter"),
            ("chefcode",  "Code Chef"),
            ("cyclist",   "Bike Commuter"),
            ("yogi",      "Zen Coder"),
            ("writer",    "Doc Author"),
            ("tester",    "Bug Finder"),
            ("architect", "System Dreamer"),
            ("joker",     "Terminal Joker"),
            ("newbie",    "Fresh Start"),
        ];

        let mut uids: Vec<i64> = Vec::new();
        for (i, (uname, dname)) in user_defs.iter().enumerate() {
            let hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST)
                .map_err(|e| anyhow::anyhow!("bcrypt failed for {}: {}", uname, e))?;
            let created = if i >= 48 { ago(60*24*800) } else { ago(60*24*7 + i as i64 * 120) };
            let row = conn.query_opt(
                "INSERT INTO users (username, password_hash, display_name, created_at, last_login_at, login_count) VALUES ($1, $2, $3, $4, $4, 1) RETURNING id",
                &[&uname.to_string(), &hash, &dname.to_string(), &created],
            ).map_err(|e| anyhow::anyhow!("Error insertando usuario {}: {}", uname, e))?
            .ok_or_else(|| anyhow::anyhow!("INSERT no retornó fila para usuario {}", uname))?;
            let id: i64 = row.get(0);
            uids.push(id);
        }

        // Inactive users: set login_count=0, no last_login for the very last 2
        for idx in [50, 51] {
            if let Some(&uid) = uids.get(idx) {
                conn.execute(
                    "UPDATE users SET login_count = 0, last_login_at = created_at WHERE id = $1",
                    &[&uid],
                )?;
            }
        }

        // ── Follows (social graph) ────────────────────────────────────
        let follows: Vec<(usize, usize)> = vec![
            (0,1),(0,2),(0,3),(0,4),(0,6),(0,8),(0,12),(0,15),
            (1,0),(1,2),(1,5),(1,7),(1,26),(1,28),(1,30),
            (2,0),(2,1),(2,4),(2,9),(2,13),(2,19),(2,35),
            (3,0),(3,11),(3,14),(3,16),(3,26),(3,29),(3,33),
            (4,0),(4,1),(4,3),(4,10),(4,17),(4,24),(4,42),
            (5,1),(5,6),(5,7),(5,15),(5,22),(5,26),(5,38),
            (6,0),(6,5),(6,8),(6,18),(6,27),(6,31),(6,40),
            (7,1),(7,5),(7,9),(7,20),(7,28),(7,32),(7,43),
            (8,0),(8,6),(8,12),(8,21),(8,29),(8,35),(8,44),
            (9,2),(9,7),(9,13),(9,22),(9,30),(9,36),(9,45),
            (10,4),(10,8),(10,14),(10,23),(10,27),(10,37),
            (11,3),(11,9),(11,15),(11,24),(11,28),(11,38),
            (12,0),(12,8),(12,16),(12,25),(12,29),(12,39),
            (13,2),(13,9),(13,17),(13,22),(13,26),(13,40),
            (14,3),(14,10),(14,18),(14,23),(14,27),(14,41),
            (15,0),(15,5),(15,19),(15,24),(15,28),(15,42),
            (26,0),(26,1),(26,2),(26,3),(26,4),(26,27),(26,28),(26,29),
            (27,26),(27,28),(27,29),(27,30),(27,31),(27,32),
            (28,1),(28,2),(28,26),(28,27),(28,29),(28,33),
            (29,3),(29,26),(29,27),(29,28),(29,30),(29,34),
            (30,5),(30,26),(30,29),(30,31),(30,32),(30,35),
            (31,27),(31,30),(31,33),(31,36),(31,37),
            (32,28),(32,29),(32,34),(32,38),(32,39),
            (33,3),(33,26),(33,31),(33,35),(33,40),
            (34,29),(34,32),(34,36),(34,41),
            (35,2),(35,8),(35,31),(35,33),(35,37),(35,42),
            (36,30),(36,33),(36,34),(36,38),(36,43),
        ];
        for (f, t) in &follows {
            conn.execute(
                "INSERT INTO follows (follower_id, following_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                &[&uids[*f], &uids[*t]],
            ).ok();
        }

        // ── Posts (250+) ───────────────────────────────────────────────
        let templates: [(&str, &[&str]); 38] = [
            ("Acabo de descubrir #rust y estoy alucinando con el borrow checker", &["rust"]),
            ("Alguien mas usa #neovim con LSP para desarrollo? Es magia pura", &["neovim"]),
            ("Hoy compile el kernel de #linux en menos de 5 minutos. Que epoca para estar vivo", &["linux"]),
            ("El #opensource es lo mejor que le paso a la humanidad. Change my mind", &["opensource"]),
            ("Buenos dias #gente. Hoy toca deploy a produccion. Deseenme suerte", &["gente"]),
            ("Recomendaciones de #musica para programar? Necesito focus total", &["musica"]),
            ("Estoy armando un cluster de Kubernetes en casa. #devops #cloud", &["devops", "cloud"]),
            ("Lean esto sobre #rust async vs sync. Les vuela la cabeza", &["rust"]),
            ("El mejor #editor para codigo es el que te deja fluir. Para mi, neovim", &["editor"]),
            ("Thread sobre #seguridad en aplicaciones web. Abro hilo", &["seguridad"]),
            ("Quien va a la #rustconf este año? Nos juntamos?", &["rustconf"]),
            ("Termine mi proyecto en #python. Ahora quiero reescribirlo en rust, es normal?", &["python", "rust"]),
            ("La documentacion de #postgresql es oro puro. No subestimen la doc oficial", &["postgresql"]),
            ("Cual es su #terminal favorita? Yo uso alacritty con tmux", &["terminal"]),
            ("Les presento mi setup de #minimalismo digital. Solo terminal y shell", &["minimalismo"]),
            ("Acabo de leer un paper sobre #algoritmos distribuidos. Increible", &["algoritmos"]),
            ("Hoy empece a contribuir a un proyecto #opensource. Que emocion", &["opensource"]),
            ("Mejor practica para #git: commits atomicos y mensajes claros", &["git"]),
            ("Reflexion del dia: menos herramientas, mas pensamiento. #filosofia", &["filosofia"]),
            ("Estoy aprendiendo #golang. Que les parece comparado con rust?", &["golang"]),
            ("Tip del dia: usa #docker para entornos de desarrollo reproducibles", &["docker"]),
            ("La #ia esta cambiando como programamos. Opiniones?", &["ia"]),
            ("Acabo de configurar #nixos y no puedo creer lo limpio que queda todo", &["nixos"]),
            ("Buenas practicas de #testing: unitarios, integracion y e2e. Los 3 hacen falta", &["testing"]),
            ("Que monitor usan para programar? Yo tengo uno ultrawide y es gloria", &["hardware"]),
            ("Los errores de compilacion de #rust son los mejores. Te enseñan en vez de asustarte", &["rust"]),
            ("Hackathon este fin de semana. Quien se apunta? #hackathon #startup", &["hackathon", "startup"]),
            ("Mi rutina: cafe, codigo, cafe, deploy, cafe, dormir. #developer #life", &["developer", "life"]),
            ("Alguien esta usando #htmx? Opiniones sinceras por favor", &["htmx"]),
            ("La comunidad de #rust es la mas acogedora que conoci en 20 años de carrera", &["rust"]),
            ("Necesito un libro sobre arquitectura de #software. Recomendaciones?", &["software"]),
            ("Hoy hice mi primer PR a un proyecto grande. #opensource #achievement", &["opensource", "achievement"]),
            ("Debate: tabs vs spaces. Yo: tabs para accesibilidad. Ustedes?", &["debate"]),
            ("Aprendiendo #elixir y el pattern matching me esta volando la cabeza", &["elixir"]),
            ("Setup minimalista para #productividad: terminal, tmux, neovim, y un buen teclado", &["productividad"]),
            ("Los que usan #archlinux, como llevan el rolling release? Vale la pena?", &["archlinux"]),
            ("Hablemos de #backend con rust. Que frameworks recomiendan: axum o actix?", &["backend", "rust"]),
            ("El #teletrabajo mejoro mi calidad de vida un 300%. No vuelvo a oficina", &["teletrabajo"]),
        ];

        // Generate ~250 posts spread across users (0..48 active)
        let mut pids: Vec<i64> = Vec::new();
        for pi in 0..250 {
            let uid_idx = (pi * 7 + 3) % 48; // pseudo-random spread
            let uid = uids[uid_idx];
            let template_idx = pi % templates.len();
            let (text, tags) = &templates[template_idx];

            // Vary the content slightly
            let variants = [
                format!("{} [{}/{}]", text, pi + 1, 250),
                format!("{}. Que opinan?", text),
                format!("{} 🔥", text),
                format!("{} 👀", text),
                format!("{} — comparto mi experiencia", text),
            ];
            let content = &variants[pi % variants.len()];

            // Timestamp from 15 days ago to now, spread out
            let mins_ago = (250 - pi) as i64 * 80 + (uid_idx as i64 * 13);
            let ts = ago(mins_ago);

            let row = conn.query_opt(
                "INSERT INTO posts (user_id, content, created_at) VALUES ($1, $2, $3) RETURNING id",
                &[&uid, &content.to_string(), &ts],
            ).map_err(|e| anyhow::anyhow!("Error insertando post {} (user {}): {}", pi, uid, e))?
            .ok_or_else(|| anyhow::anyhow!("INSERT post no retornó fila para post {} user {}", pi, uid))?;
            let pid: i64 = row.get(0);
            pids.push(pid);

            for tag in *tags {
                conn.execute(
                    "INSERT INTO post_hashtags (post_id, tag) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    &[&pid, &tag.to_lowercase().to_string()],
                ).ok();
            }

            // Mention some users occasionally
            if pi % 5 == 0 {
                let mentioned_idx = (pi * 3 + 7) % 48;
                if mentioned_idx != uid_idx {
                    let muid = uids[mentioned_idx];
                    conn.execute(
                        "INSERT INTO notifications (user_id, from_user_id, type, created_at, related_id) VALUES ($1, $2, 'mention', $3, $4)",
                        &[&muid, &uid, &ts, &pid],
                    ).ok();
                }
            }
        }

        // ── Comments (150+) ────────────────────────────────────────────
        let comment_templates = [
            "Totalmente de acuerdo!",
            "No estoy muy seguro de eso...",
            "Que gran aporte, gracias por compartir",
            "Me paso algo muy similar",
            "Podrias dar mas detalles?",
            "Increible, no sabia eso",
            "Yo uso otra herramienta pero esto se ve bien",
            "Gracias por la recomendacion",
            "Justo lo que necesitaba leer hoy",
            "Habria que probarlo en produccion",
            "Tenes algun benchmark de eso?",
            "Me sirvio mucho tu comentario",
            "Es por aca, 100%",
            "Discrepo respetuosamente",
            "Añado: tambien sirve para testing",
        ];

        let mut comment_id = 0i64;
        for ci in 0..160 {
            let post_idx = (ci * 11 + 5) % pids.len().max(1);
            let pid = pids[post_idx];
            let commenter_idx = (ci * 3 + 11) % 48;
            let uid = uids[commenter_idx];
            let text = comment_templates[ci % comment_templates.len()];

            let parent = if ci > 10 && ci % 3 == 0 {
                Some(comment_id - (ci as i64 % 5 + 1))
            } else {
                None
            };
            // Ensure parent exists
            let parent = parent.filter(|&p| p > 0 && p < comment_id);

            let ts = ago((160i64 - ci as i64) * 60 + (commenter_idx as i64 * 5));

            let row = conn.query_one(
                "INSERT INTO comments (post_id, user_id, content, created_at, parent_comment_id) VALUES ($1, $2, $3, $4, $5) RETURNING id",
                &[&pid, &uid, &text.to_string(), &ts, &parent],
            ).ok();
            if let Some(r) = row {
                comment_id = r.get(0);
            } else {
                comment_id += 1;
            }
        }

        // ── Messages (conversations) ───────────────────────────────────
        let mut msg_count = 0;
        for pair_idx in 0..20 {
            let a_idx = pair_idx % 48;
            let b_idx = (pair_idx * 3 + 7) % 48;
            if a_idx == b_idx { continue; }
            let a = uids[a_idx];
            let b = uids[b_idx];

            let conversation: [(&str, bool); 6] = [
                ("Hola! Como va todo?", true),
                ("Bien! Vi tu ultimo post, muy bueno", false),
                ("Gracias! Estoy trabajando en algo nuevo", true),
                ("Que emocion, conta mas", false),
                ("Es un proyecto de #rust con TUI", true),
                ("Pasame el repo cuando este listo!", false),
            ];

            for (mi, (text, from_a)) in conversation.iter().enumerate() {
                let (sender, receiver) = if *from_a { (a, b) } else { (b, a) };
                let ts = ago(60 * 24 * 14 - pair_idx as i64 * 500 - mi as i64 * 120);
                conn.execute(
                    "INSERT INTO messages (sender_id, receiver_id, content, created_at) VALUES ($1, $2, $3, $4)",
                    &[&sender, &receiver, &text.to_string(), &ts],
                ).ok();
                msg_count += 1;
            }
        }

        // ── Follow notifications ───────────────────────────────────────
        for (f, t) in follows.iter().take(60) {
            let ts = ago(60 * 24 * 10 - *f as i64 * 30);
            conn.execute(
                "INSERT INTO notifications (user_id, from_user_id, type, created_at, related_id) VALUES ($1, $2, 'follow', $3, $4)",
                &[&uids[*t], &uids[*f], &ts, &uids[*f]],
            ).ok();
        }

        println!(
            "Seed: {} usuarios, {} posts, {} comentarios, {} mensajes, {} hashtags.",
            user_defs.len(),
            pids.len(),
            160,
            msg_count,
            templates.len(),
        );
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod mock_db {
    use anyhow::Result;
use chrono::{DateTime, Utc};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::Instant;

    use crate::db::{AuthResult, DatabaseOps};
    use crate::models::{Comment, Message, Notification, Post, User};

    pub(crate) struct MockData {
        pub users: Vec<(User, String)>,
        pub posts: Vec<Post>,
        pub follows: Vec<(i64, i64)>,
        pub comments: Vec<Comment>,
        pub messages: Vec<Message>,
        pub notifications: Vec<Notification>,
        pub next_id: i64,
    }

    impl Default for MockData {
        fn default() -> Self {
            Self {
                users: vec![],
                posts: vec![],
                follows: vec![],
                comments: vec![],
                messages: vec![],
                notifications: vec![],
                next_id: 1,
            }
        }
    }

    pub(crate) struct MockDatabase {
        pub data: Mutex<MockData>,
        rate_limiter: Mutex<HashMap<String, Vec<Instant>>>,
    }

    impl Default for MockDatabase {
        fn default() -> Self {
            Self {
                data: Mutex::new(MockData::default()),
                rate_limiter: Mutex::new(HashMap::new()),
            }
        }
    }

    impl MockDatabase {
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl DatabaseOps for MockDatabase {
        fn check_rate_limit(&self, user_id: i64, action: &str, max: usize, window_secs: u64) -> Result<()> {
            let key = format!("{}:{}", user_id, action);
            let now = Instant::now();
            let mut limiter = self.rate_limiter.lock().unwrap();
            let entries = limiter.entry(key).or_default();
            entries.retain(|t| now.duration_since(*t).as_secs() < window_secs);
            if entries.len() >= max {
                anyhow::bail!("Demasiadas solicitudes. Espera un momento.");
            }
            entries.push(now);
            Ok(())
        }

        fn check_register_rate_limit(&self) -> Result<()> {
            Ok(())
        }

        fn register_user(&self, username: &str, password: &str, display_name: &str) -> Result<User> {
            let mut data = self.data.lock().unwrap();
            let username_lower = username.trim().to_lowercase();
            if data.users.iter().any(|(u, _)| u.username == username_lower) {
                anyhow::bail!("El usuario ya existe");
            }
            let id = data.next_id;
            data.next_id += 1;
            let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
            let user = User {
                id,
                username: username_lower,
                display_name: display_name.to_string(),
                bio: String::new(),
                utc_offset: 0,
                created_at: Utc::now(),
            };
            data.users.push((user.clone(), hash));
            Ok(user)
        }

        fn authenticate(&self, username: &str, password: &str) -> Result<AuthResult> {
            let data = self.data.lock().unwrap();
            if let Some((user, hash)) = data.users.iter().find(|(u, _)| u.username.to_lowercase() == username.to_lowercase()) {
                if bcrypt::verify(password, hash)? {
                    return Ok(AuthResult::Success(user.clone()));
                } else {
                    return Ok(AuthResult::WrongPassword);
                }
            }
            Ok(AuthResult::UserNotFound)
        }

        fn get_user_by_id(&self, id: i64) -> Result<Option<User>> {
            let data = self.data.lock().unwrap();
            Ok(data.users.iter().find(|(u, _)| u.id == id).map(|(u, _)| u.clone()))
        }

        fn search_users(&self, query: &str, offset: u64, limit: u64) -> Result<Vec<User>> {
            let data = self.data.lock().unwrap();
            let q = query.to_lowercase();
            Ok(data.users.iter()
                .filter(|(u, _)| u.username.to_lowercase().contains(&q) || u.display_name.to_lowercase().contains(&q))
                .map(|(u, _)| u.clone())
                .skip(offset as usize)
                .take(limit as usize)
                .collect())
        }

        fn create_post(&self, user_id: i64, content: &str, image_path: Option<&str>) -> Result<Post> {
            self.check_rate_limit(user_id, "post", 5, 60)?;
            let mut data = self.data.lock().unwrap();
            let username = data.users.iter()
                .find(|(u, _)| u.id == user_id)
                .map(|(u, _)| u.username.clone())
                .ok_or_else(|| anyhow::anyhow!("Usuario no encontrado"))?;
            let id = data.next_id;
            data.next_id += 1;
            let post = Post {
                id,
                user_id,
                username,
                content: content.to_string(),
                image_path: image_path.map(|s| s.to_string()).filter(|s| !s.is_empty()),
                created_at: Utc::now(),
            };
            data.posts.push(post.clone());
            Ok(post)
        }

        fn get_timeline(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>> {
            let data = self.data.lock().unwrap();
            let mut posts: Vec<Post> = data.posts.iter()
                .filter(|p| {
                    p.user_id == user_id
                        || data.follows.iter().any(|(f, fol)| *f == user_id && *fol == p.user_id)
                })
                .cloned()
                .collect();
            posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(posts.into_iter().skip(offset as usize).take(limit as usize).collect())
        }

        fn follow_user(&self, follower_id: i64, following_id: i64) -> Result<()> {
            self.check_rate_limit(follower_id, "follow", 10, 60)?;
            let mut data = self.data.lock().unwrap();
            if !data.follows.iter().any(|(f, fol)| *f == follower_id && *fol == following_id) {
                data.follows.push((follower_id, following_id));
            }
            Ok(())
        }

        fn unfollow_user(&self, follower_id: i64, following_id: i64) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            data.follows.retain(|(f, fol)| *f != follower_id || *fol != following_id);
            Ok(())
        }

        fn is_following(&self, follower_id: i64, following_id: i64) -> Result<bool> {
            let data = self.data.lock().unwrap();
            Ok(data.follows.iter().any(|(f, fol)| *f == follower_id && *fol == following_id))
        }

        fn get_followers(&self, user_id: i64) -> Result<Vec<User>> {
            let data = self.data.lock().unwrap();
            Ok(data.follows.iter()
                .filter(|(_, fol)| *fol == user_id)
                .filter_map(|(f, _)| data.users.iter().find(|(u, _)| u.id == *f).map(|(u, _)| u.clone()))
                .collect())
        }

        fn get_following(&self, user_id: i64) -> Result<Vec<User>> {
            let data = self.data.lock().unwrap();
            Ok(data.follows.iter()
                .filter(|(f, _)| *f == user_id)
                .filter_map(|(_, fol)| data.users.iter().find(|(u, _)| u.id == *fol).map(|(u, _)| u.clone()))
                .collect())
        }

    fn get_post_by_id(&self, post_id: i64) -> Result<Option<Post>> {
            let data = self.data.lock().unwrap();
            Ok(data.posts.iter().find(|p| p.id == post_id).cloned())
        }

    fn get_posts_by_user(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Post>> {
            let data = self.data.lock().unwrap();
            let mut posts: Vec<Post> = data.posts.iter()
                .filter(|p| p.user_id == user_id)
                .cloned()
                .collect();
            posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(posts.into_iter().skip(offset as usize).take(limit as usize).collect())
        }

        fn search_posts(&self, query: &str, time_filter: &str, offset: u64, limit: u64) -> Result<Vec<Post>> {
            let data = self.data.lock().unwrap();
            let q = query.to_lowercase();
            let cutoff = match time_filter {
                "24h" => Some(chrono::Duration::hours(24)),
                "7d" => Some(chrono::Duration::days(7)),
                "30d" => Some(chrono::Duration::days(30)),
                _ => None,
            };
            let now = Utc::now();
            let mut posts: Vec<Post> = data.posts.iter()
                .filter(|p| {
                    let content_match = p.content.to_lowercase().contains(&q);
                    let time_match = match cutoff {
                        Some(d) => now.signed_duration_since(p.created_at) < d,
                        None => true,
                    };
                    content_match && time_match
                })
                .cloned()
                .collect();
            posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(posts.into_iter().skip(offset as usize).take(limit as usize).collect())
        }

        fn search_posts_by_user(&self, query: &str, _offset: u64, _limit: u64) -> Result<Vec<Post>> {
            let data = self.data.lock().unwrap();
            let q = query.to_lowercase();
            let mut posts: Vec<Post> = data.posts.iter()
                .filter(|p| p.username.to_lowercase().contains(&q))
                .cloned().collect();
            posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(posts)
        }

        fn search_posts_by_date(&self, query: &str, _offset: u64, _limit: u64) -> Result<Vec<Post>> {
            let data = self.data.lock().unwrap();
            let q = query.to_lowercase();
            let mut posts: Vec<Post> = data.posts.iter()
                .filter(|p| p.created_at.to_rfc3339().to_lowercase().contains(&q))
                .cloned().collect();
            posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(posts)
        }

        fn add_comment(&self, post_id: i64, user_id: i64, content: &str, parent_id: Option<i64>) -> Result<Comment> {
            self.check_rate_limit(user_id, "comment", 10, 60)?;
            let mut data = self.data.lock().unwrap();
            let username = data.users.iter()
                .find(|(u, _)| u.id == user_id)
                .map(|(u, _)| u.username.clone())
                .ok_or_else(|| anyhow::anyhow!("Usuario no encontrado"))?;
            let id = data.next_id;
            data.next_id += 1;
            let comment = Comment {
                id,
                post_id,
                user_id,
                username,
                content: content.to_string(),
                created_at: Utc::now(),
                parent_comment_id: parent_id,
            };
            data.comments.push(comment.clone());
            Ok(comment)
        }

        fn get_comments(&self, post_id: i64) -> Result<Vec<Comment>> {
            let data = self.data.lock().unwrap();
            let mut comments: Vec<Comment> = data.comments.iter()
                .filter(|c| c.post_id == post_id)
                .cloned()
                .collect();
            comments.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            Ok(comments)
        }

        fn update_post(&self, post_id: i64, user_id: i64, content: &str) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            let post = data.posts.iter_mut()
                .find(|p| p.id == post_id)
                .ok_or_else(|| anyhow::anyhow!("Post no encontrado"))?;
            if post.user_id != user_id {
                anyhow::bail!("No tienes permiso para editar este post");
            }
            post.content = content.to_string();
            Ok(())
        }

        fn delete_post(&self, post_id: i64, user_id: i64) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            let idx = data.posts.iter().position(|p| p.id == post_id)
                .ok_or_else(|| anyhow::anyhow!("Post no encontrado"))?;
            if data.posts[idx].user_id != user_id {
                anyhow::bail!("No tienes permiso para eliminar este post");
            }
            data.comments.retain(|c| c.post_id != post_id);
            data.posts.remove(idx);
            Ok(())
        }

        fn delete_comment(&self, comment_id: i64, user_id: i64) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            let idx = data.comments.iter().position(|c| c.id == comment_id)
                .ok_or_else(|| anyhow::anyhow!("Comentario no encontrado"))?;
            if data.comments[idx].user_id != user_id {
                anyhow::bail!("No tienes permiso para eliminar este comentario");
            }
            data.comments.remove(idx);
            Ok(())
        }

        fn delete_user(&self, user_id: i64) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            data.users.retain(|(u, _)| u.id != user_id);
            data.posts.retain(|p| p.user_id != user_id);
            data.comments.retain(|c| c.user_id != user_id);
            data.follows.retain(|(f, fol)| *f != user_id && *fol != user_id);
            data.messages.retain(|m| m.sender_id != user_id && m.receiver_id != user_id);
            data.notifications.retain(|n| n.user_id != user_id && n.from_user_id != user_id);
            Ok(())
        }

        fn send_message(&self, sender_id: i64, receiver_id: i64, content: &str) -> Result<Message> {
            self.check_rate_limit(sender_id, "message", 10, 60)?;
            let mut data = self.data.lock().unwrap();
            let username = data.users.iter()
                .find(|(u, _)| u.id == sender_id)
                .map(|(u, _)| u.username.clone())
                .ok_or_else(|| anyhow::anyhow!("Usuario no encontrado"))?;
            let id = data.next_id;
            data.next_id += 1;
            let msg = Message {
                id,
                sender_id,
                receiver_id,
                sender_username: username,
                content: content.to_string(),
                created_at: Utc::now(),
                read: false,
            };
            data.messages.push(msg.clone());
            Ok(msg)
        }

        fn get_conversations(&self, user_id: i64) -> Result<Vec<User>> {
            let data = self.data.lock().unwrap();
            let mut other_ids: Vec<i64> = data.messages.iter()
                .filter(|m| m.sender_id == user_id || m.receiver_id == user_id)
                .map(|m| if m.sender_id == user_id { m.receiver_id } else { m.sender_id })
                .collect();
            other_ids.sort();
            other_ids.dedup();
            let users: Vec<User> = other_ids.iter()
                .filter_map(|id| data.users.iter().find(|(u, _)| u.id == *id).map(|(u, _)| u.clone()))
                .collect();
            Ok(users)
        }

        fn get_messages(&self, user_id: i64, other_id: i64) -> Result<Vec<Message>> {
            let data = self.data.lock().unwrap();
            let mut msgs: Vec<Message> = data.messages.iter()
                .filter(|m| {
                    (m.sender_id == user_id && m.receiver_id == other_id)
                        || (m.sender_id == other_id && m.receiver_id == user_id)
                })
                .cloned()
                .collect();
            msgs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            Ok(msgs)
        }

        fn get_unread_count(&self, user_id: i64) -> Result<i64> {
            let data = self.data.lock().unwrap();
            Ok(data.messages.iter()
                .filter(|m| m.receiver_id == user_id && !m.read)
                .count() as i64)
        }

        fn mark_messages_read(&self, user_id: i64, other_id: i64) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            for m in data.messages.iter_mut() {
                if m.sender_id == other_id && m.receiver_id == user_id {
                    m.read = true;
                }
            }
            Ok(())
        }

        fn update_profile(&self, user_id: i64, display_name: &str, bio: &str, utc_offset: i32) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            if let Some((user, _)) = data.users.iter_mut().find(|(u, _)| u.id == user_id) {
                user.display_name = display_name.to_string();
                user.bio = bio.to_string();
                user.utc_offset = utc_offset;
            }
            Ok(())
        }

        fn update_timezone(&self, user_id: i64, utc_offset: i32) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            if let Some((user, _)) = data.users.iter_mut().find(|(u, _)| u.id == user_id) {
                user.utc_offset = utc_offset;
            }
            Ok(())
        }

        fn add_notification(&self, user_id: i64, from_user_id: i64, notif_type: &str, related_id: Option<i64>) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            let from_username = data.users.iter()
                .find(|(u, _)| u.id == from_user_id)
                .map(|(u, _)| u.username.clone())
                .unwrap_or_default();
            let id = data.next_id;
            data.next_id += 1;
            data.notifications.push(Notification {
                id,
                user_id,
                from_user_id,
                from_username,
                notif_type: notif_type.to_string(),
                created_at: Utc::now(),
                read: false,
                related_id,
            });
            Ok(())
        }

        fn get_notifications(&self, user_id: i64, offset: u64, limit: u64) -> Result<Vec<Notification>> {
            let data = self.data.lock().unwrap();
            let mut notifs: Vec<Notification> = data.notifications.iter()
                .filter(|n| n.user_id == user_id)
                .cloned()
                .collect();
            notifs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(notifs.into_iter().skip(offset as usize).take(limit as usize).collect())
        }

        fn get_unread_notifications_count(&self, user_id: i64) -> Result<i64> {
            let data = self.data.lock().unwrap();
            Ok(data.notifications.iter()
                .filter(|n| n.user_id == user_id && !n.read)
                .count() as i64)
        }

        fn mark_notifications_read(&self, user_id: i64) -> Result<()> {
            let mut data = self.data.lock().unwrap();
            for n in data.notifications.iter_mut() {
                if n.user_id == user_id {
                    n.read = true;
                }
            }
            Ok(())
        }

        fn cleanup_old_data(&self, _days: i64) -> Result<(u64, u64)> {
            Ok((0, 0))
        }

        fn cleanup_inactive_users(&self, _days: i64) -> Result<u64> {
            Ok(0)
        }

        fn get_posts_by_hashtag(&self, tag: &str, _offset: u64, _limit: u64) -> Result<Vec<Post>> {
            let data = self.data.lock().unwrap();
            let tag_binding = tag.to_lowercase();
            let tag_lower = tag_binding.trim_start_matches('#');
            Ok(data.posts.iter()
                .filter(|p| {
                    p.content.split_whitespace()
                        .any(|w| {
                            let w_clean = w.to_lowercase();
                            w_clean.trim_start_matches('#') == tag_lower
                        })
                })
                .cloned()
                .collect())
        }

        fn get_trending_hashtags(&self, _limit: u64) -> Result<Vec<(String, i64)>> {
            let data = self.data.lock().unwrap();
            let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
            for post in &data.posts {
                for word in post.content.split_whitespace() {
                    if word.starts_with('#') && word.len() > 1 {
                        let tag = word[1..].to_lowercase();
                        *counts.entry(tag).or_insert(0) += 1;
                    }
                }
            }
            let mut result: Vec<(String, i64)> = counts.into_iter().collect();
            result.sort_by(|a, b| b.1.cmp(&a.1));
            Ok(result)
        }

        fn export_user_data(&self, _username: &str) -> Result<String> {
            anyhow::bail!("Export no soportado en MockDatabase");
        }

        fn clear_image_from_posts(&self, path: &str) -> Result<u64> {
            let mut data = self.data.lock().unwrap();
            let mut count = 0u64;
            for post in data.posts.iter_mut() {
                if post.image_path.as_deref() == Some(path) {
                    post.image_path = None;
                    count += 1;
                }
            }
            Ok(count)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn setup() -> MockDatabase {
            let db = MockDatabase::new();
            db.register_user("alice", "pass123", "Alice A.").unwrap();
            db.register_user("bob", "pass456", "Bob B.").unwrap();
            db
        }

        #[test]
        fn test_register_and_authenticate() {
            let db = setup();
            let user = db.authenticate("alice", "pass123").unwrap();
            assert!(matches!(user, AuthResult::Success(_)));
            if let AuthResult::Success(u) = user {
                assert_eq!(u.username, "alice");
            }

            let bad = db.authenticate("alice", "wrongpass").unwrap();
            assert!(matches!(bad, AuthResult::WrongPassword));

            let nonexist = db.authenticate("charlie", "pass").unwrap();
            assert!(matches!(nonexist, AuthResult::UserNotFound));
        }

        #[test]
        fn test_register_duplicate_username() {
            let db = setup();
            let result = db.register_user("alice", "otrapass", "Alice Dup");
            assert!(result.is_err());
        }

        #[test]
        fn test_get_user_by_id() {
            let db = setup();
            let user = db.get_user_by_id(1).unwrap().unwrap();
            assert_eq!(user.username, "alice");

            let missing = db.get_user_by_id(999).unwrap();
            assert!(missing.is_none());
        }

        #[test]
        fn test_search_users() {
            let db = setup();
            let results = db.search_users("ali", 0, 10).unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].username, "alice");

            let all = db.search_users("", 0, 10).unwrap();
            assert_eq!(all.len(), 2);
        }

        #[test]
        fn test_create_and_get_timeline() {
            let db = setup();
            db.create_post(1, "Hello from Alice", None).unwrap();
            db.create_post(2, "Bob's first post", None).unwrap();
            db.follow_user(1, 2).unwrap();

            let timeline = db.get_timeline(1, 0, 20).unwrap();
            assert_eq!(timeline.len(), 2);
            assert!(timeline.iter().any(|p| p.content == "Hello from Alice"));
            assert!(timeline.iter().any(|p| p.content == "Bob's first post"));
        }

        #[test]
        fn test_follow_unfollow() {
            let db = setup();
            assert!(!db.is_following(1, 2).unwrap());

            db.follow_user(1, 2).unwrap();
            assert!(db.is_following(1, 2).unwrap());

            db.unfollow_user(1, 2).unwrap();
            assert!(!db.is_following(1, 2).unwrap());
        }

        #[test]
        fn test_followers_and_following() {
            let db = setup();
            db.follow_user(1, 2).unwrap();
            db.follow_user(2, 1).unwrap();

            let alice_followers = db.get_followers(1).unwrap();
            let alice_following = db.get_following(1).unwrap();
            assert_eq!(alice_followers.len(), 1);
            assert_eq!(alice_followers[0].username, "bob");
            assert_eq!(alice_following.len(), 1);
            assert_eq!(alice_following[0].username, "bob");
        }

        #[test]
        fn test_get_posts_by_user() {
            let db = setup();
            db.create_post(1, "Post 1", None).unwrap();
            db.create_post(1, "Post 2", None).unwrap();
            db.create_post(2, "Bob post", None).unwrap();

            let alice_posts = db.get_posts_by_user(1, 0, 20).unwrap();
            assert_eq!(alice_posts.len(), 2);
            assert!(alice_posts.iter().all(|p| p.user_id == 1));
        }

        #[test]
        fn test_comments() {
            let db = setup();
            let post = db.create_post(1, "Alice's post", None).unwrap();
            let comment = db.add_comment(post.id, 2, "Nice!", None).unwrap();
            assert_eq!(comment.content, "Nice!");
            assert_eq!(comment.user_id, 2);

            let comments = db.get_comments(post.id).unwrap();
            assert_eq!(comments.len(), 1);
        }

        #[test]
        fn test_update_post() {
            let db = setup();
            let post = db.create_post(1, "Original", None).unwrap();
            db.update_post(post.id, 1, "Updated").unwrap();

            let timeline = db.get_timeline(1, 0, 20).unwrap();
            assert_eq!(timeline[0].content, "Updated");
        }

        #[test]
        fn test_update_post_wrong_user() {
            let db = setup();
            let post = db.create_post(1, "Alice's post", None).unwrap();
            let result = db.update_post(post.id, 2, "hacked");
            assert!(result.is_err());
        }

        #[test]
        fn test_delete_post() {
            let db = setup();
            let post = db.create_post(1, "To delete", None).unwrap();
            db.add_comment(post.id, 2, "comment", None).unwrap();
            db.delete_post(post.id, 1).unwrap();

            let timeline = db.get_timeline(1, 0, 20).unwrap();
            assert!(timeline.is_empty());
            assert!(db.get_comments(post.id).unwrap().is_empty());
        }

        #[test]
        fn test_delete_comment() {
            let db = setup();
            let post = db.create_post(1, "Post", None).unwrap();
            let comment = db.add_comment(post.id, 2, "comment", None).unwrap();
            db.delete_comment(comment.id, 2).unwrap();
            assert!(db.get_comments(post.id).unwrap().is_empty());
        }

        #[test]
        fn test_delete_user() {
            let db = setup();
            db.create_post(1, "Alice post", None).unwrap();
            db.follow_user(1, 2).unwrap();
            db.send_message(1, 2, "Hi").unwrap();
            db.delete_user(1).unwrap();

            assert!(db.get_user_by_id(1).unwrap().is_none());
            assert!(db.get_posts_by_user(1, 0, 20).unwrap().is_empty());
            assert!(db.get_following(1).unwrap().is_empty());
        }

        #[test]
        fn test_messages() {
            let db = setup();
            let msg = db.send_message(1, 2, "Hey Bob!").unwrap();
            assert_eq!(msg.content, "Hey Bob!");
            assert!(!msg.read);

            let msgs = db.get_messages(1, 2).unwrap();
            assert_eq!(msgs.len(), 1);

            let convos = db.get_conversations(1).unwrap();
            assert_eq!(convos.len(), 1);
            assert_eq!(convos[0].username, "bob");
        }

        #[test]
        fn test_unread_messages() {
            let db = setup();
            db.send_message(1, 2, "Msg1").unwrap();
            db.send_message(1, 2, "Msg2").unwrap();

            let unread = db.get_unread_count(2).unwrap();
            assert_eq!(unread, 2);

            db.mark_messages_read(2, 1).unwrap();
            let unread = db.get_unread_count(2).unwrap();
            assert_eq!(unread, 0);
        }

        #[test]
        fn test_notifications() {
            let db = setup();
            db.add_notification(1, 2, "follow", None).unwrap();
            db.add_notification(1, 2, "like", None).unwrap();

            let notifs = db.get_notifications(1, 0, 50).unwrap();
            assert_eq!(notifs.len(), 2);

            let unread = db.get_unread_notifications_count(1).unwrap();
            assert_eq!(unread, 2);

            db.mark_notifications_read(1).unwrap();
            let unread = db.get_unread_notifications_count(1).unwrap();
            assert_eq!(unread, 0);
        }

        #[test]
        fn test_update_profile() {
            let db = setup();
            db.update_profile(1, "Alice B.", "My new bio", 0).unwrap();
            let user = db.get_user_by_id(1).unwrap().unwrap();
            assert_eq!(user.display_name, "Alice B.");
            assert_eq!(user.bio, "My new bio");
        }

        #[test]
        fn test_create_post_with_image() {
            let db = setup();
            let post = db.create_post(1, "With image", Some("https://example.com/img.jpg")).unwrap();
            assert!(post.image_path.is_some());
            assert_eq!(post.image_path.unwrap(), "https://example.com/img.jpg");
        }

        #[test]
        fn test_rate_limiting() {
            let db = setup();
            // Default rate limit: 5 per 60s for posts
            for i in 0..5 {
                db.create_post(1, &format!("Post {}", i), None).unwrap();
            }
            let result = db.create_post(1, "Too many", None);
            assert!(result.is_err());
        }

        #[test]
        fn test_cleanup_old_data() {
            let db = setup();
            let (msgs, notifs) = db.cleanup_old_data(90).unwrap();
            assert_eq!(msgs, 0);
            assert_eq!(notifs, 0);
        }

        #[test]
        fn test_post_ownership() {
            let db = setup();
            let post = db.create_post(1, "Alice post", None).unwrap();
            // Bob tries to delete Alice's post
            let result = db.delete_post(post.id, 2);
            assert!(result.is_err());
            // Post should still exist
            assert_eq!(db.get_timeline(1, 0, 20).unwrap().len(), 1);
        }

        #[test]
        fn test_follow_self() {
            let db = setup();
            db.follow_user(1, 1).unwrap();
            // Should be allowed by the mock (the real DB may handle this differently)
            assert!(db.is_following(1, 1).unwrap());
        }

        #[test]
        fn test_empty_timeline() {
            let db = setup();
            let timeline = db.get_timeline(1, 0, 20).unwrap();
            assert!(timeline.is_empty());
        }

        #[test]
        fn test_multiple_comments() {
            let db = setup();
            let post = db.create_post(1, "Post", None).unwrap();
            db.add_comment(post.id, 2, "First", None).unwrap();
            db.add_comment(post.id, 1, "Second", None).unwrap();
            db.add_comment(post.id, 2, "Third", None).unwrap();

            let comments = db.get_comments(post.id).unwrap();
            assert_eq!(comments.len(), 3);
            assert_eq!(comments[0].content, "First");
            assert_eq!(comments[1].content, "Second");
            assert_eq!(comments[2].content, "Third");
        }
    }
}
