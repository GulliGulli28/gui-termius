use termius_core::model::{AuthMethod, Host, HostId, Workspace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    Password,
    PrivateKey,
    Agent,
}

impl std::fmt::Display for AuthKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AuthKind::Password => "Mot de passe",
            AuthKind::PrivateKey => "Clé privée",
            AuthKind::Agent => "Agent SSH",
        })
    }
}

pub const AUTH_KINDS: [AuthKind; 3] = [AuthKind::Agent, AuthKind::Password, AuthKind::PrivateKey];

/// A selectable entry in the bastion/jump-host picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpChoice {
    pub id: Option<HostId>,
    pub label: String,
}

impl std::fmt::Display for JumpChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Debug, Clone)]
pub enum FormField {
    Label(String),
    Address(String),
    Port(String),
    Username(String),
    AuthKind(AuthKind),
    KeyPath(String),
    Password(String),
    JumpVia(JumpChoice),
}

pub struct HostForm {
    pub editing: Option<HostId>,
    pub label: String,
    pub address: String,
    pub port: String,
    pub username: String,
    pub auth_kind: AuthKind,
    pub key_path: String,
    pub password: String,
    pub jump_via: Option<HostId>,
}

impl HostForm {
    pub fn new() -> Self {
        Self {
            editing: None,
            label: String::new(),
            address: String::new(),
            port: "22".to_string(),
            username: String::new(),
            auth_kind: AuthKind::Agent,
            key_path: String::new(),
            password: String::new(),
            jump_via: None,
        }
    }

    pub fn from_host(host: &Host) -> Self {
        let (auth_kind, key_path) = match &host.auth {
            AuthMethod::Password => (AuthKind::Password, String::new()),
            AuthMethod::PrivateKey { path } => (AuthKind::PrivateKey, path.clone()),
            AuthMethod::Agent => (AuthKind::Agent, String::new()),
        };
        Self {
            editing: Some(host.id),
            label: host.label.clone(),
            address: host.address.clone(),
            port: host.port.to_string(),
            username: host.username.clone(),
            auth_kind,
            key_path,
            password: String::new(),
            jump_via: host.jump_via,
        }
    }

    pub fn apply(&mut self, field: FormField) {
        match field {
            FormField::Label(v) => self.label = v,
            FormField::Address(v) => self.address = v,
            FormField::Port(v) => self.port = v,
            FormField::Username(v) => self.username = v,
            FormField::AuthKind(v) => self.auth_kind = v,
            FormField::KeyPath(v) => self.key_path = v,
            FormField::Password(v) => self.password = v,
            FormField::JumpVia(choice) => self.jump_via = choice.id,
        }
    }

    /// Jump-host candidates: every host except this one and any host that
    /// (transitively) jumps through this one, which would create a cycle.
    pub fn jump_choices(&self, workspace: &Workspace) -> Vec<JumpChoice> {
        let mut choices = vec![JumpChoice { id: None, label: "(direct, sans bastion)".to_string() }];
        for host in &workspace.hosts {
            if Some(host.id) == self.editing {
                continue;
            }
            if let Some(self_id) = self.editing {
                if workspace.jump_chain(host.id).map(|chain| chain.iter().any(|h| h.id == self_id)).unwrap_or(false) {
                    continue;
                }
            }
            choices.push(JumpChoice { id: Some(host.id), label: host.label.clone() });
        }
        choices
    }

    pub fn selected_jump_choice(&self, workspace: &Workspace) -> JumpChoice {
        match self.jump_via {
            None => JumpChoice { id: None, label: "(direct, sans bastion)".to_string() },
            Some(id) => JumpChoice {
                id: Some(id),
                label: workspace.host(id).map(|h| h.label.clone()).unwrap_or_else(|| "?".to_string()),
            },
        }
    }

    pub fn validate(&self) -> Result<(u16, AuthMethod), String> {
        if self.label.trim().is_empty() {
            return Err("Le nom est requis".to_string());
        }
        if self.address.trim().is_empty() {
            return Err("L'adresse est requise".to_string());
        }
        if self.username.trim().is_empty() {
            return Err("L'utilisateur est requis".to_string());
        }
        let port: u16 = self.port.trim().parse().map_err(|_| "Port invalide".to_string())?;
        let auth = match self.auth_kind {
            AuthKind::Password => AuthMethod::Password,
            AuthKind::Agent => AuthMethod::Agent,
            AuthKind::PrivateKey => {
                if self.key_path.trim().is_empty() {
                    return Err("Le chemin de la clé privée est requis".to_string());
                }
                AuthMethod::PrivateKey { path: self.key_path.clone() }
            },
        };
        Ok((port, auth))
    }
}
