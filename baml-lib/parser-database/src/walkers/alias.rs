use std::collections::HashSet;

use super::TypeWalker;
use internal_baml_diagnostics::Span;
use internal_baml_schema_ast::ast::{self, FieldType, Identifier, WithName, WithSpan};

/// Type alias walker
pub type TypeAliasWalker<'db> = super::Walker<'db, ast::TypeAliasId>;

impl<'db> TypeAliasWalker<'db> {
    /// Name of the type alias.
    pub fn name(&self) -> &str {
        &self.db.ast[self.id].identifier.name()
    }

    /// Identifier span.
    pub fn span(&self) -> &Span {
        self.db.ast[self.id].identifier.span()
    }

    /// Returns the field type that the alias points to.
    pub fn target(&self) -> &'db FieldType {
        &self.db.ast[self.id].value
    }

    /// Returns a "virtual" type that represents the fully resolved alias.
    ///
    /// Since an alias can point to other aliases we might have to create a
    /// type that does not exist in the AST.
    pub fn resolved(&self) -> &'db FieldType {
        &self.db.types.resolved_type_aliases[&self.id]
    }

    /// Add to Jinja types.
    pub fn add_to_types(self, types: &mut internal_baml_jinja_types::PredefinedTypes) {
        types.add_alias(self.name(), self.db.to_jinja_type(&self.target()))
    }
}
