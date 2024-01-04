use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;

use dossier_core::indexmap::IndexMap;

use crate::import::Import;
use crate::symbol::Symbol;

static SCOPE_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) type ScopeID = usize;

/// The symbol table for a single file.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Scope {
    pub identifier: Option<String>,
    pub id: ScopeID,
    pub parent: Option<ScopeID>,
    pub symbols: IndexMap<String, Symbol>,
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

#[allow(dead_code)]
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
                symbols: IndexMap::new(),
                imports: vec![],
            }],
        }
    }

    pub fn lookup(&self, identifier: &str, scope_id: ScopeID) -> Option<&Symbol> {
        let scope = self.scopes.iter().find(|s| s.id == scope_id).unwrap();

        scope.symbols.get(identifier).or_else(|| {
            if let Some(parent_id) = scope.parent {
                self.lookup(identifier, parent_id)
            } else {
                None
            }
        })
    }

    pub fn lookup_import(&self, identifier: &str, scope_id: ScopeID) -> Option<&Import> {
        let scope = self.scopes.iter().find(|s| s.id == scope_id).unwrap();

        scope
            .imports
            .iter()
            .find(|i| i.names.contains(&identifier.to_owned()))
            .or_else(|| {
                if let Some(parent_id) = scope.parent {
                    self.lookup_import(identifier, parent_id)
                } else {
                    None
                }
            })
    }

    pub fn all_symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.scopes.iter().flat_map(|s| s.symbols.values())
    }

    pub fn all_imports(&self) -> impl Iterator<Item = &Import> {
        self.scopes.iter().flat_map(|s| s.imports.iter())
    }

    pub fn add_symbol(&mut self, identifier: &str, symbol: Symbol) {
        self.current_scope_mut()
            .symbols
            .insert(identifier.into(), symbol);
    }

    /// The wildest part of this crate by a long shot. Type resolution!
    pub fn resolve_types(&mut self) {
        // First pass: collect the actions we need to apply to avoid mutable-immutable borrow conflict
        //
        // We collect a set of actions where the elements are:
        // - The scope index
        // - The root symbol identifier
        // - The chain of indexes to the child symbols which needs resolving
        // - The identifier in the symbol that needs resolving
        let mut actions: Vec<(usize, String, VecDeque<usize>, String)> = vec![];

        for (scope_index, scope) in self.scopes.iter().enumerate() {
            let index_chain = VecDeque::new();

            for (root_symbol_identifier, symbol) in scope.symbols.iter() {
                let mut chain = index_chain.clone();
                // Collect a list of IDs to symbols which need resolving
                Self::collect_actions_recursive(
                    symbol,
                    scope_index,
                    root_symbol_identifier,
                    &mut chain,
                    &mut actions,
                );
            }
        }

        let mut resolutions: Vec<(usize, String, VecDeque<usize>, String)> = vec![];
        // Second pass: perform the lookups and collect the results
        //
        // Look up the identifier from its scope. If we find a match, we add it to the resolutions,
        // which is an identical list as above, except the last element is the resolved FQN of the symbol
        for (scope_index, root_symbol_identifier, child_indexes, identifier) in actions {
            let scope_id = self.scopes[scope_index].id;

            if let Some(matching_symbol) = self.lookup(&identifier, scope_id) {
                resolutions.push((
                    scope_index,
                    root_symbol_identifier,
                    child_indexes,
                    matching_symbol.fqn.clone(),
                ));
            }
        }

        // Third pass: apply the resolutions back to the symbols
        for (scope_index, root_symbol_identifier, indexes, fqn) in resolutions {
            if let Some(symbol) = self
                .scopes
                .get_mut(scope_index)
                .and_then(|s| s.symbols.get_mut(root_symbol_identifier.as_str()))
            {
                let symbol = Self::resolve_symbol_mut(symbol, indexes); // Use slicing to pass the rest of the indexes
                symbol.resolve_type(&fqn);
            }
        }
    }

    /// Same as `resolve_types`, but resolves imports across files.
    pub fn resolve_imported_types<'a, T: IntoIterator<Item = &'a SymbolTable>>(
        &mut self,
        all_tables: T,
    ) {
        // First pass: collect the actions we need to apply to avoid mutable-immutable borrow conflict
        //
        // We collect a set of actions where the elements are:
        // - The scope index
        // - The root symbol identifier
        // - The chain of indexes to the child symbols which needs resolving
        // - The identifier in the symbol that needs resolving
        let mut actions: Vec<(usize, String, VecDeque<usize>, String)> = vec![];

        for (scope_index, scope) in self.scopes.iter().enumerate() {
            let index_chain = VecDeque::new();

            for (root_symbol_identifier, symbol) in scope.symbols.iter() {
                let mut chain = index_chain.clone();
                // Collect a list of IDs to symbols which need resolving
                Self::collect_actions_recursive(
                    symbol,
                    scope_index,
                    root_symbol_identifier,
                    &mut chain,
                    &mut actions,
                );
            }
        }

        let mut resolutions: Vec<(usize, String, VecDeque<usize>, String)> = vec![];
        let mut all_tables = all_tables.into_iter();
        // Second pass: perform the lookups and collect the results
        //
        // Look up the identifier from its scope. If we find a match, we add it to the resolutions,
        // which is an identical list as above, except the last element is the resolved FQN of the symbol
        for (scope_index, root_symbol_identifier, child_indexes, identifier) in actions {
            let scope_id = self.scopes[scope_index].id;

            if let Some(import) = self.lookup_import(&identifier, scope_id) {
                if let Some(imported_table) =
                    all_tables.find(|t| self.matches_import_path(&t.file, import))
                {
                    if let Some(matching_symbol) =
                        imported_table.lookup(&identifier, imported_table.root_scope().id)
                    {
                        resolutions.push((
                            scope_index,
                            root_symbol_identifier,
                            child_indexes,
                            matching_symbol.fqn.clone(),
                        ));
                    }
                }
            }
        }

        // Third pass: apply the resolutions back to the symbols
        for (scope_index, root_symbol_identifier, indexes, fqn) in resolutions {
            if let Some(symbol) = self
                .scopes
                .get_mut(scope_index)
                .and_then(|s| s.symbols.get_mut(root_symbol_identifier.as_str()))
            {
                let symbol = Self::resolve_symbol_mut(symbol, indexes); // Use slicing to pass the rest of the indexes
                symbol.resolve_type(&fqn);
            }
        }
    }

    /// Helper function to recursively collect a list of actions to perform=
    /// during type resolution.
    fn collect_actions_recursive(
        symbol: &Symbol,
        scope_index: usize,
        root_symbol_identifier: &str,
        chain: &mut VecDeque<usize>,
        actions: &mut Vec<(usize, String, VecDeque<usize>, String)>,
    ) {
        if let Some(resolvable_identifier) = symbol.resolvable_identifier() {
            actions.push((
                scope_index,
                root_symbol_identifier.to_owned(),
                chain.clone(),
                resolvable_identifier.to_owned(),
            ));
        }

        for (child_index, child) in symbol.children().iter().enumerate() {
            let mut chain = chain.clone();
            chain.push_back(child_index);

            Self::collect_actions_recursive(
                child,
                scope_index,
                root_symbol_identifier,
                &mut chain,
                actions,
            );
        }
    }

    /// Helper function to recursively resolve a symbol based on a list of nested indexes.
    fn resolve_symbol_mut(symbol: &mut Symbol, mut indexes: VecDeque<usize>) -> &mut Symbol {
        if let Some(index) = indexes.pop_front() {
            Self::resolve_symbol_mut(symbol.children_mut().get_mut(index).unwrap(), indexes)
        } else {
            symbol
        }
    }

    /// Returns true if the import path resolves to the symbol table's path
    /// from the perspective of the current symbol table's path.
    ///
    /// i.e. if a file `foo/bar.ts` imports `../fizz.ts`, this function
    /// returns true for symbol table with the path `fizz.ts`.
    fn matches_import_path(&self, symbol_table_path: &Path, import: &Import) -> bool {
        // Get the directory of the current symbol table's file
        let base_path = self.file.parent().unwrap_or_else(|| Path::new(""));

        // Combine base path with the relative path from import
        let combined_path = base_path.join(&import.source);

        // Normalize the combined path
        let normalized_path = self.normalize_path(&combined_path);

        // Compare the normalized paths
        normalized_path == symbol_table_path
    }

    // Helper function to normalize a path
    fn normalize_path(&self, path: &Path) -> PathBuf {
        let mut components = path.components().peekable();
        let mut normalized_path = PathBuf::new();

        while let Some(component) = components.next() {
            match component {
                std::path::Component::ParentDir => {
                    // If there's a previous component and it's not "..", go up one level
                    if let Some(std::path::Component::Normal(_)) = components.peek() {
                        normalized_path.pop();
                    } else {
                        normalized_path.push("..");
                    }
                }
                std::path::Component::Normal(part) => normalized_path.push(part),
                _ => {} // Ignore other components (RootDir, CurDir, Prefix)
            }
        }

        normalized_path
    }

    /// Constructs a fully qualified name for the given identifier in the current scope.
    pub fn construct_fqn(&self, identifier: &str) -> String {
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
            symbols: IndexMap::new(),
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
    use crate::symbol::{Source, SymbolKind};

    #[test]
    fn symbol_table_lookup() {
        let mut table = SymbolTable::new("foo.ts");
        table.add_symbol(
            "foo",
            Symbol {
                kind: SymbolKind::Function(crate::function::Function {
                    identifier: "foo".to_owned(),
                    documentation: None,
                    is_exported: false,
                    children: vec![],
                }),
                source: Source {
                    file: PathBuf::from("foo.ts"),
                    offset_start_bytes: 0,
                    offset_end_bytes: 0,
                },
                fqn: "foo.ts::foo".to_owned(),
                context: None,
            },
        );

        let symbol = table.lookup("foo", table.root_scope().id).unwrap();

        match &symbol {
            Symbol {
                kind: SymbolKind::Function(f),
                ..
            } => {
                assert_eq!(f.identifier, "foo");
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
                    identifier: "foo".to_owned(),
                    documentation: None,
                    is_exported: false,
                    children: vec![],
                }),
                source: Source {
                    file: PathBuf::from("foo.ts"),
                    offset_start_bytes: 0,
                    offset_end_bytes: 0,
                },
                fqn: "foo.ts::foo".to_owned(),
                context: None,
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
                    identifier: "foo".to_owned(),
                    documentation: None,
                    is_exported: false,
                    children: vec![],
                }),
                source: Source {
                    file: PathBuf::from("foo.ts"),
                    offset_start_bytes: 0,
                    offset_end_bytes: 0,
                },
                fqn: "foo.ts::foo".to_owned(),
                context: None,
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

        assert_eq!(table.construct_fqn("foo"), "foo.ts::foo");

        let _nested_scope = table.push_scope("Fizz");

        assert_eq!(table.construct_fqn("foo"), "foo.ts::Fizz::foo");
    }
}
