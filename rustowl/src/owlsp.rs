mod utils;

use models::*;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[allow(unused)]
#[derive(serde::Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Deco<R = Range> {
    Lifetime { local: usize, range: R },
    ImmBorrow { local: usize, range: R },
    MutBorrow { local: usize, range: R },
    Move { local: usize, range: R },
    Call { local: usize, range: R },
    OutLive { local: usize, range: R },
}
impl Deco<Range> {
    fn to_lsp_range(&self, s: &str) -> Deco<lsp_types::Range> {
        match self.clone() {
            Deco::Lifetime { local, range } => {
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
                Deco::Lifetime {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::ImmBorrow { local, range } => {
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
                Deco::ImmBorrow {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::MutBorrow { local, range } => {
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
                Deco::ImmBorrow {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::Move { local, range } => {
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
                Deco::Move {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::Call { local, range } => {
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
                Deco::Call {
                    local,
                    range: lsp_types::Range { start, end },
                }
            }
            Deco::OutLive { local, range } => {
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
                Deco::OutLive {
                    local,
                    range: lsp_types::Range { start, end },
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
struct CursorRequest {
    position: lsp_types::Position,
    document: lsp_types::TextDocumentIdentifier,
}

struct SelectLocal {
    pos: Loc,
    selected: Vec<usize>,
}
impl SelectLocal {
    fn new(pos: Loc) -> Self {
        Self {
            pos,
            selected: Vec::new(),
        }
    }
    fn select(&mut self, local: usize, range: Range) {
        if range.from <= self.pos && self.pos <= range.until {
            self.selected.push(local);
        }
    }
}
impl utils::MirVisitor for SelectLocal {
    fn visit_decl(&mut self, decl: &MirDecl) {
        match decl {
            MirDecl::User {
                local_index, span, ..
            } => {
                self.select(*local_index, *span);
            }
            _ => {}
        }
    }
    fn visit_stmt(&mut self, stmt: &MirStatement) {
        match stmt {
            MirStatement::Assign { rval, .. } => match rval {
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
            },
            _ => {}
        }
    }
    fn visit_term(&mut self, term: &MirTerminator) {
        match term {
            MirTerminator::Call {
                destination_local_index,
                fn_span,
            } => {
                self.select(*destination_local_index, *fn_span);
            }
            _ => {}
        }
    }
}
struct CalcDecos {
    locals: Vec<usize>,
    decorations: Vec<Deco>,
}
impl CalcDecos {
    pub fn new(locals: Vec<usize>) -> Self {
        Self {
            locals,
            decorations: Vec::new(),
        }
    }
}
impl utils::MirVisitor for CalcDecos {
    fn visit_decl(&mut self, decl: &MirDecl) {
        match decl {
            MirDecl::User {
                local_index,
                lives,
                drop,
                drop_range,
                must_live_at,
                ..
            } => {
                if self.locals.contains(local_index) {
                    for range in lives {
                        self.decorations.push(Deco::Lifetime {
                            local: *local_index,
                            range: *range,
                        });
                    }
                    for range in must_live_at
                        .into_iter()
                        .map(|v| {
                            if *drop {
                                utils::exclude_ranges(*v, drop_range.clone())
                            } else {
                                utils::exclude_ranges(*v, lives.clone())
                            }
                        })
                        .flatten()
                    {
                        self.decorations.push(Deco::OutLive {
                            local: *local_index,
                            range,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    fn visit_stmt(&mut self, stmt: &MirStatement) {
        match stmt {
            MirStatement::Assign { rval, .. } => match rval {
                Some(MirRval::Move {
                    target_local_index,
                    range,
                }) => {
                    if self.locals.contains(target_local_index) {
                        self.decorations.push(Deco::Move {
                            local: *target_local_index,
                            range: *range,
                        });
                    }
                }
                Some(MirRval::Borrow {
                    target_local_index,
                    range,
                    mutable,
                    ..
                }) => {
                    if self.locals.contains(target_local_index) {
                        if *mutable {
                            self.decorations.push(Deco::MutBorrow {
                                local: *target_local_index,
                                range: *range,
                            });
                        } else {
                            self.decorations.push(Deco::ImmBorrow {
                                local: *target_local_index,
                                range: *range,
                            });
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn visit_term(&mut self, term: &MirTerminator) {
        match term {
            MirTerminator::Call {
                destination_local_index,
                fn_span,
            } => {
                if self.locals.contains(destination_local_index) {
                    let mut i = 0;
                    for deco in &self.decorations {
                        match deco {
                            Deco::Call { range, .. } => {
                                if utils::is_super_range(*fn_span, *range) {
                                    return;
                                }
                            }
                            _ => {}
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
                }
            }
            _ => {}
        }
    }
}

fn read_dir_rec(path: &PathBuf) -> Vec<PathBuf> {
    let mut res = Vec::new();
    if let Ok(mut dirs) = std::fs::read_dir(path) {
        while let Some(Ok(dir)) = dirs.next() {
            if let Ok(meta) = dir.metadata() {
                if meta.is_dir() {
                    res.extend_from_slice(&read_dir_rec(&dir.path()));
                } else {
                    res.push(dir.path());
                }
            }
        }
    }
    res
}

#[derive(Debug)]
struct Backend {
    #[allow(unused)]
    client: Client,
    roots: Arc<RwLock<Vec<PathBuf>>>,
    analyzed: Arc<RwLock<Option<Workspace>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            roots: Arc::new(RwLock::new(Vec::new())),
            analyzed: Arc::new(RwLock::new(None)),
        }
    }
    async fn set_roots(&self, uri: &lsp_types::Url) {
        let path = uri.path();
        let mut write = self.roots.write().await;
        'outer: for path in read_dir_rec(&PathBuf::from(path)) {
            if let Some(filename) = path.file_name() {
                if filename == "Cargo.toml" {
                    let dir = path.parent().unwrap();
                    for added in write.iter() {
                        if dir.starts_with(added) {
                            continue 'outer;
                        }
                    }
                    write.push(path.parent().unwrap().to_path_buf());
                }
            }
        }
    }

    async fn analzye(&self) {
        // wait for rust-analyzer
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        {
            *self.analyzed.write().await = None;
        }
        let roots = { self.roots.read().await.clone() };
        for root in roots {
            let output = process::Command::new("cargo")
                .arg("owl")
                .current_dir(&root)
                .stdout(process::Stdio::piped())
                .stderr(process::Stdio::piped())
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
            if let Ok(ws) =
                serde_json::from_str::<Workspace>(&String::from_utf8_lossy(&output.stdout))
            {
                let write = &mut *self.analyzed.write().await;
                if let Some(write) = write {
                    *write = write.clone().merge(ws);
                } else {
                    *write = Some(ws);
                }
            }
        }
    }
    async fn decos(&self, filepath: &PathBuf, position: Loc) -> Vec<Deco> {
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
        return Ok(Decorations {
            is_analyzed,
            path: None,
            decorations: Vec::new(),
        });
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(
        &self,
        params: lsp_types::InitializeParams,
    ) -> jsonrpc::Result<lsp_types::InitializeResult> {
        if let Some(uri) = params.root_uri {
            self.set_roots(&uri).await;
        }
        let mut init_res = lsp_types::InitializeResult::default();
        let mut sync_option = lsp_types::TextDocumentSyncOptions::default();
        sync_option.save = Some(lsp_types::TextDocumentSyncSaveOptions::Supported(true));
        init_res.capabilities.text_document_sync =
            Some(lsp_types::TextDocumentSyncCapability::Options(sync_option));
        let mut workspace_cap = lsp_types::WorkspaceServerCapabilities::default();
        workspace_cap.workspace_folders = Some(lsp_types::WorkspaceFoldersServerCapabilities {
            supported: Some(true),
            change_notifications: Some(lsp_types::OneOf::Left(true)),
        });
        init_res.capabilities.workspace = Some(workspace_cap);
        Ok(init_res)
    }
    async fn initialized(&self, _p: lsp_types::InitializedParams) {
        //self.analzye().await;
    }
    async fn did_save(&self, _params: lsp_types::DidSaveTextDocumentParams) {
        self.analzye().await;
    }

    async fn did_change_workspace_folders(
        &self,
        params: lsp_types::DidChangeWorkspaceFoldersParams,
    ) -> () {
        for added in params.event.added {
            self.set_roots(&added.uri).await;
        }
        self.analzye().await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend::new(client))
        .custom_method("rustowl/cursor", Backend::cursor)
        .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
