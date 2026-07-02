use crate::host_form::{AuthKind, FormField, HostForm, AUTH_KINDS};
use crate::ssh_bridge::{self, ConnectOutcome, CopyOutcome, ForwardOutcome, ListOutcome, OneShot, PaneOutcome, PaneRef, PaneSource, TabSession};
use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length, Subscription, Task, Theme};
use std::collections::HashMap;
use std::sync::Arc;
use termius_core::model::{AuthMethod, Host, HostId, PortForward, PortForwardId, PortForwardKind, Snippet, SnippetId, Workspace};
use termius_core::sftp::{Entry as SftpEntry, SftpClient};
use termius_core::ssh::Connection;
use termius_core::vault::SecretKind;
use termius_core::{store, vault};
use termius_term::actions::Action;
use termius_term::TerminalView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    Hosts,
    Snippets,
    Tunnels,
}

pub struct App {
    workspace: Workspace,
    form: Option<HostForm>,
    tabs: Vec<Tab>,
    active_tab: Option<u64>,
    next_tab_id: u64,
    status: Option<String>,
    sidebar_panel: SidebarPanel,
    snippet_form: SnippetForm,
    forward_form: ForwardForm,
    running_forwards: HashMap<PortForwardId, ssh_bridge::ForwardSession>,
}

struct Tab {
    id: u64,
    #[allow(dead_code)]
    host_id: HostId,
    label: String,
    kind: TabKind,
}

enum TabKind {
    Connecting,
    Terminal(TabSession),
    Transfer { left: Pane, right: Pane },
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSide {
    Left,
    Right,
}

impl PaneSide {
    fn other(self) -> PaneSide {
        match self {
            PaneSide::Left => PaneSide::Right,
            PaneSide::Right => PaneSide::Left,
        }
    }
}

/// One side of a [`TabKind::Transfer`]: a local or remote file tree, plus
/// the transient UI state (path bar, in-flight flag) for that side.
struct Pane {
    source: PaneSource,
    state: PaneState,
    goto_path: String,
    busy: bool,
}

impl Pane {
    fn new(source: PaneSource) -> Self {
        Self { source, state: PaneState::Connecting, goto_path: String::new(), busy: false }
    }

    fn cwd_and_entries(&self) -> Option<(&str, &[SftpEntry])> {
        match &self.state {
            PaneState::Open { cwd, entries, .. } => Some((cwd, entries)),
            _ => None,
        }
    }

    /// A reference usable to act on this pane's filesystem (for copy/list),
    /// `None` while it's still connecting or failed.
    fn pane_ref(&self) -> Option<PaneRef> {
        match &self.state {
            PaneState::Open { client: Some(client), .. } => Some(PaneRef::Remote(client.clone())),
            PaneState::Open { client: None, .. } => Some(PaneRef::Local),
            _ => None,
        }
    }
}

enum PaneState {
    Connecting,
    Open {
        /// Kept alive for as long as the pane is open; `None` for a local pane.
        #[allow(dead_code)]
        connection: Option<Connection>,
        client: Option<Arc<SftpClient>>,
        cwd: String,
        entries: Vec<SftpEntry>,
    },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneChoice {
    source: PaneSource,
    label: String,
}

impl std::fmt::Display for PaneChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Default)]
struct SnippetForm {
    name: String,
    command: String,
}

struct ForwardForm {
    host_id: Option<HostId>,
    kind: PortForwardKind,
    bind_address: String,
    bind_port: String,
    dest_address: String,
    dest_port: String,
}

