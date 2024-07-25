use anyhow::{anyhow, bail, Result};
use baml_types::FieldType;
use either::Either;

use indexmap::IndexMap;
use internal_baml_parser_database::{
    walkers::{ClassWalker, EnumValueWalker, EnumWalker, FieldWalker, TemplateStringWalker},
    ParserDatabase, PromptAst, ToStringAttributes, WithStaticRenames,
};

use internal_baml_schema_ast::ast::{self, FieldArity, WithName, WithSpan};
use serde::Serialize;

use crate::Configuration;

/// This class represents the intermediate representation of the BAML AST.
/// It is a representation of the BAML AST that is easier to work with than the
/// raw BAML AST, and should include all information necessary to generate
/// code in any target language.
#[derive(serde::Serialize, Debug)]
pub struct IntermediateRepr {
    enums: Vec<Node<Enum>>,
    classes: Vec<Node<Class>>,
    template_strings: Vec<Node<TemplateString>>,

    #[serde(skip)]
    configuration: Configuration,
}

/// A generic walker. Only walkers instantiated with a concrete ID type (`I`) are useful.
#[derive(Clone, Copy)]
pub struct Walker<'db, I> {
    /// The parser database being traversed.
    pub db: &'db IntermediateRepr,
    /// The identifier of the focused element.
    pub item: I,
}

impl IntermediateRepr {
    pub fn create_empty() -> IntermediateRepr {
        IntermediateRepr {
            enums: vec![],
            classes: vec![],
            template_strings: vec![],
            configuration: Configuration::new(),
        }
    }

    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    pub fn walk_enums<'a>(&'a self) -> impl ExactSizeIterator<Item = Walker<'a, &'a Node<Enum>>> {
        self.enums.iter().map(|e| Walker { db: self, item: e })
    }

    pub fn walk_classes<'a>(
        &'a self,
    ) -> impl ExactSizeIterator<Item = Walker<'a, &'a Node<Class>>> {
        self.classes.iter().map(|e| Walker { db: self, item: e })
    }

    pub fn walk_template_strings<'a>(
        &'a self,
    ) -> impl ExactSizeIterator<Item = Walker<'a, &'a Node<TemplateString>>> {
        self.template_strings
            .iter()
            .map(|e| Walker { db: self, item: e })
    }

    pub fn from_parser_database(
        db: &ParserDatabase,
        configuration: Configuration,
    ) -> Result<IntermediateRepr> {
        let mut repr = IntermediateRepr {
            enums: db
                .walk_enums()
                .map(|e| e.node(db))
                .collect::<Result<Vec<_>>>()?,
            classes: db
                .walk_classes()
                .map(|e| e.node(db))
                .collect::<Result<Vec<_>>>()?,
            template_strings: db
                .walk_templates()
                .map(|e| e.node(db))
                .collect::<Result<Vec<_>>>()?,
            configuration,
        };

        // Sort each item by name.
        repr.enums.sort_by(|a, b| a.elem.name.cmp(&b.elem.name));
        repr.classes.sort_by(|a, b| a.elem.name.cmp(&b.elem.name));

        Ok(repr)
    }
}

// TODO:
//
//   [x] clients - need to finish expressions
//   [x] metadata per node (attributes, spans, etc)
//           block-level attributes on enums, classes
//           field-level attributes on enum values, class fields
//           overrides can only exist in impls
//   [x] FieldArity (optional / required) needs to be handled
//   [x] other types of identifiers?
//   [ ] `baml update` needs to update lockfile right now
//          but baml CLI is installed globally
//   [ ] baml configuration - retry policies, generator, etc
//          [x] retry policies
//   [x] rename lockfile/mod.rs to ir/mod.rs
//   [x] wire Result<> type through, need this to be more sane

#[derive(Debug, serde::Serialize)]
pub struct NodeAttributes {
    /// Map of attributes on the corresponding IR node.
    ///
    /// Some follow special conventions:
    ///
    ///   - @skip becomes ("skip", bool)
    ///   - @alias(...) becomes ("alias", ...)
    ///   - @get(python code) becomes ("get/python", python code)
    #[serde(with = "indexmap::map::serde_seq")]
    meta: IndexMap<String, Expression>,

    // Spans
    #[serde(skip)]
    pub span: Option<ast::Span>,
}

impl NodeAttributes {
    pub fn get(&self, key: &str) -> Option<&Expression> {
        self.meta.get(key)
    }
}

