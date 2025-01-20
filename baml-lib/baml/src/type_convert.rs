pub use internal_baml_core::{
    self,
    ast,
    internal_baml_parser_database::{ParserDatabase, TypeWalker},
};
use internal_baml_core::ast::Identifier;
use baml_types;


// added by LMNR team to convert walker `FieldType`s to actual `baml_types::FieldType`s
/// Convert ast FieldType to raw FieldType
pub fn to_raw_field_type(ft: &ast::FieldType, db: &ParserDatabase) -> baml_types::FieldType {
    match ft {
        ast::FieldType::Symbol(arity, identifier, _) => {
            let inner = match identifier {
                Identifier::ENV(_, _) => {
                    baml_types::FieldType::Primitive(baml_types::TypeValue::String)
                }
                Identifier::Ref(x, _) => match db.find_type(identifier) {
                    None => baml_types::FieldType::Primitive(baml_types::TypeValue::Null),
                    Some(TypeWalker::Class(_)) => baml_types::FieldType::Class(x.full_name.clone()),
                    Some(TypeWalker::Enum(_)) => {
                        baml_types::FieldType::Primitive(baml_types::TypeValue::String)
                    }
                    Some(TypeWalker::TypeAlias(_)) => {
                        baml_types::FieldType::RecursiveTypeAlias(x.full_name.clone())
                    }
                },
                Identifier::Local(x, _) => match db.find_type(identifier) {
                    None => baml_types::FieldType::Primitive(baml_types::TypeValue::Null),
                    Some(TypeWalker::Class(_c)) => baml_types::FieldType::Class(x.clone()),
                    Some(TypeWalker::Enum(_e)) => baml_types::FieldType::Enum(x.clone()),
                    Some(TypeWalker::TypeAlias(_t)) => {
                        baml_types::FieldType::RecursiveTypeAlias(x.clone())
                    }
                },
                //Identifier::Primitive(idx, _) => baml_types::FieldType::Primitive(idx.clone()),
                Identifier::String(_, _) => {
                    baml_types::FieldType::Primitive(baml_types::TypeValue::String)
                }
                Identifier::Invalid(_, _) => {
                    baml_types::FieldType::Primitive(baml_types::TypeValue::Null)
                }
            };
            if arity.is_optional() {
                baml_types::FieldType::Optional(Box::new(inner))
            } else {
                inner
            }
        }
        ast::FieldType::Primitive(arity, type_value, _, _) => {
            let inner = baml_types::FieldType::Primitive(*type_value);
            if arity.is_optional() {
                baml_types::FieldType::Optional(Box::new(inner))
            } else {
                inner
            }
        }
        ast::FieldType::Literal(arity, literal_value, _, _) => {
            let inner = baml_types::FieldType::Literal(literal_value.clone());
            if arity.is_optional() {
                baml_types::FieldType::Optional(Box::new(inner))
            } else {
                inner
            }
        } 
        ast::FieldType::List(arity, inner, dims, _, _) => {
            let mut t = to_raw_field_type(inner, db);
            for _ in 0..*dims {
                t = baml_types::FieldType::List(Box::new(t));
            }
            if arity.is_optional() {
                baml_types::FieldType::Optional(Box::new(t))
            } else {
                t
            }
        }
        ast::FieldType::Tuple(arity, inner, _, _) => {
            let t = baml_types::FieldType::Tuple(
                inner
                    .iter()
                    .map(|e| to_raw_field_type(e, db))
                    .collect::<Vec<_>>(),
            );
            if arity.is_optional() {
                baml_types::FieldType::Optional(Box::new(t))
            } else {
                t
            }
        }
        ast::FieldType::Union(arity, inner, _, _) => {
            let t = baml_types::FieldType::Union(
                inner
                    .iter()
                    .map(|e| to_raw_field_type(e, db))
                    .collect::<Vec<_>>(),
            );
            if arity.is_optional() {
                baml_types::FieldType::Optional(Box::new(t))
            } else {
                t
            }
        }
        ast::FieldType::Map(arity, inner, _, _) => {
            let t = baml_types::FieldType::Map(
                Box::new(to_raw_field_type(&inner.0, db)),
                Box::new(to_raw_field_type(&inner.1, db)),
            );
            if arity.is_optional() {
                baml_types::FieldType::Optional(Box::new(t))
            } else {
                t
            }
        }
    }
}