impl Default for ForwardForm {
    fn default() -> Self {
        Self {
            host_id: None,
            kind: PortForwardKind::Local,
            bind_address: "127.0.0.1".to_string(),
            bind_port: String::new(),
            dest_address: "127.0.0.1".to_string(),
            dest_port: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostChoice {
    id: HostId,
    label: String,
}

impl std::fmt::Display for HostChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

const FORWARD_KINDS: [PortForwardKind; 2] = [PortForwardKind::Local, PortForwardKind::Remote];

#[derive(Clone)]
pub enum Message {
    SidebarPanel(SidebarPanel),

    // Hosts
    NewHost,
    EditHost(HostId),
    DeleteHost(HostId),
    Form(FormField),
    SaveHost,
    CancelForm,

    // Terminal tabs
    Connect(HostId),
    Connected(u64, Arc<OneShot<ConnectOutcome>>),
    TerminalEvent(termius_term::Event),

    // Transfer tabs (dual-pane file browser: local and/or remote trees)
    OpenTransfer(HostId),
    PaneSourceChanged(u64, PaneSide, PaneChoice),
    PaneOpened(u64, PaneSide, Arc<OneShot<PaneOutcome>>),
    PaneNavigate(u64, PaneSide, String),
    PaneGotoChanged(u64, PaneSide, String),
    PaneUp(u64, PaneSide),
    PaneActivate(u64, PaneSide, SftpEntry),
    PaneListed(u64, PaneSide, Arc<OneShot<ListOutcome>>),
    PaneCopy(u64, PaneSide, SftpEntry),
    PaneCopyDone(u64, PaneSide, Arc<OneShot<CopyOutcome>>),

    // Tabs (generic)
    SelectTab(u64),
    CloseTab(u64),

    // Snippets
    SnippetNameChanged(String),
    SnippetCommandChanged(String),
    AddSnippet,
    DeleteSnippet(SnippetId),
    RunSnippet(SnippetId),

    // Port forwarding
    ForwardHostChanged(HostChoice),
    ForwardKindChanged(PortForwardKind),
    ForwardBindAddressChanged(String),
    ForwardBindPortChanged(String),
    ForwardDestAddressChanged(String),
    ForwardDestPortChanged(String),
    AddForward,
    DeleteForward(PortForwardId),
    StartForward(PortForwardId),
    ForwardStarted(PortForwardId, Arc<OneShot<ForwardOutcome>>),
    StopForward(PortForwardId),

    DismissStatus,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let workspace = store::load().unwrap_or_default();
        (
            Self {
                workspace,
                form: None,
                tabs: Vec::new(),
                active_tab: None,
                next_tab_id: 0,
                status: None,
                sidebar_panel: SidebarPanel::Hosts,
                snippet_form: SnippetForm::default(),
                forward_form: ForwardForm::default(),
                running_forwards: HashMap::new(),
            },
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        "gui-termius".to_string()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(self.tabs.iter().filter_map(|tab| match &tab.kind {
            TabKind::Terminal(session) => Some(session.terminal.subscription().map(Message::TerminalEvent)),
            _ => None,
        }))
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        id
    }

    fn tab_index(&self, id: u64) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SidebarPanel(panel) => {
                self.sidebar_panel = panel;
                Task::none()
            },
            Message::NewHost => {
                self.form = Some(HostForm::new());
                Task::none()
            },
            Message::EditHost(id) => {
                if let Some(host) = self.workspace.host(id) {
                    self.form = Some(HostForm::from_host(host));
                }
                Task::none()
            },
            Message::DeleteHost(id) => {
                self.workspace.hosts.retain(|h| h.id != id);
                for host in &mut self.workspace.hosts {
                    if host.jump_via == Some(id) {
                        host.jump_via = None;
                    }
                }
                let _ = vault::delete(id, SecretKind::Password);
                let _ = vault::delete(id, SecretKind::KeyPassphrase);
                self.persist();
                Task::none()
            },
            Message::Form(field) => {
                if let Some(form) = &mut self.form {
                    form.apply(field);
                }
                Task::none()
            },
            Message::SaveHost => {
                self.save_form();
                Task::none()
            },
            Message::CancelForm => {
                self.form = None;
                Task::none()
            },
            Message::Connect(host_id) => self.start_connect(host_id),
            Message::Connected(tab_id, slot) => self.handle_connected(tab_id, slot),
            Message::TerminalEvent(termius_term::Event::BackendCall(id, cmd)) => self.handle_terminal_event(id, cmd),
            Message::OpenTransfer(host_id) => self.start_transfer(host_id),
            Message::PaneSourceChanged(tab_id, side, choice) => self.change_pane_source(tab_id, side, choice.source),
            Message::PaneOpened(tab_id, side, slot) => self.handle_pane_opened(tab_id, side, slot),
            Message::PaneNavigate(tab_id, side, path) => self.pane_navigate(tab_id, side, path),
            Message::PaneGotoChanged(tab_id, side, value) => {
                self.with_pane(tab_id, side, |pane| pane.goto_path = value);
                Task::none()
            },
            Message::PaneUp(tab_id, side) => {
                let Some(pane) = self.pane(tab_id, side) else { return Task::none() };
                let Some((cwd, _)) = pane.cwd_and_entries() else { return Task::none() };
                let parent = termius_core::sftp::join(cwd, "..");
                self.pane_navigate(tab_id, side, parent)
            },
            Message::PaneActivate(tab_id, side, entry) => {
                if !entry.is_dir {
                    return Task::none();
                }
                let Some(pane) = self.pane(tab_id, side) else { return Task::none() };
                let Some((cwd, _)) = pane.cwd_and_entries() else { return Task::none() };
                let path = termius_core::sftp::join(cwd, &entry.name);
                self.pane_navigate(tab_id, side, path)
            },
            Message::PaneListed(tab_id, side, slot) => self.handle_pane_listed(tab_id, side, slot),
            Message::PaneCopy(tab_id, side, entry) => self.pane_copy(tab_id, side, entry),
            Message::PaneCopyDone(tab_id, side, slot) => self.handle_pane_copy_done(tab_id, side, slot),
            Message::SelectTab(id) => {
                self.active_tab = Some(id);
                Task::none()
            },
            Message::CloseTab(id) => {
                if let Some(idx) = self.tab_index(id) {
                    self.tabs.remove(idx);
                }
                self.active_tab = match (self.active_tab, self.tabs.last()) {
                    (Some(active), _) if active == id => self.tabs.last().map(|t| t.id),
                    (active, _) => active,
                };
                Task::none()
            },
            Message::SnippetNameChanged(value) => {
                self.snippet_form.name = value;
                Task::none()
            },
            Message::SnippetCommandChanged(value) => {
                self.snippet_form.command = value;
                Task::none()
            },
            Message::AddSnippet => {
                if !self.snippet_form.name.trim().is_empty() && !self.snippet_form.command.trim().is_empty() {
                    self.workspace.snippets.push(Snippet {
                        id: SnippetId::new_v4(),
                        name: std::mem::take(&mut self.snippet_form.name),
                        command: std::mem::take(&mut self.snippet_form.command),
                        tags: Vec::new(),
                    });
                    self.persist();
                }
                Task::none()
            },
            Message::DeleteSnippet(id) => {
                self.workspace.snippets.retain(|s| s.id != id);
                self.persist();
                Task::none()
            },
            Message::RunSnippet(id) => {
                self.run_snippet(id);
                Task::none()
            },
            Message::ForwardHostChanged(choice) => {
                self.forward_form.host_id = Some(choice.id);
                Task::none()
            },
            Message::ForwardKindChanged(kind) => {
                self.forward_form.kind = kind;
                Task::none()
            },
            Message::ForwardBindAddressChanged(value) => {
                self.forward_form.bind_address = value;
                Task::none()
            },
            Message::ForwardBindPortChanged(value) => {
                self.forward_form.bind_port = value;
                Task::none()
            },
            Message::ForwardDestAddressChanged(value) => {
                self.forward_form.dest_address = value;
                Task::none()
            },
            Message::ForwardDestPortChanged(value) => {
                self.forward_form.dest_port = value;
                Task::none()
            },
            Message::AddForward => {
                self.add_forward();
                Task::none()
            },
            Message::DeleteForward(id) => {
                self.workspace.port_forwards.retain(|f| f.id != id);
                self.persist();
                if let Some(session) = self.running_forwards.remove(&id) {
                    return Task::perform(ssh_bridge::stop_forward(session), |()| Message::DismissStatus);
                }
                Task::none()
            },
            Message::StartForward(id) => self.start_forward(id),
            Message::ForwardStarted(id, slot) => self.handle_forward_started(id, slot),
            Message::StopForward(id) => {
                if let Some(session) = self.running_forwards.remove(&id) {
                    return Task::perform(ssh_bridge::stop_forward(session), |()| Message::DismissStatus);
                }
                Task::none()
            },
            Message::DismissStatus => {
                self.status = None;
                Task::none()
            },
        }
    }

    fn with_pane(&mut self, tab_id: u64, side: PaneSide, f: impl FnOnce(&mut Pane)) {
        if let Some(idx) = self.tab_index(tab_id) {
            if let TabKind::Transfer { left, right } = &mut self.tabs[idx].kind {
                f(match side {
                    PaneSide::Left => left,
                    PaneSide::Right => right,
                });
            }
        }
    }

    fn pane(&self, tab_id: u64, side: PaneSide) -> Option<&Pane> {
        let idx = self.tab_index(tab_id)?;
        let TabKind::Transfer { left, right } = &self.tabs[idx].kind else { return None };
        Some(match side {
            PaneSide::Left => left,
            PaneSide::Right => right,
        })
    }

    fn start_connect(&mut self, host_id: HostId) -> Task<Message> {
        let label = match self.workspace.host(host_id) {
            Some(host) => host.label.clone(),
            None => return Task::none(),
        };
        let tab_id = self.next_id();
        self.tabs.push(Tab { id: tab_id, host_id, label, kind: TabKind::Connecting });
        self.active_tab = Some(tab_id);

        let workspace = self.workspace.clone();
        Task::perform(ssh_bridge::connect_and_build_terminal(workspace, host_id, tab_id), move |outcome| {
            Message::Connected(tab_id, OneShot::new(outcome))
        })
    }

    fn handle_connected(&mut self, tab_id: u64, slot: Arc<OneShot<ConnectOutcome>>) -> Task<Message> {
        let Some(outcome) = slot.take() else { return Task::none() };
        let Some(idx) = self.tab_index(tab_id) else { return Task::none() };
        match outcome {
            ConnectOutcome::Success(session) => {
                let widget_id = session.terminal.widget_id().clone();
                self.tabs[idx].kind = TabKind::Terminal(*session);
                self.active_tab = Some(tab_id);
                TerminalView::focus::<Message>(widget_id)
            },
            ConnectOutcome::Failure(error) => {
                let label = self.tabs[idx].label.clone();
                self.status = Some(format!("Connexion à '{label}' échouée : {error}"));
                self.tabs[idx].kind = TabKind::Failed(error);
                Task::none()
            },
        }
    }

    fn handle_terminal_event(&mut self, id: u64, cmd: termius_term::BackendCommand) -> Task<Message> {
        if let Some(idx) = self.tab_index(id) {
            if let TabKind::Terminal(session) = &mut self.tabs[idx].kind {
                match session.terminal.handle(termius_term::Command::ProxyToBackend(cmd)) {
                    Action::ChangeTitle(title) => self.tabs[idx].label = title,
                    Action::Shutdown | Action::Ignore => {},
                }
            }
        }
        Task::none()
    }

    fn start_transfer(&mut self, host_id: HostId) -> Task<Message> {
        let label = match self.workspace.host(host_id) {
            Some(host) => format!("Transfert : {}", host.label),
            None => return Task::none(),
        };
        let tab_id = self.next_id();
        self.tabs.push(Tab {
            id: tab_id,
            host_id,
            label,
            kind: TabKind::Transfer { left: Pane::new(PaneSource::Local), right: Pane::new(PaneSource::Remote(host_id)) },
        });
        self.active_tab = Some(tab_id);

        Task::batch([
            self.open_pane_task(tab_id, PaneSide::Left, PaneSource::Local),
            self.open_pane_task(tab_id, PaneSide::Right, PaneSource::Remote(host_id)),
        ])
    }

    fn open_pane_task(&self, tab_id: u64, side: PaneSide, source: PaneSource) -> Task<Message> {
        let workspace = self.workspace.clone();
        Task::perform(ssh_bridge::open_pane(source, workspace), move |outcome| Message::PaneOpened(tab_id, side, OneShot::new(outcome)))
    }

    fn change_pane_source(&mut self, tab_id: u64, side: PaneSide, source: PaneSource) -> Task<Message> {
        let source_for_task = source.clone();
        self.with_pane(tab_id, side, move |pane| {
            pane.source = source;
            pane.state = PaneState::Connecting;
            pane.goto_path.clear();
        });
        self.open_pane_task(tab_id, side, source_for_task)
    }

    fn handle_pane_opened(&mut self, tab_id: u64, side: PaneSide, slot: Arc<OneShot<PaneOutcome>>) -> Task<Message> {
        let Some(outcome) = slot.take() else { return Task::none() };
        match outcome {
            PaneOutcome::Local { cwd, entries } => {
                self.with_pane(tab_id, side, move |pane| pane.state = PaneState::Open { connection: None, client: None, cwd, entries });
            },
            PaneOutcome::Remote { connection, client, cwd, entries } => {
                self.with_pane(tab_id, side, move |pane| {
                    pane.state = PaneState::Open { connection: Some(connection), client: Some(client), cwd, entries }
                });
            },
            PaneOutcome::Failure(error) => {
                self.with_pane(tab_id, side, |pane| pane.state = PaneState::Failed(error.clone()));
                self.status = Some(format!("Erreur : {error}"));
            },
        }
        Task::none()
    }

    fn pane_navigate(&mut self, tab_id: u64, side: PaneSide, path: String) -> Task<Message> {
        let pane_ref = {
            let Some(pane) = self.pane(tab_id, side) else { return Task::none() };
            if pane.busy {
                return Task::none();
            }
            let Some(pane_ref) = pane.pane_ref() else { return Task::none() };
            pane_ref
        };
        self.with_pane(tab_id, side, |p| p.busy = true);
        match pane_ref {
            PaneRef::Local => Task::perform(ssh_bridge::list_local(path), move |outcome| Message::PaneListed(tab_id, side, OneShot::new(outcome))),
            PaneRef::Remote(client) => {
                Task::perform(ssh_bridge::list_remote(client, path), move |outcome| Message::PaneListed(tab_id, side, OneShot::new(outcome)))
            },
        }
    }

    fn handle_pane_listed(&mut self, tab_id: u64, side: PaneSide, slot: Arc<OneShot<ListOutcome>>) -> Task<Message> {
        let Some(outcome) = slot.take() else { return Task::none() };
        match outcome {
            ListOutcome::Success { cwd, entries } => {
                self.with_pane(tab_id, side, move |pane| {
                    pane.busy = false;
                    if let PaneState::Open { cwd: c, entries: e, .. } = &mut pane.state {
                        *c = cwd;
                        *e = entries;
                    }
                });
            },
            ListOutcome::Failure(error) => {
                self.with_pane(tab_id, side, |pane| pane.busy = false);
                self.status = Some(format!("Erreur : {error}"));
            },
        }
        Task::none()
    }

    fn pane_copy(&mut self, tab_id: u64, side: PaneSide, entry: SftpEntry) -> Task<Message> {
        let dest_side = side.other();

        let (source_ref, source_cwd) = {
            let Some(pane) = self.pane(tab_id, side) else { return Task::none() };
            if pane.busy {
                return Task::none();
            }
            let Some(pane_ref) = pane.pane_ref() else { return Task::none() };
            let Some((cwd, _)) = pane.cwd_and_entries() else { return Task::none() };
            (pane_ref, cwd.to_string())
        };
        let (dest_ref, dest_cwd) = {
            let Some(pane) = self.pane(tab_id, dest_side) else { return Task::none() };
            if pane.busy {
                return Task::none();
            }
            let Some(pane_ref) = pane.pane_ref() else { return Task::none() };
            let Some((cwd, _)) = pane.cwd_and_entries() else { return Task::none() };
            (pane_ref, cwd.to_string())
        };

        self.with_pane(tab_id, dest_side, |pane| pane.busy = true);
        Task::perform(ssh_bridge::copy_entry(source_ref, source_cwd, entry, dest_ref, dest_cwd), move |outcome| {
            Message::PaneCopyDone(tab_id, dest_side, OneShot::new(outcome))
        })
    }

    fn handle_pane_copy_done(&mut self, tab_id: u64, dest_side: PaneSide, slot: Arc<OneShot<CopyOutcome>>) -> Task<Message> {
        let Some(outcome) = slot.take() else { return Task::none() };
        match outcome {
            CopyOutcome::Success => {
                let cwd = {
                    let Some(pane) = self.pane(tab_id, dest_side) else { return Task::none() };
                    let Some((cwd, _)) = pane.cwd_and_entries() else { return Task::none() };
                    cwd.to_string()
                };
                self.with_pane(tab_id, dest_side, |pane| pane.busy = false);
                self.pane_navigate(tab_id, dest_side, cwd)
            },
            CopyOutcome::Failure(error) => {
                self.with_pane(tab_id, dest_side, |pane| pane.busy = false);
                self.status = Some(format!("Erreur de copie : {error}"));
                Task::none()
            },
        }
    }

    fn run_snippet(&mut self, id: SnippetId) {
        let Some(snippet) = self.workspace.snippets.iter().find(|s| s.id == id) else { return };
        let command = snippet.command.clone();
        let Some(active_id) = self.active_tab else {
            self.status = Some("Aucun terminal actif pour exécuter ce snippet".to_string());
            return;
        };
        let Some(idx) = self.tab_index(active_id) else { return };
        let TabKind::Terminal(session) = &mut self.tabs[idx].kind else {
            self.status = Some("L'onglet actif n'est pas un terminal".to_string());
            return;
        };
        let mut bytes = command.into_bytes();
        bytes.push(b'\r');
        session.terminal.handle(termius_term::Command::ProxyToBackend(termius_term::BackendCommand::Write(bytes)));
    }

    fn add_forward(&mut self) {
        let Some(host_id) = self.forward_form.host_id else {
            self.status = Some("Choisissez un hôte pour le tunnel".to_string());
            return;
        };
        let (Ok(bind_port), Ok(dest_port)) = (self.forward_form.bind_port.trim().parse::<u16>(), self.forward_form.dest_port.trim().parse::<u16>()) else {
            self.status = Some("Ports invalides".to_string());
            return;
        };
        if self.forward_form.bind_address.trim().is_empty() || self.forward_form.dest_address.trim().is_empty() {
            self.status = Some("Adresses invalides".to_string());
            return;
        }
        self.workspace.port_forwards.push(PortForward {
            id: PortForwardId::new_v4(),
            host_id,
            kind: self.forward_form.kind,
            bind_address: self.forward_form.bind_address.clone(),
            bind_port,
            dest_address: self.forward_form.dest_address.clone(),
            dest_port,
        });
        self.persist();
    }

    fn start_forward(&mut self, id: PortForwardId) -> Task<Message> {
        let Some(forward) = self.workspace.port_forwards.iter().find(|f| f.id == id).cloned() else { return Task::none() };
        if self.running_forwards.contains_key(&id) {
            return Task::none();
        }
        let workspace = self.workspace.clone();
        Task::perform(ssh_bridge::connect_and_start_forward(workspace, forward), move |outcome| Message::ForwardStarted(id, OneShot::new(outcome)))
    }

    fn handle_forward_started(&mut self, id: PortForwardId, slot: Arc<OneShot<ForwardOutcome>>) -> Task<Message> {
        let Some(outcome) = slot.take() else { return Task::none() };
        match outcome {
            ForwardOutcome::Success(session) => {
                self.running_forwards.insert(id, *session);
            },
            ForwardOutcome::Failure(error) => self.status = Some(format!("Échec du tunnel : {error}")),
        }
        Task::none()
    }

    fn save_form(&mut self) {
        let Some(form) = self.form.take() else { return };
        let (port, auth) = match form.validate() {
            Ok(parsed) => parsed,
            Err(error) => {
                self.status = Some(error);
                self.form = Some(form);
                return;
            },
        };

        let host_id = match form.editing {
            Some(id) => {
                if let Some(host) = self.workspace.hosts.iter_mut().find(|h| h.id == id) {
                    host.label = form.label.clone();
                    host.address = form.address.clone();
                    host.port = port;
                    host.username = form.username.clone();
                    host.auth = auth.clone();
                    host.jump_via = form.jump_via;
                }
                id
            },
            None => {
                let mut host = Host::new(form.label.clone(), form.address.clone(), form.username.clone());
                host.port = port;
                host.auth = auth.clone();
                host.jump_via = form.jump_via;
                let id = host.id;
                self.workspace.hosts.push(host);
                id
            },
        };

        match &auth {
            AuthMethod::Password if !form.password.is_empty() => {
                let _ = vault::store(host_id, SecretKind::Password, &form.password);
            },
            AuthMethod::PrivateKey { .. } if !form.password.is_empty() => {
                let _ = vault::store(host_id, SecretKind::KeyPassphrase, &form.password);
            },
            _ => {},
        }

        self.persist();
    }

    fn persist(&mut self) {
        if let Err(err) = store::save(&self.workspace) {
            self.status = Some(format!("Échec de sauvegarde : {err}"));
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();
        let main: Element<'_, Message> = if let Some(form) = &self.form { self.view_form(form) } else { self.view_tabs() };
        let body: Element<'_, Message> = row![sidebar, main].height(Length::Fill).width(Length::Fill).into();

        match &self.status {
            Some(message) => column![
                row![text(message.clone()), button("X").on_press(Message::DismissStatus)].spacing(8).padding(8),
                body
            ]
            .height(Length::Fill)
            .width(Length::Fill)
            .into(),
            None => body,
        }
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let switcher = row![
            button("Hôtes").on_press(Message::SidebarPanel(SidebarPanel::Hosts)).width(Length::FillPortion(1)),
            button("Snippets").on_press(Message::SidebarPanel(SidebarPanel::Snippets)).width(Length::FillPortion(1)),
            button("Tunnels").on_press(Message::SidebarPanel(SidebarPanel::Tunnels)).width(Length::FillPortion(1)),
        ]
        .spacing(4);

        let panel: Element<'_, Message> = match self.sidebar_panel {
            SidebarPanel::Hosts => self.view_hosts_panel(),
            SidebarPanel::Snippets => self.view_snippets_panel(),
            SidebarPanel::Tunnels => self.view_tunnels_panel(),
        };

        container(column![switcher, panel].spacing(8).padding(8).width(Length::Fixed(300.0)).height(Length::Fill)).height(Length::Fill).into()
    }

    fn view_hosts_panel(&self) -> Element<'_, Message> {
        let mut list = column![].spacing(4).width(Length::Fill);
        for host in &self.workspace.hosts {
            list = list.push(self.view_host_row(host));
        }

        column![
            text("Hôtes").size(18),
            scrollable(list).height(Length::Fill).width(Length::Fill),
            button("+ Ajouter un hôte").on_press(Message::NewHost).width(Length::Fill),
        ]
        .spacing(8)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    fn view_host_row(&self, host: &Host) -> Element<'_, Message> {
        column![
            button(text(host.label.clone())).on_press(Message::Connect(host.id)).width(Length::Fill),
            row![
                button(text("Transfert").size(12)).on_press(Message::OpenTransfer(host.id)).width(Length::FillPortion(1)),
                button(text("Éditer").size(12)).on_press(Message::EditHost(host.id)).width(Length::FillPortion(1)),
                button(text("Suppr.").size(12)).on_press(Message::DeleteHost(host.id)).width(Length::FillPortion(1)),
            ]
            .spacing(4),
        ]
        .spacing(4)
        .width(Length::Fill)
        .into()
    }

