//! # RustOwl cargo-owlsp
//!
//! An LSP server for visualizing ownership and lifetimes in Rust, designed for debugging and optimization.

use rustowl::models::*;
use rustowl::utils;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
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
        hover_text: String,
        overlapped: bool,
    },
    ImmBorrow {
        local: Local,
        range: R,
        hover_text: String,
        overlapped: bool,
    },
    MutBorrow {
        local: Local,
        range: R,
        hover_text: String,
        overlapped: bool,
    },
    Move {
        local: Local,
        range: R,
        hover_text: String,
        overlapped: bool,
    },
    Call {
        local: Local,
        range: R,
        hover_text: String,
        overlapped: bool,
    },
    SharedMut {
        local: Local,
        range: R,
        hover_text: String,
        overlapped: bool,
    },
    Outlive {
        local: Local,
        range: R,
        hover_text: String,
        overlapped: bool,
    },
}
impl Deco<Range> {
    fn to_lsp_range(&self, s: &str) -> Deco<lsp_types::Range> {
        match self.clone() {
            Deco::Lifetime {
                local,
                range,
                hover_text,
                overlapped,
            } => {
                let start = utils::index_to_line_char(s, range.from());
                let end = utils::index_to_line_char(s, range.until());
                let start = lsp_types::Position {
                    line: start.0,
                    character: start.1,
                };
                let end = lsp_types::Position {
                    line: end.0,
                    character: end.1,
                };
                Deco::Lifetime {
                    local,
                    range: lsp_types::Range { start, end },
                    hover_text,
                    overlapped,
                }
            }
            Deco::ImmBorrow {
                local,
                range,
                hover_text,
                overlapped,
            } => {
                let start = utils::index_to_line_char(s, range.from());
                let end = utils::index_to_line_char(s, range.until());
                let start = lsp_types::Position {
                    line: start.0,
                    character: start.1,
                };
                let end = lsp_types::Position {
                    line: end.0,
                    character: end.1,
                };
                Deco::ImmBorrow {
                    local,
                    range: lsp_types::Range { start, end },
                    hover_text,
                    overlapped,
                }
            }
            Deco::MutBorrow {
                local,
                range,
                hover_text,
                overlapped,
            } => {
                let start = utils::index_to_line_char(s, range.from());
                let end = utils::index_to_line_char(s, range.until());
                let start = lsp_types::Position {
                    line: start.0,
                    character: start.1,
                };
                let end = lsp_types::Position {
                    line: end.0,
                    character: end.1,
                };
                Deco::MutBorrow {
                    local,
                    range: lsp_types::Range { start, end },
                    hover_text,
                    overlapped,
                }
            }
            Deco::Move {
                local,
                range,
                hover_text,
                overlapped,
            } => {
                let start = utils::index_to_line_char(s, range.from());
                let end = utils::index_to_line_char(s, range.until());
                let start = lsp_types::Position {
                    line: start.0,
                    character: start.1,
                };
                let end = lsp_types::Position {
                    line: end.0,
                    character: end.1,
                };
                Deco::Move {
                    local,
                    range: lsp_types::Range { start, end },
                    hover_text,
                    overlapped,
                }
            }
            Deco::Call {
                local,
                range,
                hover_text,
                overlapped,
            } => {
                let start = utils::index_to_line_char(s, range.from());
                let end = utils::index_to_line_char(s, range.until());
                let start = lsp_types::Position {
                    line: start.0,
                    character: start.1,
                };
                let end = lsp_types::Position {
                    line: end.0,
                    character: end.1,
                };
                Deco::Call {
                    local,
                    range: lsp_types::Range { start, end },
                    hover_text,
                    overlapped,
                }
            }
            Deco::SharedMut {
                local,
                range,
                hover_text,
                overlapped,
            } => {
                let start = utils::index_to_line_char(s, range.from());
                let end = utils::index_to_line_char(s, range.until());
                let start = lsp_types::Position {
                    line: start.0,
                    character: start.1,
                };
                let end = lsp_types::Position {
                    line: end.0,
                    character: end.1,
                };
                Deco::SharedMut {
                    local,
                    range: lsp_types::Range { start, end },
                    hover_text,
                    overlapped,
                }
            }

            Deco::Outlive {
                local,
                range,
                hover_text,
                overlapped,
            } => {
                let start = utils::index_to_line_char(s, range.from());
                let end = utils::index_to_line_char(s, range.until());
                let start = lsp_types::Position {
                    line: start.0,
                    character: start.1,
                };
                let end = lsp_types::Position {
                    line: end.0,
                    character: end.1,
                };
                Deco::Outlive {
                    local,
                    range: lsp_types::Range { start, end },
                    hover_text,
                    overlapped,
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
    selected: HashSet<Local>,
    current_fn_id: u32,
}
impl SelectLocal {
    fn new(pos: Loc) -> Self {
        Self {
            pos,
            selected: HashSet::new(),
            current_fn_id: 0,
        }
    }
    fn select(&mut self, local_id: u32, range: Range) {
        if range.from() <= self.pos && self.pos <= range.until() {
            self.selected
                .insert(Local::new(local_id, self.current_fn_id));
        }
    }
}
impl utils::MirVisitor for SelectLocal {
    fn visit_decl(&mut self, decl: &MirUserDecl) {
        let MirUserDecl {
            local_index,
            fn_id,
            span,
            ..
        } = decl;
        self.current_fn_id = *fn_id;
        self.select(*local_index, *span);
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
    locals: HashSet<Local>,
    decorations: Vec<Deco>,
    current_fn_id: u32,
}
impl CalcDecos {
    pub fn new(locals: HashSet<Local>) -> Self {
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
            Deco::SharedMut { .. } => 5,
            Deco::Outlive { .. } => 6,
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
                | Deco::SharedMut { range, .. }
                | Deco::Outlive { range, .. } => *range,
            };

            let mut j = 0;
            while j < i {
                let prev = &self.decorations[j];
                let (prev_range, prev_overlapped) = match prev {
                    Deco::Lifetime {
                        range, overlapped, ..
                    }
                    | Deco::ImmBorrow {
                        range, overlapped, ..
                    }
                    | Deco::MutBorrow {
                        range, overlapped, ..
                    }
                    | Deco::Move {
                        range, overlapped, ..
                    }
                    | Deco::Call {
                        range, overlapped, ..
                    }
                    | Deco::SharedMut {
                        range, overlapped, ..
                    }
                    | Deco::Outlive {
                        range, overlapped, ..
                    } => (*range, *overlapped),
                };

                if prev_overlapped {
                    j += 1;
                    continue;
                }

                if let Some(common) = utils::common_range(current_range, prev_range) {
                    let mut new_decos = Vec::new();
                    let non_overlapping = utils::exclude_ranges(vec![prev_range], vec![common]);

                    for range in non_overlapping {
                        let new_deco = match prev {
                            Deco::Lifetime {
                                local, hover_text, ..
                            } => Deco::Lifetime {
                                local: *local,
                                range,
                                hover_text: hover_text.clone(),
                                overlapped: false,
                            },
                            Deco::ImmBorrow {
                                local, hover_text, ..
                            } => Deco::ImmBorrow {
                                local: *local,
                                range,
                                hover_text: hover_text.clone(),
                                overlapped: false,
                            },
                            Deco::MutBorrow {
                                local, hover_text, ..
                            } => Deco::MutBorrow {
                                local: *local,
                                range,
                                hover_text: hover_text.clone(),
                                overlapped: false,
                            },
                            Deco::Move {
                                local, hover_text, ..
                            } => Deco::Move {
                                local: *local,
                                range,
                                hover_text: hover_text.clone(),
                                overlapped: false,
                            },
                            Deco::Call {
                                local, hover_text, ..
                            } => Deco::Call {
                                local: *local,
                                range,
                                hover_text: hover_text.clone(),
                                overlapped: false,
                            },
                            Deco::SharedMut {
                                local, hover_text, ..
                            } => Deco::SharedMut {
                                local: *local,
                                range,
                                hover_text: hover_text.clone(),
                                overlapped: false,
                            },
                            Deco::Outlive {
                                local, hover_text, ..
                            } => Deco::Outlive {
                                local: *local,
                                range,
                                hover_text: hover_text.clone(),
                                overlapped: false,
                            },
                        };
                        new_decos.push(new_deco);
                    }

                    match &mut self.decorations[j] {
                        Deco::Lifetime {
                            range, overlapped, ..
                        }
                        | Deco::ImmBorrow {
                            range, overlapped, ..
                        }
                        | Deco::MutBorrow {
                            range, overlapped, ..
                        }
                        | Deco::Move {
                            range, overlapped, ..
                        }
                        | Deco::Call {
                            range, overlapped, ..
                        }
                        | Deco::SharedMut {
                            range, overlapped, ..
                        }
                        | Deco::Outlive {
                            range, overlapped, ..
                        } => {
                            *range = common;
                            *overlapped = true;
                        }
                    }

                    for (jj, deco) in new_decos.into_iter().enumerate() {
                        self.decorations.insert(j + jj + 1, deco);
                    }
                }
                j += 1;
            }
            i += 1;
        }
    }
}
impl utils::MirVisitor for CalcDecos {
    fn visit_decl(&mut self, decl: &MirUserDecl) {
        let MirUserDecl {
            local_index,
            fn_id,
            lives,
            shared_borrow,
            mutable_borrow,
            drop_range,
            must_live_at,
            name,
            ..
        } = decl;
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
                    hover_text: format!("lifetime of variable `{}`", name),
                    overlapped: false,
                });
            }
            let mut borrow_ranges = shared_borrow.clone();
            borrow_ranges.extend_from_slice(mutable_borrow);
            let shared_mut = utils::common_ranges(&borrow_ranges);
            for range in shared_mut {
                self.decorations.push(Deco::SharedMut {
                    local,
                    range,
                    hover_text: format!(
                        "immutable and mutable borrows of variable `{name}` exist here",
                    ),
                    overlapped: false,
                });
            }
            let outlive = utils::exclude_ranges(must_live_at.clone(), drop_copy_live);
            for range in outlive {
                self.decorations.push(Deco::Outlive {
                    local,
                    range,
                    hover_text: format!("variable `{name}` is required to live here"),
                    overlapped: false,
                });
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
                            hover_text: "variable moved".to_string(),
                            overlapped: false,
                        });
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
                                hover_text: "mutable borrow".to_string(),
                                overlapped: false,
                            });
                        } else {
                            self.decorations.push(Deco::ImmBorrow {
                                local,
                                range: *range,
                                hover_text: "immutable borrow".to_string(),
                                overlapped: false,
                            });
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
                    hover_text: "function call".to_string(),
                    overlapped: false,
                });
            }
        }
    }
}

