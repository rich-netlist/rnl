//! The context a statement is evaluated in, and the frame of a definition body being expanded.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::obj::{Lit, MemberKey, MemberRef, ObjId, Pos, RecId, SceneId, UnitId};
use super::unit::Header;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameKind {
    /// The body of an ordinary definition being expanded for one usage (9.4).
    Definition,
    /// A symbol body (9.2).
    Symbol,
}

/// The body of a definition being expanded, or a symbol body (6.9, 6.10, 9.2).
#[derive(Debug)]
pub struct Frame {
    pub kind: FrameKind,
    pub usage: Option<ObjId>,
    pub roles: Vec<(String, ObjId)>,
    /// Parameter values; None for a parameter neither passed nor given a default.
    pub params: Vec<(String, Option<Lit>)>,
    pub locals: Vec<(String, ObjId)>,
    /// Local names and the members that bind them, evaluated on first use (forward references).
    pub local_members: Vec<(String, MemberRef, usize)>,
    pub symbol: Option<ObjId>,
    pub definition: Option<ObjId>,
    pub generated: bool,
    pub selected: bool,
}

impl Frame {
    pub fn new(kind: FrameKind) -> Frame {
        Frame {
            kind,
            usage: None,
            roles: Vec::new(),
            params: Vec::new(),
            locals: Vec::new(),
            local_members: Vec::new(),
            symbol: None,
            definition: None,
            generated: false,
            selected: false,
        }
    }

    pub fn role(&self, name: &str) -> Option<ObjId> {
        self.roles.iter().find(|(n, _)| n == name).map(|(_, o)| *o)
    }

    pub fn param(&self, name: &str) -> Option<&Option<Lit>> {
        self.params.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    pub fn local(&self, name: &str) -> Option<ObjId> {
        self.locals.iter().find(|(n, _)| n == name).map(|(_, o)| *o)
    }

    pub fn set_local(&mut self, name: &str, v: ObjId) {
        if let Some(e) = self.locals.iter_mut().find(|(n, _)| n == name) {
            e.1 = v;
        } else {
            self.locals.push((name.to_string(), v));
        }
    }
}

pub type FrameRef = Rc<RefCell<Frame>>;

/// A name bound inside a nested `{…}`: the member that binds it, or its value once known (D4).
#[derive(Clone, Debug)]
pub enum Nested {
    Member(MemberRef, usize),
    Val(ObjId),
}

pub type NestedRef = Rc<RefCell<HashMap<(char, String), Nested>>>;

/// The names whose definition bodies are being read: a body does not see the name it defines,
/// it means the outer definition of that name (6.9). (prefix, name) -> (member that binds it, unit).
pub type Defining = HashMap<(char, String), (MemberRef, UnitId)>;

/// A statement of a record, or of a definition body, by its record and its characters: what the
/// objects it makes are numbered by (the draft 0.3, 5.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StmtSpan {
    pub rec: Option<RecId>,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub struct Ctx {
    /// The drawing level; None for packages and included file levels (no circuit).
    pub scene: Option<SceneId>,
    /// Where the text is written (names).
    pub unit: UnitId,
    pub rec: Option<RecId>,
    pub header: Rc<Header>,
    pub pos: Pos,
    pub frame: Option<FrameRef>,
    pub origin: Rc<str>,
    pub nested: Option<NestedRef>,
    pub member_key: MemberKey,
    /// The value came through a selection or generation.
    pub selected: bool,
    pub defining: Option<Rc<Defining>>,
    /// The unit whose names a definition body is resolved with (6.9).
    pub def_unit: Option<UnitId>,
    /// A record written above with `scope=` this subcircuit (2.6).
    pub scope_in: bool,
    /// The statement being evaluated: the member of the record, or in a definition body the member
    /// of the body (the draft 0.3, 5.9).
    pub stmt: Option<StmtSpan>,
}

impl Ctx {
    pub fn frame_kind(&self) -> Option<FrameKind> {
        self.frame.as_ref().map(|f| f.borrow().kind)
    }

    pub fn in_symbol(&self) -> bool {
        self.frame_kind() == Some(FrameKind::Symbol)
    }

    pub fn in_definition(&self) -> bool {
        self.frame_kind() == Some(FrameKind::Definition)
    }

    /// The usage whose definition body this is.
    pub fn frame_usage(&self) -> Option<ObjId> {
        match &self.frame {
            Some(f) if f.borrow().kind == FrameKind::Definition => f.borrow().usage,
            _ => None,
        }
    }

    pub fn frame_generated(&self) -> bool {
        match &self.frame {
            Some(f) => {
                let f = f.borrow();
                f.kind == FrameKind::Definition && f.generated
            }
            None => false,
        }
    }

    pub fn with_pos(&self, pos: Pos) -> Ctx {
        let mut c = self.clone();
        c.pos = pos;
        c
    }
}