    fn view_snippets_panel(&self) -> Element<'_, Message> {
        let mut list = column![].spacing(4).width(Length::Fill);
        for snippet in &self.workspace.snippets {
            list = list.push(
                column![
                    text(snippet.name.clone()),
                    text(snippet.command.clone()).size(12),
                    row![
                        button(text("Exécuter").size(12)).on_press(Message::RunSnippet(snippet.id)).width(Length::FillPortion(1)),
                        button(text("Suppr.").size(12)).on_press(Message::DeleteSnippet(snippet.id)).width(Length::FillPortion(1)),
                    ]
                    .spacing(4),
                ]
                .spacing(2)
                .width(Length::Fill),
            );
        }

        let add_form = column![
            text_input("Nom", &self.snippet_form.name).on_input(Message::SnippetNameChanged),
            text_input("Commande", &self.snippet_form.command).on_input(Message::SnippetCommandChanged),
            button("+ Ajouter un snippet").on_press(Message::AddSnippet).width(Length::Fill),
        ]
        .spacing(4);

        column![text("Snippets").size(18), scrollable(list).height(Length::Fill).width(Length::Fill), add_form]
            .spacing(8)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn view_tunnels_panel(&self) -> Element<'_, Message> {
        let mut list = column![].spacing(4).width(Length::Fill);
        for forward in &self.workspace.port_forwards {
            let host_label = self.workspace.host(forward.host_id).map(|h| h.label.as_str()).unwrap_or("?");
            let is_running = self.running_forwards.contains_key(&forward.id);
            let summary = format!(
                "{} {}:{} → {}:{} ({host_label})",
                forward.kind, forward.bind_address, forward.bind_port, forward.dest_address, forward.dest_port
            );
            let toggle = if is_running {
                button(text("Arrêter").size(12)).on_press(Message::StopForward(forward.id))
            } else {
                button(text("Démarrer").size(12)).on_press(Message::StartForward(forward.id))
            };
            list = list.push(
                column![
                    text(summary).size(12),
                    row![toggle.width(Length::FillPortion(1)), button(text("Suppr.").size(12)).on_press(Message::DeleteForward(forward.id)).width(Length::FillPortion(1)),]
                        .spacing(4),
                ]
                .spacing(2)
                .width(Length::Fill),
            );
        }