fn to_ir_attributes(
    db: &ParserDatabase,
    maybe_ast_attributes: Option<&ToStringAttributes>,
) -> IndexMap<String, Expression> {
    let mut attributes = IndexMap::new();

    if let Some(ast_attributes) = maybe_ast_attributes {
        match ast_attributes {
            ToStringAttributes::Static(s) => {
                if let Some(true) = s.dynamic_type() {
                    attributes.insert("dynamic_type".to_string(), Expression::Bool(true));
                }

                if let Some(skip) = s.skip() {
                    attributes.insert("skip".to_string(), Expression::Bool(*skip));
                }
                if let Some(v) = s.alias() {
                    attributes.insert("alias".to_string(), Expression::String(db[*v].to_string()));
                }
                for (&k, &v) in s.meta().into_iter() {
                    attributes.insert(db[k].to_string(), Expression::String(db[v].to_string()));
                }
            }
            ToStringAttributes::Dynamic(d) => {
                for (&lang, &lang_code) in d.code.iter() {
                    attributes.insert(
                        format!("get/{}", db[lang].to_string()),
                        Expression::String(db[lang_code].to_string()),
                    );
                }
            }
        }
    }

    attributes
}

/// Nodes allow attaching metadata to a given IR entity: attributes, source location, etc
#[derive(serde::Serialize, Debug)]
pub struct Node<T> {
    pub attributes: NodeAttributes,
    pub elem: T,
}

/// Implement this for every node in the IR AST, where T is the type of IR node
pub trait WithRepr<T> {
    /// Represents block or field attributes - @@ for enums and classes, @ for enum values and class fields
    fn attributes(&self, _: &ParserDatabase) -> NodeAttributes {
        NodeAttributes {
            meta: IndexMap::new(),
            span: None,
        }
    }

    fn repr(&self, db: &ParserDatabase) -> Result<T>;

    fn node(&self, db: &ParserDatabase) -> Result<Node<T>> {
        Ok(Node {
            elem: self.repr(db)?,
            attributes: self.attributes(db),
        })
    }
}

fn type_with_arity(t: FieldType, arity: &FieldArity) -> FieldType {
    match arity {
        FieldArity::Required => t,
        FieldArity::Optional => FieldType::Optional(Box::new(t)),
    }
}

impl WithRepr<FieldType> for ast::FieldType {
    fn repr(&self, db: &ParserDatabase) -> Result<FieldType> {
        Ok(match self {
            ast::FieldType::Identifier(arity, idn) => type_with_arity(
                match idn {
                    ast::Identifier::Primitive(t, ..) => FieldType::Primitive(*t),
                    ast::Identifier::Local(name, _) => match db.find_type(idn) {
                        Some(Either::Left(_class_walker)) => Ok(FieldType::Class(name.clone())),
                        Some(Either::Right(_enum_walker)) => Ok(FieldType::Enum(name.clone())),
                        None => Err(anyhow!("Field type uses unresolvable local identifier")),
                    }?,
                    _ => bail!("Field type uses unsupported identifier type"),
                },
                arity,
            ),
            ast::FieldType::List(ft, dims, _) => {
                // NB: potential bug: this hands back a 1D list when dims == 0
                let mut repr = FieldType::List(Box::new(ft.repr(db)?));

                for _ in 1u32..*dims {
                    repr = FieldType::List(Box::new(repr));
                }

                repr
            }
            ast::FieldType::Dictionary(kv, _) => {
                // NB: we can't just unpack (*kv) into k, v because that would require a move/copy
                FieldType::Map(Box::new((*kv).0.repr(db)?), Box::new((*kv).1.repr(db)?))
            }
            ast::FieldType::Union(arity, t, _) => {
                // NB: preempt union flattening by mixing arity into union types
                let mut types = t.iter().map(|ft| ft.repr(db)).collect::<Result<Vec<_>>>()?;

                if arity.is_optional() {
                    types.push(FieldType::Primitive(baml_types::TypeValue::Null));
                }

                FieldType::Union(types)
            }
            ast::FieldType::Tuple(arity, t, _) => type_with_arity(
                FieldType::Tuple(t.iter().map(|ft| ft.repr(db)).collect::<Result<Vec<_>>>()?),
                arity,
            ),
        })
    }
}

#[derive(serde::Serialize, Debug)]
pub enum Identifier {
    /// Starts with env.*
    ENV(String),
    /// The path to a Local Identifer + the local identifer. Separated by '.'
    #[allow(dead_code)]
    Ref(Vec<String>),
    /// A string without spaces or '.' Always starts with a letter. May contain numbers
    Local(String),
    /// Special types (always lowercase).
    Primitive(baml_types::TypeValue),
}

