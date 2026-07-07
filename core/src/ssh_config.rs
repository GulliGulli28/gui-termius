//! Minimal, read-only parser for `~/.ssh/config`-style files — just enough to
//! preview and import individual `Host` blocks into the workspace. This is not
//! a full implementation of the OpenSSH config grammar: no `Match`, `Include`,
//! multi-pattern `Host` lines, or wildcard hosts (those are skipped).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshConfigHost {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
}

/// The user's default `~/.ssh/config` path, if a home directory can be determined.
pub fn default_path() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.home_dir().join(".ssh").join("config"))
}

pub fn parse(path: &Path) -> anyhow::Result<Vec<SshConfigHost>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_str(&content))
}

fn parse_str(content: &str) -> Vec<SshConfigHost> {
    let mut hosts = Vec::new();
    let mut current: Option<SshConfigHost> = None;

    for raw_line in content.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |c: char| c.is_whitespace() || c == '=');
        let Some(keyword) = parts.next() else {
            continue;
        };
        let value = parts
            .next()
            .unwrap_or("")
            .trim_start_matches(|c: char| c.is_whitespace() || c == '=')
            .trim();

        match keyword.to_ascii_lowercase().as_str() {
            "host" => {
                if let Some(host) = current.take() {
                    hosts.push(host);
                }
                let alias = value.split_whitespace().next().unwrap_or("");
                if !alias.is_empty() && !alias.contains('*') && !alias.contains('?') {
                    current = Some(SshConfigHost {
                        alias: alias.to_string(),
                        hostname: None,
                        user: None,
                        port: None,
                        identity_file: None,
                        proxy_jump: None,
                    });
                }
            }
            "hostname" => {
                if let Some(h) = current.as_mut() {
                    h.hostname = Some(value.to_string());
                }
            }
            "user" => {
                if let Some(h) = current.as_mut() {
                    h.user = Some(value.to_string());
                }
            }
            "port" => {
                if let Some(h) = current.as_mut() {
                    h.port = value.parse().ok();
                }
            }
            "identityfile" => {
                if let Some(h) = current.as_mut() {
                    h.identity_file = Some(value.to_string());
                }
            }
            "proxyjump" => {
                if let Some(h) = current.as_mut() {
                    h.proxy_jump = Some(value.to_string());
                }
            }
            _ => {}
        }
    }
    if let Some(host) = current.take() {
        hosts.push(host);
    }
    hosts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_hosts() {
        let content = "\
# comment\n\
Host web-01\n\
    HostName 10.0.1.15\n\
    User deploy\n\
    Port 2222\n\
    IdentityFile ~/.ssh/id_ed25519\n\
\n\
Host bastion\n\
    HostName bastion.example.com\n\
    User jump\n\
\n\
Host *\n\
    ServerAliveInterval 30\n\
";
        let hosts = parse_str(content);
        assert_eq!(hosts.len(), 2, "wildcard Host * block must be skipped");

        assert_eq!(hosts[0].alias, "web-01");
        assert_eq!(hosts[0].hostname.as_deref(), Some("10.0.1.15"));
        assert_eq!(hosts[0].user.as_deref(), Some("deploy"));
        assert_eq!(hosts[0].port, Some(2222));
        assert_eq!(hosts[0].identity_file.as_deref(), Some("~/.ssh/id_ed25519"));

        assert_eq!(hosts[1].alias, "bastion");
        assert_eq!(hosts[1].hostname.as_deref(), Some("bastion.example.com"));
    }

    #[test]
    fn supports_equals_syntax() {
        let content = "Host=foo\nHostName=1.2.3.4\nUser = root\n";
        let hosts = parse_str(content);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "foo");
        assert_eq!(hosts[0].hostname.as_deref(), Some("1.2.3.4"));
        assert_eq!(hosts[0].user.as_deref(), Some("root"));
    }
}
