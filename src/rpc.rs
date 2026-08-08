use std::io::{BufRead, Write};

use anyhow::Result;
use serde_json::{Value, json};

use crate::db::{AuthResult, Database};

pub fn run(db: Database) -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let mut session_user_id: Option<i64> = None;

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                write_response(&mut stdout, None, Err(anyhow::anyhow!(error)))?;
                continue;
            }
        };
        let id = request.get("id").and_then(Value::as_u64);
        let action = request.get("action").and_then(Value::as_str).unwrap_or("");
        let result = handle(&db, &request, action, &mut session_user_id);
        write_response(&mut stdout, id, result)?;
    }
    Ok(())
}

fn handle(
    db: &Database,
    request: &Value,
    action: &str,
    session_user_id: &mut Option<i64>,
) -> Result<Value> {
    match action {
        "ping" => Ok(json!({ "version": env!("CARGO_PKG_VERSION") })),
        "login" => {
            let username = text(request, "username")?;
            let password = text(request, "password")?;
            match db.authenticate(username, password)? {
                AuthResult::Success(user) => {
                    *session_user_id = Some(user.id);
                    Ok(serde_json::to_value(user)?)
                }
                AuthResult::UserNotFound => anyhow::bail!("El usuario no existe."),
                AuthResult::WrongPassword => anyhow::bail!("Contraseña incorrecta."),
            }
        }
        "register" => {
            db.check_register_rate_limit()?;
            let user = db.register_user(
                text(request, "username")?,
                text(request, "password")?,
                text(request, "display_name")?,
                request.get("invite_code").and_then(Value::as_str),
            )?;
            *session_user_id = Some(user.id);
            Ok(serde_json::to_value(user)?)
        }
        "logout" => {
            *session_user_id = None;
            Ok(json!(null))
        }
        "timeline" => {
            let user_id = require_session(*session_user_id)?;
            Ok(serde_json::to_value(db.get_timeline(
                user_id,
                number(request, "offset", 0),
                30,
            )?)?)
        }
        "create_post" => {
            let user_id = require_session(*session_user_id)?;
            let content = text(request, "content")?.trim();
            if content.is_empty() || content.chars().count() > 5000 {
                anyhow::bail!("La publicación debe contener entre 1 y 5000 caracteres.");
            }
            let image_path = request.get("image_path").and_then(Value::as_str);
            if let Some(path) = image_path {
                let external = (path.starts_with("https://") || path.starts_with("http://"))
                    && path.len() <= 2048;
                if !external {
                    let username = db
                        .get_user_by_id(user_id)?
                        .ok_or_else(|| anyhow::anyhow!("La cuenta ya no existe."))?
                        .username;
                    let allowed =
                        crate::ssh::list_uploaded_images(&username)
                            .into_iter()
                            .any(|(name, _)| {
                                path == format!(
                                    "{}/{}/{}",
                                    crate::ssh::upload_dir(),
                                    username,
                                    name
                                )
                            });
                    if !allowed {
                        anyhow::bail!("La imagen no pertenece a tus archivos subidos.");
                    }
                }
            }
            Ok(serde_json::to_value(
                db.create_post(user_id, content, image_path)?,
            )?)
        }
        "post_detail" => {
            require_session(*session_user_id)?;
            let post_id = required_i64(request, "post_id")?;
            let post = db
                .get_post_by_id(post_id)?
                .ok_or_else(|| anyhow::anyhow!("La publicación no existe."))?;
            let comments = db.get_comments(post_id)?;
            Ok(json!({ "post": post, "comments": comments }))
        }
        "add_comment" => {
            let user_id = require_session(*session_user_id)?;
            let post_id = required_i64(request, "post_id")?;
            let content = text(request, "content")?.trim();
            if content.is_empty() || content.chars().count() > 5000 {
                anyhow::bail!("El comentario debe contener entre 1 y 5000 caracteres.");
            }
            let parent_id = request.get("parent_id").and_then(Value::as_i64);
            Ok(serde_json::to_value(
                db.add_comment(post_id, user_id, content, parent_id)?,
            )?)
        }
        "search" => {
            require_session(*session_user_id)?;
            Ok(serde_json::to_value(db.search_posts(
                text(request, "query")?,
                "all",
                0,
                50,
            )?)?)
        }
        "search_all" => {
            require_session(*session_user_id)?;
            let query = text(request, "query")?.trim().trim_start_matches('@');
            let users = db.search_users(query, 0, 20)?;
            let posts = db.search_posts(query, "all", 0, 50)?;
            Ok(json!({ "users": users, "posts": posts }))
        }
        "conversations" => {
            let user_id = require_session(*session_user_id)?;
            Ok(serde_json::to_value(db.get_conversations(user_id)?)?)
        }
        "unread_messages" => {
            let user_id = require_session(*session_user_id)?;
            Ok(json!({ "count": db.get_unread_count(user_id)? }))
        }
        "message_previews" => {
            let user_id = require_session(*session_user_id)?;
            Ok(serde_json::to_value(
                db.get_recent_message_previews(user_id, 4)?,
            )?)
        }
        "unread_notifications" => {
            let user_id = require_session(*session_user_id)?;
            Ok(json!({ "count": db.get_unread_notifications_count(user_id)? }))
        }
        "notifications_peek" => {
            let user_id = require_session(*session_user_id)?;
            Ok(serde_json::to_value(db.get_notifications(user_id, 0, 4)?)?)
        }
        "messages" => {
            let user_id = require_session(*session_user_id)?;
            let other_id = required_i64(request, "other_id")?;
            db.mark_messages_read(user_id, other_id)?;
            Ok(serde_json::to_value(db.get_messages(user_id, other_id)?)?)
        }
        "send_message" => {
            let user_id = require_session(*session_user_id)?;
            let other_id = required_i64(request, "other_id")?;
            let content = text(request, "content")?.trim();
            if content.is_empty() || content.chars().count() > 5000 {
                anyhow::bail!("El mensaje debe contener entre 1 y 5000 caracteres.");
            }
            Ok(serde_json::to_value(
                db.send_message(user_id, other_id, content, false)?,
            )?)
        }
        "notifications" => {
            let user_id = require_session(*session_user_id)?;
            let notifications = db.get_notifications(user_id, 0, 50)?;
            db.mark_notifications_read(user_id)?;
            Ok(serde_json::to_value(notifications)?)
        }
        "profile" => {
            let own_user_id = require_session(*session_user_id)?;
            let user_id = request
                .get("user_id")
                .and_then(Value::as_i64)
                .unwrap_or(own_user_id);
            let user = db
                .get_user_by_id(user_id)?
                .ok_or_else(|| anyhow::anyhow!("La cuenta ya no existe."))?;
            let posts = db.get_posts_by_user(user_id, 0, 30)?;
            Ok(json!({ "user": user, "posts": posts }))
        }
        "upload_prepare" | "uploads" => {
            let user_id = require_session(*session_user_id)?;
            let username = db
                .get_user_by_id(user_id)?
                .ok_or_else(|| anyhow::anyhow!("La cuenta ya no existe."))?
                .username;
            if action == "upload_prepare" {
                let ip = std::env::var("SSH_CLIENT_IP")
                    .map_err(|_| anyhow::anyhow!("No se pudo identificar la sesión SSH."))?;
                crate::ssh::write_scp_user_for_token(&ip, &username);
            }
            let files = crate::ssh::list_uploaded_images(&username);
            let upload_user = if action == "upload_prepare" {
                let token = format!("upload-{:016x}", rand::random::<u64>());
                crate::ssh::write_scp_user_for_token(&token, &username);
                token
            } else {
                "upload".to_string()
            };
            let host = std::env::var("AGORA_PUBLIC_HOST").unwrap_or_else(|_| "localhost".into());
            let port = std::env::var("AGORA_PUBLIC_SSH_PORT").unwrap_or_else(|_| "2222".into());
            Ok(json!({
                "username": username,
                "expires_in": 300,
                "command": format!("scp -P {port} archivo.jpg {upload_user}@{host}:"),
                "files": files.into_iter().map(|(name, size)| {
                    let path = format!("{}/{}/{}", crate::ssh::upload_dir(), username, name);
                    json!({ "name": name, "path": path, "size": size })
                }).collect::<Vec<_>>()
            }))
        }
        "radio" => {
            require_session(*session_user_id)?;
            let hashtags = db.get_trending_hashtags(30)?;
            let mut posts = Vec::new();
            for (tag, count) in &hashtags {
                if let Some(post) = db.get_posts_by_hashtag(tag, 0, 1)?.into_iter().next() {
                    posts.push(json!({ "tag": tag, "count": count, "post": post }));
                }
            }
            Ok(json!({ "items": posts }))
        }
        _ => anyhow::bail!("Acción RPC desconocida: {action}"),
    }
}

fn write_response(stdout: &mut impl Write, id: Option<u64>, result: Result<Value>) -> Result<()> {
    let response = match result {
        Ok(data) => json!({ "id": id, "ok": true, "data": data }),
        Err(error) => json!({ "id": id, "ok": false, "error": error.to_string() }),
    };
    serde_json::to_writer(&mut *stdout, &response)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn require_session(user_id: Option<i64>) -> Result<i64> {
    user_id.ok_or_else(|| anyhow::anyhow!("Debes iniciar sesión."))
}

fn text<'a>(request: &'a Value, field: &str) -> Result<&'a str> {
    request
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Falta el campo {field}."))
}

fn required_i64(request: &Value, field: &str) -> Result<i64> {
    request
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("Falta el campo {field}."))
}

fn number(request: &Value, field: &str, default: u64) -> u64 {
    request
        .get(field)
        .and_then(Value::as_u64)
        .unwrap_or(default)
}
