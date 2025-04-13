use crate::{lsp::progress, models::*, utils};
use std::collections::HashSet;
use std::path::PathBuf;
use tower_lsp::lsp_types;

#[derive(serde::Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Deco<R = Range> {
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
    pub fn to_lsp_range(&self, s: &str) -> Deco<lsp_types::Range> {
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
pub struct Decorations {
    pub is_analyzed: bool,
    pub status: progress::AnalysisStatus,
    pub path: Option<PathBuf>,
    pub decorations: Vec<Deco<lsp_types::Range>>,
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct CursorRequest {
    pub position: lsp_types::Position,
    pub document: lsp_types::TextDocumentIdentifier,
}
impl CursorRequest {
    pub fn path(&self) -> Option<PathBuf> {
        self.document.uri.to_file_path().ok()
    }
    pub fn position(&self) -> lsp_types::Position {
        self.position
    }
}

#[derive(Clone, Debug)]
pub struct SelectLocal {
    pos: Loc,
    selected: HashSet<Local>,
    current_fn_id: u32,
}
impl SelectLocal {
    pub fn new(pos: Loc) -> Self {
        Self {
            pos,
            selected: HashSet::new(),
            current_fn_id: 0,
        }
    }
    pub fn select(&mut self, local_id: u32, range: Range) {
        if range.from() <= self.pos && self.pos <= range.until() {
            self.selected
                .insert(Local::new(local_id, self.current_fn_id));
        }
    }
    pub fn selected(&self) -> &HashSet<Local> {
        &self.selected
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
pub struct CalcDecos {
    locals: HashSet<Local>,
    decorations: Vec<Deco>,
    current_fn_id: u32,
}
impl CalcDecos {
    pub fn new(locals: impl IntoIterator<Item = Local>) -> Self {
        Self {
            locals: locals.into_iter().collect(),
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

    pub fn handle_overlapping(&mut self) {
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

    pub fn decorations(self) -> Vec<Deco> {
        self.decorations
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