type Subprocess = Option<u32>;

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(tag = "reason", rename_all = "kebab-case")]
enum CargoCheckMessage {
    #[allow(unused)]
    CompilerArtifact {},
    #[allow(unused)]
    BuildFinished {},
}

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
    /// returns `true` if the root was not registered
    async fn set_roots(&self, path: PathBuf) -> bool {
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
            let target = metadata
                .target_directory
                .as_std_path()
                .to_path_buf()
                .join("owl");
            tokio::fs::create_dir_all(&target).await.unwrap();

            if !write.contains_key(path.as_std_path()) {
                log::info!("add {} to watch list", path);
                write.insert(path.as_std_path().to_path_buf(), target);
                return true;
            }
        }
        false
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
        log::info!("stop running analysis processes");
        self.processes.write().await.shutdown().await;
        log::info!("wait 100ms for rust-analyzer");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        log::info!("start analysis");
        let roots = { self.roots.read().await.clone() };
        let mut join = self.processes.write().await;
        join.shutdown().await;
        self.abort_subprocess().await;

        for (root, target) in roots {
            // progress report
            let meta = cargo_metadata::MetadataCommand::new()
                .current_dir(&root)
                .exec()
                .ok();
            let dep_count = meta
                .as_ref()
                .and_then(|v| v.resolve.as_ref().map(|w| w.nodes.len()))
                .unwrap_or(0);
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
                        message: Some(format!("0 / {dep_count}")),
                        percentage: Some(0),
                    }),
                );
                client
                    .send_notification::<lsp_types::notification::Progress>(
                        lsp_types::ProgressParams {
                            token: token.clone(),
                            value,
                        },
                    )
                    .await;
            }

            if let Some(package_name) = meta.and_then(|v| v.root_package().cloned()).map(|v| v.name)
            {
                log::info!("clear cargo cache");
                let mut command = process::Command::new("cargo");
                command
                    .args(["clean", "--package", &package_name])
                    .env("CARGO_TARGET_DIR", &target)
                    .current_dir(&root)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                command.spawn().unwrap().wait().await.ok();
            }

            log::info!("start checking {}", root.display());
            let mut command = process::Command::new("rustup");
            command
                .args([
                    "run",
                    rustowl::toolchain_version::TOOLCHAIN_VERSION,
                    "cargo",
                    "check",
                    "--all-targets",
                    "--all-features",
                    "--keep-going",
                    "--message-format=json",
                ])
                .env("CARGO_TARGET_DIR", &target)
                .env("RUSTC_WORKSPACE_WRAPPER", "rustowlc")
                .env_remove("RUSTC_WRAPPER")
                .current_dir(&root)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
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
            let token = progress_token.clone();
            join.spawn(async move {
                let mut build_count = 0;
                while let Ok(Some(line)) = stdout.next_line().await {
                    if let Ok(CargoCheckMessage::CompilerArtifact { .. }) =
                        serde_json::from_str(&line)
                    {
                        build_count += 1;
                        log::info!("{build_count} crates checked");
                        if let Some(token) = token.clone() {
                            let percentage = (build_count * 100 / dep_count).min(100);
                            let value = lsp_types::ProgressParamsValue::WorkDone(
                                lsp_types::WorkDoneProgress::Report(
                                    lsp_types::WorkDoneProgressReport {
                                        cancellable: Some(false),
                                        message: Some(format!("{build_count} / {dep_count}")),
                                        percentage: Some(percentage as u32),
                                    },
                                ),
                            );
                            client
                                .send_notification::<lsp_types::notification::Progress>(
                                    lsp_types::ProgressParams { token, value },
                                )
                                .await;
                        }
                    }
                    if let Ok(ws) = serde_json::from_str::<Workspace>(&line) {
                        let write = &mut *analyzed.write().await;
                        if let Some(write) = write {
                            write.merge(ws);
                        } else {
                            *write = Some(ws);
                        }
                    }
                }
            });

            let mut stderr = BufReader::new(child.stderr.take().unwrap()).lines();
            join.spawn(async move {
                while let Ok(Some(line)) = stderr.next_line().await {
                    log::debug!("rustowlc: {line}");
                }
            });

            let pid = child.id();
            let client = self.client.clone();
            let subprocesses = self.subprocesses.clone();
            let token = progress_token.clone();
            let cache_target = target.join("cache.json");
            let analyzed = self.analyzed.clone();
            join.spawn(async move {
                let _ = child.wait().await;
                log::info!("check finished");
                let mut write = subprocesses.write().await;
                *write = write.iter().filter(|v| **v != pid).copied().collect();
                if let Ok(mut cache_file) = tokio::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(cache_target)
                    .await
                {
                    cache_file
                        .write_all(&serde_json::to_vec(&*analyzed.read().await).unwrap())
                        .await
                        .ok();
                }
                if write.is_empty() {
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

    async fn decos(&self, filepath: &Path, position: Loc) -> Vec<Deco> {
        let mut selected = SelectLocal::new(position);
        if let Some(analyzed) = &*self.analyzed.read().await {
            for (_crate_name, krate) in analyzed.0.iter() {
                for (filename, file) in krate.0.iter() {
                    if filepath == PathBuf::from(filename) {
                        for item in &file.items {
                            utils::mir_visit(item, &mut selected);
                        }
                    }
                }
            }

            let mut calc = CalcDecos::new(selected.selected);
            for (_crate_name, krate) in analyzed.0.iter() {
                for (filename, file) in krate.0.iter() {
                    if filepath == PathBuf::from(filename) {
                        for item in &file.items {
                            utils::mir_visit(item, &mut calc);
                        }
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
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if let Err(err) = self.shutdown().await {
                    log::error!("failed to shutdown the server gracefully: {err}");
                };
            });
        });
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        params: lsp_types::InitializeParams,
    ) -> jsonrpc::Result<lsp_types::InitializeResult> {
        let sync_options = lsp_types::TextDocumentSyncOptions {
            open_close: Some(true),
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
            .and_then(|v| v.work_done_progress)
            .unwrap_or(false)
        {
            *self.work_done_progress.write().await = true;
        }
        tokio::spawn(health_checker);
        Ok(init_res)
    }
    async fn did_open(&self, params: lsp_types::DidOpenTextDocumentParams) {
        if params.text_document.language_id == "rust"
            && self
                .set_roots(params.text_document.uri.to_file_path().unwrap())
                .await
        {
            self.analyze().await;
        }
    }
    async fn did_save(&self, _params: lsp_types::DidSaveTextDocumentParams) {
        self.analyze().await;
    }
    async fn did_change(&self, _params: lsp_types::DidChangeTextDocumentParams) {
        *self.analyzed.write().await = None;
        self.processes.write().await.shutdown().await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        self.processes.write().await.shutdown().await;
        self.abort_subprocess().await;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_colors(true)
        .init()
        .unwrap();
    log::set_max_level(log::LevelFilter::Off);

    #[cfg(windows)]
    {
        let home = PathBuf::from(
            String::from_utf8_lossy(
                &process::Command::new("rustup")
                    .args(["show", "home"])
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .unwrap()
                    .wait_with_output()
                    .await
                    .unwrap()
                    .stdout,
            )
            .trim(),
        );
        let mut paths = env::split_paths(&env::var_os("Path").unwrap())
            .collect::<std::collections::VecDeque<_>>();
        paths.push_front(
            home.join("toolchains")
                .join(rustowl::toolchain_version::TOOLCHAIN_VERSION)
                .join("bin"),
        );
        unsafe {
            env::set_var("Path", env::join_paths(paths).unwrap());
        }
    }

    let matches = clap::Command::new("RustOwl Language Server")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .arg(
            clap::Arg::new("io")
                .long("stdio")
                .required(false)
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand_required(false)
        .subcommand(
            clap::Command::new("check").arg(
                clap::Arg::new("log_level")
                    .long("log")
                    .required(false)
                    .action(clap::ArgAction::Set),
            ),
        )
        .get_matches();

    if let Some(arg) = matches.subcommand() {
        if let ("check", matches) = arg {
            let log_level = matches
                .get_one::<String>("log_level")
                .cloned()
                .unwrap_or("info".to_owned());
            log::set_max_level(log_level.parse().unwrap());
            if check(env::current_dir().unwrap()).await {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
    } else {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::build(Backend::new)
            .custom_method("rustowl/cursor", Backend::cursor)
            .finish();
        Server::new(stdin, stdout, socket).serve(service).await;
    }
}

async fn check(path: PathBuf) -> bool {
    let (service, _) = LspService::build(Backend::new).finish();
    let backend = service.inner();
    backend.set_roots(path).await;
    backend.analyze().await;
    while backend.processes.write().await.join_next().await.is_some() {}
    backend
        .analyzed
        .read()
        .await
        .as_ref()
        .map(|v| !v.0.is_empty())
        .unwrap_or(false)
}