        let host_choices: Vec<HostChoice> = self.workspace.hosts.iter().map(|h| HostChoice { id: h.id, label: h.label.clone() }).collect();
        let selected_host = self.forward_form.host_id.and_then(|id| host_choices.iter().find(|c| c.id == id).cloned());

        let add_form = column![
            pick_list(host_choices, selected_host, Message::ForwardHostChanged).width(Length::Fill),
            pick_list(FORWARD_KINDS, Some(self.forward_form.kind), Message::ForwardKindChanged).width(Length::Fill),
            text_input("Adresse locale", &self.forward_form.bind_address).on_input(Message::ForwardBindAddressChanged),
            text_input("Port local", &self.forward_form.bind_port).on_input(Message::ForwardBindPortChanged),
            text_input("Adresse distante", &self.forward_form.dest_address).on_input(Message::ForwardDestAddressChanged),
            text_input("Port distant", &self.forward_form.dest_port).on_input(Message::ForwardDestPortChanged),
            button("+ Ajouter un tunnel").on_press(Message::AddForward).width(Length::Fill),
        ]
        .spacing(4);

        column![text("Tunnels").size(18), scrollable(list).height(Length::Fill).width(Length::Fill), add_form]
            .spacing(8)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn view_form<'a>(&'a self, form: &'a HostForm) -> Element<'a, Message> {
        let mut fields = column![
            labeled_input("Nom", &form.label, FormField::Label),
            labeled_input("Adresse", &form.address, FormField::Address),
            labeled_input("Port", &form.port, FormField::Port),
            labeled_input("Utilisateur", &form.username, FormField::Username),
            text("Authentification"),
            pick_list(AUTH_KINDS, Some(form.auth_kind), FormField::AuthKind).width(Length::Fill),
        ]
        .spacing(10);

        fields = match form.auth_kind {
            AuthKind::Password => fields.push(labeled_input_secret("Mot de passe", &form.password, FormField::Password)),
            AuthKind::PrivateKey => fields
                .push(labeled_input("Chemin de la clé privée", &form.key_path, FormField::KeyPath))
                .push(labeled_input_secret("Passphrase (optionnelle)", &form.password, FormField::Password)),
            AuthKind::Agent => fields,
        };

        let jump_choices = form.jump_choices(&self.workspace);
        let selected_jump = form.selected_jump_choice(&self.workspace);
        fields = fields
            .push(text("Bastion / jump host"))
            .push(pick_list(jump_choices, Some(selected_jump), FormField::JumpVia).width(Length::Fill));

        let fields_element: Element<'a, Message> = Element::<'a, FormField>::from(fields).map(Message::Form);

        let content = column![
            text(if form.editing.is_some() { "Modifier l'hôte" } else { "Nouvel hôte" }).size(20),
            fields_element,
            row![button("Enregistrer").on_press(Message::SaveHost), button("Annuler").on_press(Message::CancelForm),].spacing(8),
        ]
        .spacing(10)
        .padding(16)
        .width(Length::Fixed(420.0));

        container(content).height(Length::Fill).width(Length::Fill).into()
    }