impl Identifier {
    pub fn name(&self) -> String {
        match self {
            Identifier::ENV(k) => k.clone(),
            Identifier::Ref(r) => r.join("."),
            Identifier::Local(l) => l.clone(),
            Identifier::Primitive(p) => p.to_string(),
        }
    }
}

#[derive(serde::Serialize, Debug)]
pub enum Expression {
    Identifier(Identifier),
    Bool(bool),
    Numeric(String),
    String(String),
    RawString(String),
    List(Vec<Expression>),
    Map(Vec<(Expression, Expression)>),
}

impl Expression {
    pub fn required_env_vars(&self) -> Vec<&str> {
        match self {
            Expression::Identifier(Identifier::ENV(k)) => vec![k.as_str()],
            Expression::List(l) => l.iter().flat_map(Expression::required_env_vars).collect(),
            Expression::Map(m) => m
                .iter()
                .flat_map(|(k, v)| {
                    let mut keys = k.required_env_vars();
                    keys.extend(v.required_env_vars());
                    keys
                })
                .collect(),
            _ => vec![],
        }
    }
}

impl WithRepr<Expression> for ast::Expression {
    fn repr(&self, db: &ParserDatabase) -> Result<Expression> {
        Ok(match self {
            ast::Expression::BoolValue(val, _) => Expression::Bool(val.clone()),
            ast::Expression::NumericValue(val, _) => Expression::Numeric(val.clone()),
            ast::Expression::StringValue(val, _) => Expression::String(val.clone()),
            ast::Expression::RawStringValue(val) => Expression::RawString(val.value().to_string()),
            ast::Expression::Identifier(idn) => match idn {
                ast::Identifier::ENV(k, _) => {
                    Ok(Expression::Identifier(Identifier::ENV(k.clone())))
                }
                ast::Identifier::String(s, _) => Ok(Expression::String(s.clone())),
                ast::Identifier::Local(l, _) => {
                    Ok(Expression::Identifier(Identifier::Local(l.clone())))
                }
                ast::Identifier::Ref(r, _) => {
                    // NOTE(sam): this feels very very wrong, but per vbv, we don't really use refs
                    // right now, so this should be safe. this is done to ensure that
                    // "options { model gpt-3.5-turbo }" is represented correctly in the resulting IR,
                    // specifically that "gpt-3.5-turbo" is actually modelled as Expression::String
                    //
                    // this does not impact the handling of "options { api_key env.OPENAI_API_KEY }"
                    // because that's modelled as Identifier::ENV, not Identifier::Ref
                    Ok(Expression::String(r.full_name.clone()))
                }
                ast::Identifier::Primitive(p, _) => {
                    Ok(Expression::Identifier(Identifier::Primitive(*p)))
                }
                ast::Identifier::Invalid(_, _) => {
                    Err(anyhow!("Cannot represent an invalid parser-AST identifier"))
                }
            }?,
            ast::Expression::Array(arr, _) => {
                Expression::List(arr.iter().map(|e| e.repr(db)).collect::<Result<Vec<_>>>()?)
            }
            ast::Expression::Map(arr, _) => Expression::Map(
                arr.iter()
                    .map(|(k, v)| Ok((k.repr(db)?, v.repr(db)?)))
                    .collect::<Result<Vec<_>>>()?,
            ),
        })
    }
}

type TemplateStringId = String;

#[derive(serde::Serialize, Debug)]

pub struct TemplateString {
    pub name: TemplateStringId,
    pub params: Vec<Field>,
    pub content: String,
}

impl WithRepr<TemplateString> for TemplateStringWalker<'_> {
    fn attributes(&self, _: &ParserDatabase) -> NodeAttributes {
        NodeAttributes {
            meta: Default::default(),
            span: Some(self.span().clone()),
        }
    }

    fn repr(&self, _db: &ParserDatabase) -> Result<TemplateString> {
        Ok(TemplateString {
            name: self.name().to_string(),
            params: self.ast_node().input().map_or(vec![], |e| match e {
                ast::FunctionArgs::Named(arg_list) => arg_list
                    .args
                    .iter()
                    .filter_map(|(id, arg)| {
                        arg.field_type
                            .node(_db)
                            .map(|f| Field {
                                name: id.name().to_string(),
                                r#type: f,
                            })
                            .ok()
                    })
                    .collect::<Vec<_>>(),
                ast::FunctionArgs::Unnamed(_) => {
                    vec![]
                }
            }),
            content: self.template_string().to_string(),
        })
    }
}

type EnumId = String;

#[derive(serde::Serialize, Debug)]
pub struct EnumValue(pub String);

