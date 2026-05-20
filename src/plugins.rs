use anyhow::Result;
use std::collections::HashSet;

pub trait ModerationPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn filter_post(&self, _user_id: i64, _content: &str) -> Result<()> { Ok(()) }
    fn filter_comment(&self, _user_id: i64, _content: &str) -> Result<()> { Ok(()) }
    fn filter_message(&self, _sender_id: i64, _receiver_id: i64, _content: &str) -> Result<()> { Ok(()) }
    fn can_register(&self, _username: &str, _display_name: &str) -> Result<()> { Ok(()) }
}

pub struct PluginRegistry {
    plugins: Vec<Box<dyn ModerationPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn register(&mut self, plugin: Box<dyn ModerationPlugin>) {
        self.plugins.push(plugin);
    }

    pub fn filter_post(&self, user_id: i64, content: &str) -> Result<()> {
        for p in &self.plugins {
            p.filter_post(user_id, content)?;
        }
        Ok(())
    }

    pub fn filter_comment(&self, user_id: i64, content: &str) -> Result<()> {
        for p in &self.plugins {
            p.filter_comment(user_id, content)?;
        }
        Ok(())
    }

    pub fn filter_message(&self, sender_id: i64, receiver_id: i64, content: &str) -> Result<()> {
        for p in &self.plugins {
            p.filter_message(sender_id, receiver_id, content)?;
        }
        Ok(())
    }

    pub fn can_register(&self, username: &str, display_name: &str) -> Result<()> {
        for p in &self.plugins {
            p.can_register(username, display_name)?;
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn load_plugins(enabled: &[String]) -> PluginRegistry {
    let mut registry = PluginRegistry::new();
    for name in enabled {
        match name.as_str() {
            "spam" => registry.register(Box::new(SpamFilter::new())),
            "profanity" => registry.register(Box::new(ProfanityFilter::new())),
            "link" => registry.register(Box::new(LinkFilter::new())),
            _ => tracing::warn!("Plugin desconocido: {}", name),
        }
    }
    registry
}

pub fn load_plugins_from_env(env_var: &str) -> PluginRegistry {
    let enabled: Vec<String> = std::env::var(env_var)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    load_plugins(&enabled)
}

pub struct SpamFilter;

impl SpamFilter {
    pub fn new() -> Self { Self }
}

impl ModerationPlugin for SpamFilter {
    fn name(&self) -> &str { "spam" }

    fn filter_post(&self, _user_id: i64, content: &str) -> Result<()> {
        if content.len() > 5000 {
            anyhow::bail!("[spam] El post excede los 5000 caracteres");
        }
        let upper_ratio = content.chars().filter(|c| c.is_uppercase()).count() as f64
            / content.chars().filter(|c| c.is_alphabetic()).count().max(1) as f64;
        if content.len() > 30 && upper_ratio > 0.8 {
            anyhow::bail!("[spam] Demasiadas mayúsculas (posible spam)");
        }
        let mut repeated = 1u32;
        let mut max_repeated = 1u32;
        let chars: Vec<char> = content.chars().collect();
        for i in 1..chars.len() {
            if chars[i] == chars[i - 1] {
                repeated += 1;
                max_repeated = max_repeated.max(repeated);
            } else {
                repeated = 1;
            }
        }
        if max_repeated > 8 {
            anyhow::bail!("[spam] Caracteres excesivamente repetidos (posible spam)");
        }
        Ok(())
    }

    fn filter_comment(&self, _user_id: i64, content: &str) -> Result<()> {
        let mut repeated = 1u32;
        let mut max_repeated = 1u32;
        let chars: Vec<char> = content.chars().collect();
        for i in 1..chars.len() {
            if chars[i] == chars[i - 1] {
                repeated += 1;
                max_repeated = max_repeated.max(repeated);
            } else {
                repeated = 1;
            }
        }
        if max_repeated > 8 {
            anyhow::bail!("[spam] Caracteres excesivamente repetidos (posible spam)");
        }
        Ok(())
    }
}

pub struct ProfanityFilter {
    blocked: HashSet<String>,
}

impl ProfanityFilter {
    pub fn new() -> Self {
        let blocked: HashSet<String> = vec![
            "spamword1", "spamword2",
        ].into_iter().map(|s| s.to_string()).collect();
        Self { blocked }
    }
}

impl ModerationPlugin for ProfanityFilter {
    fn name(&self) -> &str { "profanity" }

    fn filter_post(&self, _user_id: i64, content: &str) -> Result<()> {
        let lower = content.to_lowercase();
        for word in &self.blocked {
            if lower.contains(word) {
                anyhow::bail!("[profanity] El contenido contiene lenguaje no permitido");
            }
        }
        Ok(())
    }

    fn filter_comment(&self, _user_id: i64, content: &str) -> Result<()> {
        let lower = content.to_lowercase();
        for word in &self.blocked {
            if lower.contains(word) {
                anyhow::bail!("[profanity] El comentario contiene lenguaje no permitido");
            }
        }
        Ok(())
    }

    fn filter_message(&self, _sender_id: i64, _receiver_id: i64, content: &str) -> Result<()> {
        let lower = content.to_lowercase();
        for word in &self.blocked {
            if lower.contains(word) {
                anyhow::bail!("[profanity] El mensaje contiene lenguaje no permitido");
            }
        }
        Ok(())
    }
}

pub struct LinkFilter;

impl LinkFilter {
    pub fn new() -> Self { Self }
}

impl ModerationPlugin for LinkFilter {
    fn name(&self) -> &str { "link" }

    fn filter_post(&self, _user_id: i64, content: &str) -> Result<()> {
        let lower = content.to_lowercase();
        let suspicious = [
            ".ru/", ".cn/", "bit.ly/", "tinyurl.com/", "short.link/",
            "free-", "click-here", "act-now", "limited-offer",
        ];
        for s in &suspicious {
            if lower.contains(s) {
                anyhow::bail!("[link] El contenido contiene enlaces sospechosos");
            }
        }
        Ok(())
    }
}