    fn view_tabs(&self) -> Element<'_, Message> {
        if self.tabs.is_empty() {
            return container(text("Sélectionnez ou ajoutez un hôte pour vous connecter"))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let mut bar = row![].spacing(4).padding(8);
        for tab in &self.tabs {
            let is_active = self.active_tab == Some(tab.id);
            let label = format!("{}{}", if is_active { "> " } else { "" }, tab.label);
            bar = bar.push(row![button(text(label)).on_press(Message::SelectTab(tab.id)), button("X").on_press(Message::CloseTab(tab.id)),]);
        }

        let content: Element<'_, Message> = match self.active_tab.and_then(|id| self.tabs.iter().find(|t| t.id == id)) {
            Some(Tab { kind: TabKind::Terminal(session), .. }) => TerminalView::show(&session.terminal).map(Message::TerminalEvent),
            Some(Tab { kind: TabKind::Transfer { left, right }, id, .. }) => self.view_transfer_tab(*id, left, right),
            Some(Tab { kind: TabKind::Connecting, label, .. }) => {
                container(text(format!("Connexion à {label}…"))).center_x(Length::Fill).center_y(Length::Fill).into()
            },
            Some(Tab { kind: TabKind::Failed(error), label, .. }) => {
                container(text(format!("Échec de connexion à {label} : {error}"))).center_x(Length::Fill).center_y(Length::Fill).into()
            },
            None => container(text("")).into(),
        };

