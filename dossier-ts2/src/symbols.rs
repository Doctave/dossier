use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;

use dossier_core::indexmap::IndexMap;

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

impl SymbolKind {
    pub fn function(&self) -> Option<&crate::function::Function> {
        match self {
            SymbolKind::Function(f) => Some(f),
        }
    }
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
    pub source: Source,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TableEntry {
    pub kind: EntryKind,
    pub fqn: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EntryKind {
    Symbol(Symbol),
    Ref(SymbolReference),
}

impl EntryKind {
    pub fn symbol(&self) -> Option<&Symbol> {
        match self {
            EntryKind::Symbol(s) => Some(s),
            _ => None,
        }
    }

    pub fn reference(&self) -> Option<&SymbolReference> {
        match self {
            EntryKind::Ref(r) => Some(r),
            _ => None,
        }
    }

    pub fn is_symbol(&self) -> bool {
        self.symbol().is_some()
    }

    pub fn is_reference(&self) -> bool {
        self.reference().is_some()
    }
}

static SCOPE_ID: AtomicUsize = AtomicUsize::new(0);

type ScopeID = usize;

/// The symbol table for a single file.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Scope {
    pub identifier: Option<String>,
    pub id: ScopeID,
    pub parent: Option<ScopeID>,
    pub entries: IndexMap<String, TableEntry>,
    pub imports: Vec<Import>,
}

#[derive(Debug, Clone, PartialEq)]
/// A module that keeps track of all the symbols and their
/// scopes in a file.
///
/// The table starts in the global scope, and new scopes
/// can be added by calling `push_scope`.
///
/// When looking up a symbol, the caller needs to know the
/// scope ID of the scope they're looking in.
pub(crate) struct SymbolTable {
    pub file: PathBuf,
    scopes: Vec<Scope>,
    current_scope_id: ScopeID,
}

impl SymbolTable {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        let root_id = SCOPE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Self {
            file: path.into(),
            current_scope_id: root_id,
            scopes: vec![Scope {
                identifier: None,
                id: root_id,
                parent: None,
                entries: IndexMap::new(),
                imports: vec![],
            }],
        }
    }

    pub fn lookup(&self, identifier: &str, scope_id: ScopeID) -> Option<&TableEntry> {
        let scope = self.scopes.iter().find(|s| s.id == scope_id).unwrap();

        scope.entries.get(identifier).or_else(|| {
            if let Some(parent_id) = scope.parent {
                self.lookup(identifier, parent_id)
            } else {
                None
            }
        })
    }

    pub fn all_entries(&self) -> impl Iterator<Item = &TableEntry> {
        self.scopes.iter().flat_map(|s| s.entries.values())
    }

    pub fn all_imports(&self) -> impl Iterator<Item = &Import> {
        self.scopes.iter().flat_map(|s| s.imports.iter())
    }

    pub fn add_symbol(&mut self, identifier: &str, symbol: Symbol) {
        let fqn = self.construct_fqn(identifier);

        self.current_scope_mut().entries.insert(
            identifier.into(),
            TableEntry {
                kind: EntryKind::Symbol(symbol),
                fqn,
            },
        );
    }

    fn construct_fqn(&self, identifier: &str) -> String {
        let mut out = format!("{}", self.file.display());

        let mut scope = self.current_scope();
        let mut parts = vec![];

        while scope.parent.is_some() {
            if let Some(ident) = &self.current_scope().identifier {
                parts.push(ident.as_str());
            }

            scope = self
                .scopes
                .iter()
                .find(|s| s.id == scope.parent.unwrap())
                .unwrap();
        }

        parts.push(identifier);

        out.push_str(&format!("::{}", parts.join("::")));

        out
    }

    pub fn add_import(&mut self, import: Import) {
        self.current_scope_mut().imports.push(import);
    }

    /// Create a new scope with the given name.
    pub fn push_scope(&mut self, name: &str) -> ScopeID {
        let id = SCOPE_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        self.scopes.push(Scope {
            id,
            identifier: Some(name.into()),
            parent: Some(self.current_scope_id),
            entries: IndexMap::new(),
            imports: vec![],
        });

        self.current_scope_id = id;
        id
    }

    /// Pop the current scope, and set the current scope to the parent.
    ///
    /// If the current scope has no parent, (aka, it's the global scope), this is a no-op.
    ///
    /// Returns the popped scope ID.
    pub fn pop_scope(&mut self) -> ScopeID {
        let current_id = self.current_scope_id;

        if let Some(parent_id) = self.current_scope().parent {
            self.current_scope_id = parent_id;
        }

        current_id
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn symbol_table_lookup() {
        let mut table = SymbolTable::new("foo.ts");
        table.add_symbol(
            "foo",
            Symbol {
                kind: SymbolKind::Function(crate::function::Function {
                    title: "foo".to_owned(),
                    documentation: None,
                    is_exported: false,
                }),
                source: Source {
                    offset_start_bytes: 0,
                    offset_end_bytes: 0,
                },
            },
        );

        let entry = table.lookup("foo", table.root_scope().id).unwrap();

        match &entry.kind {
            EntryKind::Symbol(Symbol {
                kind: SymbolKind::Function(f),
                ..
            }) => {
                assert_eq!(f.title, "foo");
            }
            _ => panic!("Expected a function symbol"),
        }
    }

    #[test]
    fn symbol_table_lookup_fails_if_no_match_in_a_parent_scope() {
        let mut table = SymbolTable::new("foo.ts");

        table.push_scope("bar");

        table.add_symbol(
            "foo",
            Symbol {
                kind: SymbolKind::Function(crate::function::Function {
                    title: "foo".to_owned(),
                    documentation: None,
                    is_exported: false,
                }),
                source: Source {
                    offset_start_bytes: 0,
                    offset_end_bytes: 0,
                },
            },
        );

        table.pop_scope();

        assert_eq!(table.lookup("foo", table.root_scope().id), None);
    }

    #[test]
    fn lookup_in_parent_scopes() {
        let mut table = SymbolTable::new("foo.ts");

        table.add_symbol(
            "foo",
            Symbol {
                kind: SymbolKind::Function(crate::function::Function {
                    title: "foo".to_owned(),
                    documentation: None,
                    is_exported: false,
                }),
                source: Source {
                    offset_start_bytes: 0,
                    offset_end_bytes: 0,
                },
            },
        );

        table.push_scope("foo");
        table.push_scope("bar");
        let nested_scope_id = table.push_scope("baz");

        assert!(table.lookup("foo", nested_scope_id).is_some());
    }

    #[test]
    fn computes_fqns_for_entries() {
        let mut table = SymbolTable::new("foo.ts");

        table.add_symbol(
            "foo",
            Symbol {
                kind: SymbolKind::Function(crate::function::Function {
                    title: "foo".to_owned(),
                    documentation: None,
                    is_exported: false,
                }),
                source: Source {
                    offset_start_bytes: 0,
                    offset_end_bytes: 0,
                },
            },
        );

        let nested_scope = table.push_scope("Fizz");

        table.add_symbol(
            "bar",
            Symbol {
                kind: SymbolKind::Function(crate::function::Function {
                    title: "bar".to_owned(),
                    documentation: None,
                    is_exported: false,
                }),
                source: Source {
                    offset_start_bytes: 0,
                    offset_end_bytes: 0,
                },
            },
        );

        assert_eq!(
            table.lookup("bar", nested_scope).unwrap().fqn,
            "foo.ts::Fizz::bar"
        );
    }
}
