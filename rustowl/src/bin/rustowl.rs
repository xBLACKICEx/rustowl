//! # RustOwl cargo-owlsp
//!
//! An LSP server for visualizing ownership and lifetimes in Rust, designed for debugging and optimization.

use mktemp::Temp;
use rustowl::models::*;
use rustowl::utils;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process,
    sync::RwLock,
    task::JoinSet,
};
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[allow(unused)]
#[derive(serde::Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Deco<R = Range> {
    Lifetime {
        local: Local,
        range: R,
    },
    ImmBorrow {
        local: Local,
        range: R,
    },
    MutBorrow {
        local: Local,
        range: R,
    },
    Move {
        local: Local,
        range: R,
    },
    Call {
        local: Local,
        range: R,
    },
    Outlive {
        local: Local,
        range: R,
    },

    Message {
        local: Local,
        range: R,
        message: String,
    },
}
impl Deco<Range> {
    fn to_lsp_range(&self, s: &str) -> Deco<lsp_types::Range> {
        fn get_start_end(s: &str, range: Range) -> (lsp_types::Position, lsp_types::Position) {
            let start = utils::index_to_line_char(s, range.from.0);
            let end = utils::index_to_line_char(s, range.until.0);
            let start = lsp_types::Position {
                line: start.0,
                character: start.1,
            };
            let end = lsp_types::Position {
                line: end.0,
                character: end.1,
            };
            (start, end)
        }

        match self.clone() {
            Deco::Lifetime { local, range } => {
                let (start, end) = get_start_end(s, range);
                Deco::Lifetime {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::ImmBorrow { local, range } => {
                let (start, end) = get_start_end(s, range);
                Deco::ImmBorrow {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::MutBorrow { local, range } => {
                let (start, end) = get_start_end(s, range);
                Deco::MutBorrow {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::Move { local, range } => {
                let (start, end) = get_start_end(s, range);
                Deco::Move {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::Call { local, range } => {
                let (start, end) = get_start_end(s, range);
                Deco::Call {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::Outlive { local, range } => {
                let (start, end) = get_start_end(s, range);
                Deco::Outlive {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::Message {
                local,
                range,
                message,
            } => {
                let (start, end) = get_start_end(s, range);
                Deco::Message {
                    local,
                    range: lsp_types::Range { start, end },
                    message,
                }
            }
        }
    }
}
#[derive(serde::Serialize, Clone, Debug)]
struct Decorations {
    is_analyzed: bool,
    path: Option<PathBuf>,
    decorations: Vec<Deco<lsp_types::Range>>,
}
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
struct CursorRequest {
    position: lsp_types::Position,
    document: lsp_types::TextDocumentIdentifier,
}

#[derive(Clone, Debug)]
struct SelectLocal {
    pos: Loc,
    selected: Vec<Local>,
    current_fn_id: u32,
}
impl SelectLocal {
    fn new(pos: Loc) -> Self {
        Self {
            pos,
            selected: Vec::new(),
            current_fn_id: 0,
        }
    }
    fn select(&mut self, local_id: u32, range: Range) {
        if range.from <= self.pos && self.pos <= range.until {
            self.selected.push(Local::new(local_id, self.current_fn_id));
        }
    }
}
impl utils::MirVisitor for SelectLocal {
    fn visit_decl(&mut self, decl: &MirDecl) {
        if let MirDecl::User {
            local_index,
            fn_id,
            span,
            ..
        } = decl
        {
            self.current_fn_id = *fn_id;
            self.select(*local_index, *span);
        }
    }
    fn visit_stmt(&mut self, stmt: &MirStatement) {
        if let MirStatement::Assign { rval, .. } = stmt {
            match rval {
                Some(MirRval::Move {
                    target_local_index,
                    range,
                }) => {
                    self.select(*target_local_index, *range);
                }
                Some(MirRval::Borrow {
                    target_local_index,
                    range,
                    ..
                }) => {
                    self.select(*target_local_index, *range);
                }
                _ => {}
            }
        }
    }
    fn visit_term(&mut self, term: &MirTerminator) {
        if let MirTerminator::Call {
            destination_local_index,
            fn_span,
        } = term
        {
            self.select(*destination_local_index, *fn_span);
        }
    }
}
#[derive(Clone, Debug)]
struct CalcDecos {
    locals: Vec<Local>,
    decorations: Vec<Deco>,
    current_fn_id: u32,
}
impl CalcDecos {
    pub fn new(locals: Vec<Local>) -> Self {
        Self {
            locals,
            decorations: Vec::new(),
            current_fn_id: 0,
        }
    }

    fn get_deco_order(deco: &Deco) -> u8 {
        match deco {
            Deco::Lifetime { .. } => 0,
            Deco::ImmBorrow { .. } => 1,
            Deco::MutBorrow { .. } => 2,
            Deco::Move { .. } => 3,
            Deco::Call { .. } => 4,
            Deco::Outlive { .. } => 5,
            Deco::Message { .. } => 6,
        }
    }

    fn sort_by_definition(&mut self) {
        self.decorations.sort_by_key(Self::get_deco_order);
    }

    fn handle_overlapping(&mut self) {
        self.sort_by_definition();
        let mut i = 1;
        while i < self.decorations.len() {
            let current_range = match &self.decorations[i] {
                Deco::Lifetime { range, .. }
                | Deco::ImmBorrow { range, .. }
                | Deco::MutBorrow { range, .. }
                | Deco::Move { range, .. }
                | Deco::Call { range, .. }
                | Deco::Outlive { range, .. }
                | Deco::Message { range, .. } => *range,
            };

            let mut j = 0;
            while j < i {
                let prev = &self.decorations[j];
                let prev_range = match prev {
                    Deco::Lifetime { range, .. }
                    | Deco::ImmBorrow { range, .. }
                    | Deco::MutBorrow { range, .. }
                    | Deco::Move { range, .. }
                    | Deco::Call { range, .. }
                    | Deco::Outlive { range, .. }
                    | Deco::Message { range, .. } => *range,
                };
                let prev_is_message = matches!(prev, Deco::Message { .. });
                if !prev_is_message {
                    j += 1;
                    continue;
                }

                if let Some(common) = utils::common_range(current_range, prev_range) {
                    if common.from < common.until {
                        let mut new_decos = Vec::new();
                        let non_overlapping = utils::exclude_ranges(vec![prev_range], vec![common]);

                        for range in non_overlapping {
                            let new_deco = match prev {
                                Deco::Lifetime { local, .. } => Deco::Lifetime {
                                    local: *local,
                                    range,
                                },
                                Deco::ImmBorrow { local, .. } => Deco::ImmBorrow {
                                    local: *local,
                                    range,
                                },
                                Deco::MutBorrow { local, .. } => Deco::MutBorrow {
                                    local: *local,
                                    range,
                                },
                                Deco::Move { local, .. } => Deco::Move {
                                    local: *local,
                                    range,
                                },
                                Deco::Call { local, .. } => Deco::Call {
                                    local: *local,
                                    range,
                                },
                                Deco::Outlive { local, .. } => Deco::Outlive {
                                    local: *local,
                                    range,
                                },
                                Deco::Message { local, message, .. } => Deco::Message {
                                    local: *local,
                                    range,
                                    message: message.to_owned(),
                                },
                            };
                            new_decos.push(new_deco);
                        }

                        match &mut self.decorations[j] {
                            Deco::Lifetime { range, .. }
                            | Deco::ImmBorrow { range, .. }
                            | Deco::MutBorrow { range, .. }
                            | Deco::Move { range, .. }
                            | Deco::Call { range, .. }
                            | Deco::Outlive { range, .. }
                            | Deco::Message { range, .. } => {
                                *range = common;
                            }
                        }

                        for (jj, deco) in new_decos.into_iter().enumerate() {
                            self.decorations.insert(j + jj + 1, deco);
                        }
                        self.sort_by_definition();
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }
}
impl utils::MirVisitor for CalcDecos {
    fn visit_decl(&mut self, decl: &MirDecl) {
        if let MirDecl::User {
            local_index,
            fn_id,
            lives,
            drop_range,
            must_live_at,
            name,
            ..
        } = decl
        {
            self.current_fn_id = *fn_id;
            let local = Local::new(*local_index, *fn_id);
            if self.locals.contains(&local) {
                // merge Drop object lives
                let mut drop_copy_live = lives.clone();
                drop_copy_live.extend_from_slice(drop_range);
                drop_copy_live = utils::eliminated_ranges(drop_copy_live.clone());
                for range in &drop_copy_live {
                    self.decorations.push(Deco::Lifetime {
                        local,
                        range: *range,
                    });
                    self.decorations.push(Deco::Message {
                        local,
                        range: *range,
                        message: format!("lifetime of variable `{}`", name),
                    })
                }
                let outlive = utils::exclude_ranges(must_live_at.clone(), drop_copy_live);
                for range in outlive {
                    self.decorations.push(Deco::Outlive { local, range });
                    self.decorations.push(Deco::Message {
                        local,
                        range,
                        message: format!("variable `{}` is required to live here", name),
                    })
                }
            }
        }
    }
    fn visit_stmt(&mut self, stmt: &MirStatement) {
        if let MirStatement::Assign { rval, .. } = stmt {
            match rval {
                Some(MirRval::Move {
                    target_local_index,
                    range,
                }) => {
                    let local = Local::new(*target_local_index, self.current_fn_id);
                    if self.locals.contains(&local) {
                        self.decorations.push(Deco::Move {
                            local,
                            range: *range,
                        });
                        self.decorations.push(Deco::Message {
                            local,
                            range: *range,
                            message: "variable moved".to_string(),
                        })
                    }
                }
                Some(MirRval::Borrow {
                    target_local_index,
                    range,
                    mutable,
                    ..
                }) => {
                    let local = Local::new(*target_local_index, self.current_fn_id);
                    if self.locals.contains(&local) {
                        if *mutable {
                            self.decorations.push(Deco::MutBorrow {
                                local,
                                range: *range,
                            });
                            self.decorations.push(Deco::Message {
                                local,
                                range: *range,
                                message: "mutable borrow".to_string(),
                            })
                        } else {
                            self.decorations.push(Deco::ImmBorrow {
                                local,
                                range: *range,
                            });
                            self.decorations.push(Deco::Message {
                                local,
                                range: *range,
                                message: "immutable borrow".to_string(),
                            })
                        }
                    }
                }
                _ => {}
            }
        }
    }
    fn visit_term(&mut self, term: &MirTerminator) {
        if let MirTerminator::Call {
            destination_local_index,
            fn_span,
        } = term
        {
            let local = Local::new(*destination_local_index, self.current_fn_id);
            if self.locals.contains(&local) {
                let mut i = 0;
                for deco in &self.decorations {
                    if let Deco::Call { range, .. } = deco {
                        if utils::is_super_range(*fn_span, *range) {
                            return;
                        }
                    }
                }
                while i < self.decorations.len() {
                    let range = match &self.decorations[i] {
                        Deco::Call { range, .. } => Some(range),
                        _ => None,
                    };
                    if let Some(range) = range {
                        if utils::is_super_range(*range, *fn_span) {
                            self.decorations.remove(i);
                            continue;
                        }
                    }
                    i += 1;
                }
                self.decorations.push(Deco::Call {
                    local,
                    range: *fn_span,
                });
                self.decorations.push(Deco::Message {
                    local,
                    range: *fn_span,
                    message: "function call".to_string(),
                })
            }
        }
    }
}

type Subprocess = Option<u32>;

#[derive(Debug)]
struct Backend {
    #[allow(unused)]
    client: Client,
    roots: Arc<RwLock<HashMap<PathBuf, PathBuf>>>,
    analyzed: Arc<RwLock<Option<Workspace>>>,
    processes: Arc<RwLock<JoinSet<()>>>,
    subprocesses: Arc<RwLock<Vec<Subprocess>>>,
    work_done_progress: Arc<RwLock<bool>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            roots: Arc::new(RwLock::new(HashMap::new())),
            analyzed: Arc::new(RwLock::new(None)),
            processes: Arc::new(RwLock::new(JoinSet::new())),
            subprocesses: Arc::new(RwLock::new(vec![])),
            work_done_progress: Arc::new(RwLock::new(false)),
        }
    }
    async fn set_roots(&self, uri: &lsp_types::Url) {
        let path = uri.to_file_path().unwrap();
        let dir = if path.is_dir() {
            path
        } else {
            path.parent().unwrap().to_path_buf()
        };
        let mut write = self.roots.write().await;
        if let Ok(metadata) = cargo_metadata::MetadataCommand::new()
            .current_dir(&dir)
            .exec()
        {
            let path = metadata.workspace_root;
            if !write.contains_key(path.as_std_path()) {
                write.insert(
                    path.as_std_path().to_path_buf(),
                    Temp::new_dir().unwrap().to_path_buf(),
                );
            }
        }
    }

    async fn abort_subprocess(&self) {
        #[cfg(unix)]
        while let Some(pid) = self.subprocesses.write().await.pop() {
            if let Some(pid) = pid {
                unsafe {
                    libc::killpg(pid.try_into().unwrap(), libc::SIGTERM);
                }
            }
        }
    }

    async fn analyze(&self) {
        // wait for rust-analyzer
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        {
            *self.analyzed.write().await = None;
        }
        let roots = { self.roots.read().await.clone() };
        let mut join = self.processes.write().await;
        join.shutdown().await;
        self.abort_subprocess().await;

        // progress report
        let progress_token = if *self.work_done_progress.read().await {
            let token = format!("{}", uuid::Uuid::new_v4());
            Some(lsp_types::NumberOrString::String(token))
        } else {
            None
        };
        let client = self.client.clone();
        if let Some(token) = &progress_token {
            client
                .send_request::<lsp_types::request::WorkDoneProgressCreate>(
                    lsp_types::WorkDoneProgressCreateParams {
                        token: token.clone(),
                    },
                )
                .await
                .ok();

            let value = lsp_types::ProgressParamsValue::WorkDone(
                lsp_types::WorkDoneProgress::Begin(lsp_types::WorkDoneProgressBegin {
                    title: "RustOwl checking".to_owned(),
                    cancellable: Some(false),
                    message: None,
                    percentage: None,
                }),
            );
            client
                .send_notification::<lsp_types::notification::Progress>(lsp_types::ProgressParams {
                    token: token.clone(),
                    value,
                })
                .await;
        }

        for (root, target) in roots {
            let mut command = process::Command::new("rustup");
            command
                .args([
                    "run",
                    rustowl::toolchain_version::TOOLCHAIN_VERSION,
                    "cargo",
                    "check",
                ])
                .env("CARGO_TARGET_DIR", &target)
                .env("RUSTC_WORKSPACE_WRAPPER", "rustowlc")
                .env_remove("RUSTC_WRAPPER")
                .current_dir(&root)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true);
            #[cfg(unix)]
            unsafe {
                command.pre_exec(|| {
                    libc::setsid();
                    Ok(())
                });
            }
            let mut child = command.spawn().unwrap();
            let mut stdout = BufReader::new(child.stdout.take().unwrap()).lines();
            let analyzed = self.analyzed.clone();
            join.spawn(async move {
                while let Ok(Some(line)) = stdout.next_line().await {
                    if let Ok(ws) = serde_json::from_str::<Workspace>(&line) {
                        let write = &mut *analyzed.write().await;
                        if let Some(write) = write {
                            *write = write.clone().merge(ws);
                        } else {
                            *write = Some(ws);
                        }
                    }
                }
            });

            let pid = child.id();
            let client = self.client.clone();
            let subprocesses = self.subprocesses.clone();
            let token = progress_token.clone();
            join.spawn(async move {
                let _ = child.wait().await;
                let mut write = subprocesses.write().await;
                *write = write
                    .iter()
                    .filter(|v| **v != pid)
                    .map(|v| v.clone())
                    .collect();
                if write.len() == 0 {
                    if let Some(token) = token {
                        let value = lsp_types::ProgressParamsValue::WorkDone(
                            lsp_types::WorkDoneProgress::End(lsp_types::WorkDoneProgressEnd {
                                message: None,
                            }),
                        );
                        client
                            .send_notification::<lsp_types::notification::Progress>(
                                lsp_types::ProgressParams { token, value },
                            )
                            .await;
                    }
                }
            });
            self.subprocesses.write().await.push(pid);
        }
    }
    async fn cleanup_targets(&self) {
        for (_, target) in self.roots.read().await.iter() {
            std::fs::remove_dir_all(target).ok();
        }
    }

    async fn decos(&self, filepath: &Path, position: Loc) -> Vec<Deco> {
        let mut selected = SelectLocal::new(position);
        if let Some(analyzed) = &*self.analyzed.read().await {
            for (mir_filename, file) in analyzed.0.iter() {
                if filepath.ends_with(mir_filename) {
                    for item in &file.items {
                        utils::mir_visit(item, &mut selected);
                    }
                }
            }

            let mut calc = CalcDecos::new(selected.selected);
            for (mir_filename, file) in analyzed.0.iter() {
                if filepath.ends_with(mir_filename) {
                    for item in &file.items {
                        utils::mir_visit(item, &mut calc);
                    }
                }
            }
            calc.handle_overlapping();
            calc.decorations
        } else {
            Vec::new()
        }
    }

    async fn cursor(&self, params: CursorRequest) -> jsonrpc::Result<Decorations> {
        let is_analyzed = self.analyzed.read().await.is_some();
        if let Ok(path) = params.document.uri.to_file_path() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                let pos = Loc(utils::line_char_to_index(
                    &text,
                    params.position.line,
                    params.position.character,
                ));
                let decos = self.decos(&path, pos).await;
                let decorations = decos.into_iter().map(|v| v.to_lsp_range(&text)).collect();
                return Ok(Decorations {
                    is_analyzed,
                    path: Some(path),
                    decorations,
                });
            }
        }
        Ok(Decorations {
            is_analyzed,
            path: None,
            decorations: Vec::new(),
        })
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(err) => {
                log::error!("failed to create async runtime for graceful shutdown: {err}");
                return;
            }
        };
        rt.block_on(async {
            if let Err(err) = self.shutdown().await {
                log::error!("failed to shutdown the server gracefully: {err}");
            };
        });
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        params: lsp_types::InitializeParams,
    ) -> jsonrpc::Result<lsp_types::InitializeResult> {
        if let Some(wss) = params.workspace_folders {
            for ws in wss {
                self.set_roots(&ws.uri).await;
            }
        }
        let sync_options = lsp_types::TextDocumentSyncOptions {
            save: Some(lsp_types::TextDocumentSyncSaveOptions::Supported(true)),
            change: Some(lsp_types::TextDocumentSyncKind::INCREMENTAL),
            ..Default::default()
        };
        let workspace_cap = lsp_types::WorkspaceServerCapabilities {
            workspace_folders: Some(lsp_types::WorkspaceFoldersServerCapabilities {
                supported: Some(true),
                change_notifications: Some(lsp_types::OneOf::Left(true)),
            }),
            ..Default::default()
        };
        let server_cap = lsp_types::ServerCapabilities {
            text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Options(sync_options)),
            workspace: Some(workspace_cap),
            ..Default::default()
        };
        let init_res = lsp_types::InitializeResult {
            capabilities: server_cap,
            ..Default::default()
        };
        let health_checker = async move {
            if let Some(process_id) = params.process_id {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    if !process_alive::state(process_alive::Pid::from(process_id)).is_alive() {
                        panic!("The client process is dead");
                    }
                }
            }
        };
        if params
            .capabilities
            .window
            .map(|v| v.work_done_progress)
            .flatten()
            .unwrap_or(false)
        {
            *self.work_done_progress.write().await = true;
        }
        tokio::spawn(health_checker);
        Ok(init_res)
    }
    async fn initialized(&self, _p: lsp_types::InitializedParams) {
        self.analyze().await;
    }
    async fn did_save(&self, _params: lsp_types::DidSaveTextDocumentParams) {
        self.analyze().await;
    }
    async fn did_change(&self, _params: lsp_types::DidChangeTextDocumentParams) {
        *self.analyzed.write().await = None;
        self.processes.write().await.shutdown().await;
    }

    async fn did_change_workspace_folders(
        &self,
        params: lsp_types::DidChangeWorkspaceFoldersParams,
    ) -> () {
        for added in params.event.added {
            self.set_roots(&added.uri).await;
        }
        self.analyze().await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.cleanup_targets().await;
        self.processes.write().await.shutdown().await;
        self.abort_subprocess().await;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(Backend::new)
        .custom_method("rustowl/cursor", Backend::cursor)
        .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