        column![bar, container(content).height(Length::Fill).width(Length::Fill)].height(Length::Fill).width(Length::Fill).into()
    }

    /// All selectable sources for a transfer pane: the local filesystem, plus every saved host.
    fn pane_choices(&self) -> Vec<PaneChoice> {
        let mut choices = vec![PaneChoice { source: PaneSource::Local, label: "Local".to_string() }];
        choices.extend(self.workspace.hosts.iter().map(|h| PaneChoice { source: PaneSource::Remote(h.id), label: h.label.clone() }));
        choices
    }

    fn pane_choice_label(&self, source: &PaneSource) -> String {
        match source {
            PaneSource::Local => "Local".to_string(),
            PaneSource::Remote(id) => self.workspace.host(*id).map(|h| h.label.clone()).unwrap_or_else(|| "?".to_string()),
        }
    }

    fn view_transfer_tab<'a>(&'a self, tab_id: u64, left: &'a Pane, right: &'a Pane) -> Element<'a, Message> {
        row![self.view_pane(tab_id, PaneSide::Left, left), self.view_pane(tab_id, PaneSide::Right, right)]
            .spacing(1)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn view_pane<'a>(&'a self, tab_id: u64, side: PaneSide, pane: &'a Pane) -> Element<'a, Message> {
        let choices = self.pane_choices();
        let selected = PaneChoice { source: pane.source.clone(), label: self.pane_choice_label(&pane.source) };
        let source_picker =
            pick_list(choices, Some(selected), move |choice| Message::PaneSourceChanged(tab_id, side, choice)).width(Length::Fill);

        let body: Element<'a, Message> = match &pane.state {
            PaneState::Connecting => container(text("Connexion…")).center_x(Length::Fill).center_y(Length::Fill).into(),
            PaneState::Failed(error) => container(text(format!("Erreur : {error}"))).center_x(Length::Fill).center_y(Length::Fill).into(),
            PaneState::Open { cwd, entries, .. } => {
                let header = row![
                    button("↑").on_press(Message::PaneUp(tab_id, side)),
                    text(cwd.clone()),
                    text_input("Aller à...", &pane.goto_path).on_input(move |v| Message::PaneGotoChanged(tab_id, side, v)).width(Length::Fill),
                    button("Aller").on_press(Message::PaneNavigate(tab_id, side, pane.goto_path.clone())),
                ]
                .spacing(4)
                .padding(4);

                let copy_label = match side {
                    PaneSide::Left => "→",
                    PaneSide::Right => "←",
                };
                let mut list = column![].spacing(2).width(Length::Fill);
                for entry in entries {
                    let kind = if entry.is_dir { "[dir]" } else { "" };
                    let mut row_el = row![
                        button(text(format!("{kind} {}", entry.name))).on_press(Message::PaneActivate(tab_id, side, entry.clone())).width(Length::FillPortion(3)),
                        text(format!("{} o", entry.size)).width(Length::FillPortion(1)),
                    ]
                    .spacing(4);
                    if !entry.is_dir {
                        row_el = row_el.push(button(text(copy_label).size(12)).on_press(Message::PaneCopy(tab_id, side, entry.clone())));
                    }
                    list = list.push(row_el);
                }

                column![header, scrollable(list).height(Length::Fill).width(Length::Fill)].height(Length::Fill).width(Length::Fill).into()
            },
        };

        let busy_indicator: Element<'a, Message> = if pane.busy { text("…").into() } else { text("").into() };

        container(
            column![row![source_picker, busy_indicator].spacing(4).padding(4), body]
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
}

fn labeled_input<'a>(label: &'a str, value: &'a str, on_input: impl Fn(String) -> FormField + 'a) -> Element<'a, FormField> {
    column![text(label), text_input(label, value).on_input(on_input)].spacing(4).into()
}

fn labeled_input_secret<'a>(label: &'a str, value: &'a str, on_input: impl Fn(String) -> FormField + 'a) -> Element<'a, FormField> {
    column![text(label), text_input(label, value).on_input(on_input).secure(true)].spacing(4).into()
}
