use std::process::Command;

const TABLE: &str = "agora_fw";
const SET: &str = "scp_allowed";

pub fn allow_scp(ip: &str) -> bool {
    if ip.is_empty() || ip == "127.0.0.1" || ip == "::1" {
        return true;
    }
    let result = Command::new("nft")
        .args(["add", "element", "inet", TABLE, SET, &format!("{{ {} }}", ip)])
        .output();
    match result {
        Ok(o) if o.status.success() => {
            tracing::info!("[firewall] SCP allowed for IP: {}", ip);
            true
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            tracing::warn!("[firewall] Failed to allow SCP for {}: {}", ip, stderr.trim());
            false
        }
        Err(e) => {
            tracing::warn!("[firewall] nft command failed: {}", e);
            false
        }
    }
}

pub fn revoke_scp(ip: &str) -> bool {
    if ip.is_empty() || ip == "127.0.0.1" || ip == "::1" {
        return true;
    }
    let result = Command::new("nft")
        .args(["delete", "element", "inet", TABLE, SET, &format!("{{ {} }}", ip)])
        .output();
    match result {
        Ok(o) if o.status.success() => {
            tracing::info!("[firewall] SCP revoked for IP: {}", ip);
            true
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            tracing::warn!("[firewall] Failed to revoke SCP for {}: {}", ip, stderr.trim());
            false
        }
        Err(e) => {
            tracing::warn!("[firewall] nft command failed: {}", e);
            false
        }
    }
}

pub fn get_client_ip(env_var: &str) -> Option<String> {
    std::env::var(env_var).ok().filter(|s| !s.is_empty())
}