#[derive(serde::Serialize, Debug)]
pub struct Enum {
    pub name: EnumId,
    pub values: Vec<Node<EnumValue>>,
}

impl WithRepr<EnumValue> for EnumValueWalker<'_> {
    fn attributes(&self, db: &ParserDatabase) -> NodeAttributes {
        NodeAttributes {
            meta: to_ir_attributes(db, self.get_default_attributes()),
            span: Some(self.span().clone()),
        }
    }

    fn repr(&self, _db: &ParserDatabase) -> Result<EnumValue> {
        Ok(EnumValue(self.name().to_string()))
    }
}

impl WithRepr<Enum> for EnumWalker<'_> {
    fn attributes(&self, db: &ParserDatabase) -> NodeAttributes {
        let mut attributes = NodeAttributes {
            meta: to_ir_attributes(db, self.get_default_attributes()),
            span: Some(self.span().clone()),
        };

        attributes.meta = to_ir_attributes(db, self.get_default_attributes());

        attributes
    }

    fn repr(&self, db: &ParserDatabase) -> Result<Enum> {
        Ok(Enum {
            name: self.name().to_string(),
            values: self
                .values()
                .map(|v| v.node(db))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(serde::Serialize, Debug)]
pub struct Field {
    pub name: String,
    pub r#type: Node<FieldType>,
}

impl WithRepr<Field> for FieldWalker<'_> {
    fn attributes(&self, db: &ParserDatabase) -> NodeAttributes {
        NodeAttributes {
            meta: to_ir_attributes(db, self.get_default_attributes()),
            span: Some(self.span().clone()),
        }
    }

    fn repr(&self, db: &ParserDatabase) -> Result<Field> {
        Ok(Field {
            name: self.name().to_string(),
            r#type: self.ast_field().field_type.node(db)?,
        })
    }
}

type ClassId = String;

#[derive(serde::Serialize, Debug)]
pub struct Class {
    pub name: ClassId,
    pub static_fields: Vec<Node<Field>>,
    pub dynamic_fields: Vec<Node<Field>>,
}

impl WithRepr<Class> for ClassWalker<'_> {
    fn attributes(&self, db: &ParserDatabase) -> NodeAttributes {
        let mut attributes = NodeAttributes {
            meta: to_ir_attributes(db, self.get_default_attributes()),
            span: Some(self.span().clone()),
        };

        attributes.meta = to_ir_attributes(db, self.get_default_attributes());

        attributes
    }

    fn repr(&self, db: &ParserDatabase) -> Result<Class> {
        Ok(Class {
            name: self.name().to_string(),
            static_fields: self
                .static_fields()
                .map(|e| e.node(db))
                .collect::<Result<Vec<_>>>()?,
            dynamic_fields: self
                .dynamic_fields()
                .map(|e| e.node(db))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(serde::Serialize, Debug)]
pub enum OracleType {
    LLM,
}
#[derive(serde::Serialize, Debug)]
pub struct AliasOverride {
    pub name: String,
    // This is used to generate deserializers with aliased keys (see .overload in python deserializer)
    pub aliased_keys: Vec<AliasedKey>,
}

// TODO, also add skips
#[derive(serde::Serialize, Debug)]
pub struct AliasedKey {
    pub key: String,
    pub alias: Expression,
}

#[derive(Debug, Clone, Serialize)]
pub enum Prompt {
    // The prompt stirng, and a list of input replacer keys (raw key w/ magic string, and key to replace with)
    String(String, Vec<(String, String)>),

    // same thing, the chat message, and the replacer input keys (raw key w/ magic string, and key to replace with)
    Chat(Vec<ChatMessage>, Vec<(String, String)>),
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct ChatMessage {
    pub idx: u32,
    pub role: String,
    pub content: String,
}

impl WithRepr<Prompt> for PromptAst<'_> {
    fn repr(&self, _db: &ParserDatabase) -> Result<Prompt> {
        Ok(match self {
            PromptAst::String(content, _) => Prompt::String(content.clone(), vec![]),
            PromptAst::Chat(messages, input_replacers) => Prompt::Chat(
                messages
                    .iter()
                    .filter_map(|(message, content)| {
                        message.as_ref().map(|m| ChatMessage {
                            idx: m.idx,
                            role: m.role.0.clone(),
                            content: content.clone(),
                        })
                    })
                    .collect::<Vec<_>>(),
                input_replacers.to_vec(),
            ),
        })
    }
}

// impl ChatBlock {
//     /// Unique Key
//     pub fn key(&self) -> String {
//         format!("{{//BAML_CLIENT_REPLACE_ME_CHAT_MAGIC_{}//}}", self.idx)
//     }
// }
