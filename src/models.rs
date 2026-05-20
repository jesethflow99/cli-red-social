use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub bio: String,
    pub utc_offset: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub content: String,
    pub image_path: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub post_id: i64,
    pub user_id: i64,
    pub username: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub parent_comment_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Login,
    Register,
    Timeline,
    CreatePost,
    PostDetail(i64),
    Profile(i64),
    UserSearch,
    Messages,
    Chat(i64),
    EditProfile,
    Notifications,
    PostSearch,
    HashtagView,
    HashtagTrending,
    Radio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub sender_id: i64,
    pub receiver_id: i64,
    pub sender_username: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: i64,
    pub user_id: i64,
    pub from_user_id: i64,
    pub from_username: String,
    pub notif_type: String,
    pub created_at: DateTime<Utc>,
    pub read: bool,
    pub related_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_date() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 1, 15, 10, 30, 0).unwrap()
    }

    #[test]
    fn test_user_creation() {
        let user = User {
            id: 1,
            username: "testuser".into(),
            display_name: "Test User".into(),
            bio: "Hello".into(),
            utc_offset: 0,
            created_at: make_date(),
        };
        assert_eq!(user.id, 1);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.display_name, "Test User");
        assert_eq!(user.bio, "Hello");
    }

    #[test]
    fn test_post_creation() {
        let post = Post {
            id: 1,
            user_id: 1,
            username: "testuser".into(),
            content: "Hello world".into(),
            image_path: None,
            created_at: make_date(),
        };
        assert_eq!(post.content, "Hello world");
        assert!(post.image_path.is_none());
    }

    #[test]
    fn test_post_with_image() {
        let post = Post {
            id: 2,
            user_id: 1,
            username: "testuser".into(),
            content: "Check this out".into(),
            image_path: Some("https://example.com/img.jpg".into()),
            created_at: make_date(),
        };
        assert!(post.image_path.is_some());
        assert!(post.image_path.unwrap().starts_with("https://"));
    }

    #[test]
    fn test_comment_creation() {
        let comment = Comment {
            id: 1,
            post_id: 1,
            user_id: 2,
            username: "commenter".into(),
            content: "Nice post!".into(),
            created_at: make_date(),
            parent_comment_id: None,
        };
        assert_eq!(comment.content, "Nice post!");
        assert_eq!(comment.post_id, 1);
    }

    #[test]
    fn test_message_creation() {
        let msg = Message {
            id: 1,
            sender_id: 1,
            receiver_id: 2,
            sender_username: "alice".into(),
            content: "Hey Bob!".into(),
            created_at: make_date(),
            read: false,
        };
        assert!(!msg.read);
        assert_eq!(msg.sender_username, "alice");
    }

    #[test]
    fn test_notification_creation() {
        let notif = Notification {
            id: 1,
            user_id: 1,
            from_user_id: 2,
            from_username: "bob".into(),
            notif_type: "follow".into(),
            created_at: make_date(),
            read: false,
            related_id: None,
        };
        assert_eq!(notif.notif_type, "follow");
        assert!(!notif.read);
    }

    #[test]
    fn test_screen_variants() {
        assert!(matches!(Screen::Login, Screen::Login));
        assert!(matches!(Screen::Timeline, Screen::Timeline));
        assert!(matches!(Screen::PostDetail(5), Screen::PostDetail(5)));
        assert!(matches!(Screen::Profile(3), Screen::Profile(3)));
        assert!(matches!(Screen::Chat(1), Screen::Chat(1)));
    }
}
