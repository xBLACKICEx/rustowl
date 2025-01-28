use models::*;

pub fn is_super_range(r1: Range, r2: Range) -> bool {
    (r1.from < r2.from && r2.until <= r1.until) || (r1.from <= r2.from && r2.until < r1.until)
}

pub fn common_range(r1: Range, r2: Range) -> Option<Range> {
    if r2.from < r1.from {
        return common_range(r2, r1);
    }
    if r1.until < r2.from {
        return None;
    }
    let from = r2.from;
    let until = r1.until.min(r2.until);
    Some(Range { from, until })
}

pub fn merge_ranges(r1: Range, r2: Range) -> Option<Range> {
    if let Some(_) = common_range(r1, r2) {
        let from = r1.from.min(r2.from);
        let until = r1.until.max(r2.until);
        Some(Range { from, until })
    } else {
        None
    }
}

pub fn eliminated_ranges(mut ranges: Vec<Range>) -> Vec<Range> {
    let mut i = 0;
    'outer: while i < ranges.len() {
        let mut j = i + 1;
        while j < ranges.len() {
            if let Some(eliminated) = merge_ranges(ranges[i], ranges[j]) {
                ranges[i] = eliminated;
                ranges.remove(j);
                continue 'outer;
            } else {
                j += 1;
            }
        }
        i += 1;
    }
    ranges
}

pub fn exclude_range(from: Range, exclude: Range) -> Vec<Range> {
    if let Some(common) = common_range(from, exclude) {
        let r1 = Range {
            from: from.from,
            until: common.from - 1,
        };
        let r2 = Range {
            from: common.until + 1,
            until: from.until,
        };
        let mut res = if r1.from < r1.until {
            vec![r1]
        } else {
            Vec::new()
        };
        if r2.from < r2.until {
            res.push(r2);
        }
        res
    } else {
        vec![from]
    }
}

pub fn exclude_ranges(from: Range, excludes: Vec<Range>) -> Vec<Range> {
    let mut res = vec![from];
    let mut i = 0;
    while i < excludes.len() {
        let mut j = 0;
        while j < res.len() {
            let tmp = exclude_range(res[j], excludes[i]);
            res.remove(j);
            j += tmp.len();
            res.extend(tmp);
        }
        i += 1;
    }
    eliminated_ranges(res)
}

#[allow(unused)]
pub trait MirVisitor {
    fn visit_func(&mut self, func: &Function) {}
    fn visit_decl(&mut self, decl: &MirDecl) {}
    fn visit_stmt(&mut self, stmt: &MirStatement) {}
    fn visit_term(&mut self, term: &MirTerminator) {}
}
pub fn mir_visit(func: &Function, visitor: &mut impl MirVisitor) {
    visitor.visit_func(func);
    for decl in &func.decls {
        visitor.visit_decl(decl);
    }
    for bb in &func.basic_blocks {
        for stmt in &bb.statements {
            visitor.visit_stmt(stmt);
        }
        if let Some(term) = &bb.terminator {
            visitor.visit_term(term);
        }
    }
}

pub fn index_to_line_char(s: &str, idx: u32) -> (u32, u32) {
    let mut line = 0;
    let mut col = 0;
    for (i, c) in s.chars().enumerate() {
        if idx == i as u32 {
            return (line, col);
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else if c != '\r' {
            col += 1;
        }
    }
    (0, 0)
}
pub fn line_char_to_index(s: &str, mut line: u32, char: u32) -> u32 {
    let mut col = 0;
    for (i, c) in s.chars().enumerate() {
        if line == 0 && col == char {
            return i as u32;
        }
        if c == '\n' && 0 < line {
            line -= 1;
            col = 0;
        } else if c != '\r' {
            col += 1;
        }
    }
    0
}
