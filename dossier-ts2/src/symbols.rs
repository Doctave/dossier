use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;

use crate::import::Import;

#[derive(Debug, Clone, PartialEq)]
/// A symbol we've discovered in the source code.
pub(crate) struct Symbol {
    pub kind: SymbolKind,
    pub source: Source,
}

/// The type of the symbol.
/// Contains all the metadata associated with that type of symbol
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolKind {
    Function(crate::function::Function),
}

/// The source of the symbol.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Source {
    pub offset_start_bytes: usize,
    pub offset_end_bytes: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SymbolReference {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TableEntry {
    Symbol(Symbol),
    Ref(SymbolReference),
}

static SCOPE_ID: AtomicUsize = AtomicUsize::new(0);

type ScopeID = usize;

/// The symbol table for a single file.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Scope {
    pub id: ScopeID,
    pub parent: Option<ScopeID>,
    pub entries: Vec<TableEntry>,
    pub imports: Vec<Import>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SymbolTable {
    pub file: PathBuf,
    scopes: Vec<Scope>,
    current_scope_id: ScopeID,
}

impl SymbolTable {
    pub fn new() -> Self {
        let root_id = SCOPE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Self {
            file: PathBuf::new(),
            current_scope_id: root_id,
            scopes: vec![Scope {
                id: root_id,
                parent: None,
                entries: vec![],
                imports: vec![],
            }],
        }
    }

    pub fn all_entries(&self) -> impl Iterator<Item = &TableEntry> {
        self.scopes.iter().flat_map(|s| s.entries.iter())
    }

    pub fn all_imports(&self) -> impl Iterator<Item = &Import> {
        self.scopes.iter().flat_map(|s| s.imports.iter())
    }

    pub fn add_entry(&mut self, entry: TableEntry) {
        self.current_scope_mut().entries.push(entry);
    }

    pub fn add_import(&mut self, import: Import) {
        self.current_scope_mut().imports.push(import);
    }

    pub fn root_scope(&self) -> &Scope {
        self.scopes.iter().find(|s| s.parent.is_none()).unwrap()
    }

    pub fn current_scope(&self) -> &Scope {
        self.scopes
            .iter()
            .find(|s| s.id == self.current_scope_id)
            .unwrap()
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes
            .iter_mut()
            .find(|s| s.id == self.current_scope_id)
            .unwrap()
    }
}
