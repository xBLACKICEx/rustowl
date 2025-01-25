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
    decorations: Vec<Deco<lsp_types::Range>>,
}
#[derive(serde::Deserialize, Clone, Debug)]
struct CursorRequest {
    position: lsp_types::Position,
    document: lsp_types::TextDocumentItem,
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

#[derive(Debug)]
struct Backend {
    #[allow(unused)]
    client: Client,
    workspace: Arc<RwLock<Option<PathBuf>>>,
    analyzed: Arc<RwLock<Option<Workspace>>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            workspace: Arc::new(RwLock::new(None)),
            analyzed: Arc::new(RwLock::new(None)),
        }
    }
    async fn analzye(&self) {
        if let Some(ws) = { self.workspace.read().await.clone() } {
            let output = process::Command::new("cargo")
                .arg("owl")
                .current_dir(ws)
                .stdout(process::Stdio::piped())
                .spawn()
                .unwrap()
                .wait_with_output()
                .unwrap();
            *self.analyzed.write().await =
                serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
        }
    }
    async fn decos(&self, filepath: PathBuf, position: Loc) -> Vec<Deco> {
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
        if params.document.language_id == "rust" {
            if let Ok(path) = params.document.uri.to_file_path() {
                let decos = self
                    .decos(
                        path,
                        Loc(utils::line_char_to_index(
                            &params.document.text,
                            params.position.line,
                            params.position.character,
                        )),
                    )
                    .await;
                let decorations = decos
                    .into_iter()
                    .map(|v| v.to_lsp_range(&params.document.text))
                    .collect();
                return Ok(Decorations { decorations });
            }
        }
        return Ok(Decorations {
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
        if let Some(root) = params.root_uri {
            *self.workspace.write().await = root.to_file_path().ok();
        }
        Ok(lsp_types::InitializeResult::default())
    }
    async fn initialized(&self, _p: lsp_types::InitializedParams) {
        self.analzye().await;
    }
    async fn did_save(&self, _params: lsp_types::DidSaveTextDocumentParams) {
        self.analzye().await;
    }

    async fn did_change_workspace_folders(
        &self,
        params: lsp_types::DidChangeWorkspaceFoldersParams,
    ) -> () {
        if let Some(ws) = params.event.added.get(0) {
            {
                *self.workspace.write().await = ws.uri.to_file_path().ok();
            }
            self.analzye().await;
        }
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
