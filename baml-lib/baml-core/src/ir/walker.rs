use anyhow::Result;
use baml_types::{BamlValue, EvaluationContext, UnresolvedValue};
use indexmap::IndexMap;

use internal_baml_diagnostics::Span;
use internal_baml_parser_database::RetryPolicyStrategy;
use internal_llm_client::ClientSpec;

use std::collections::{HashMap, HashSet};

use super::{
    repr::{self, FunctionConfig, WithRepr},
    Class, Client, Enum, EnumValue, Field, FunctionNode, IRHelper, Impl, RetryPolicy,
    TemplateString, TestCase, TypeAlias, Walker,
};
use crate::ir::jinja_helpers::render_expression;

impl<'a> Walker<'a, &'a FunctionNode> {
    pub fn name(&self) -> &'a str {
        self.elem().name()
    }

    pub fn is_v1(&self) -> bool {
        false
    }

    pub fn is_v2(&self) -> bool {
        true
    }

    pub fn client_name(&self) -> Option<String> {
        if let Some(c) = self.elem().configs.first() {
            return Some(c.client.as_str());
        }
        None
    }

    pub fn required_env_vars(&'a self) -> Result<HashSet<String>> {
        if let Some(c) = self.elem().configs.first() {
            match &c.client {
                ClientSpec::Named(n) => {
                    let client: super::ClientWalker<'a> = self.db.find_client(n)?;
                    Ok(client.required_env_vars())
                }
                ClientSpec::Shorthand(provider, model) => {
                    let options = IndexMap::from_iter([(
                        "model".to_string(),
                        (
                            (),
                            baml_types::UnresolvedValue::String(
                                baml_types::StringOr::Value(model.clone()),
                                (),
                            ),
                        ),
                    )]);
                    let properties = internal_llm_client::PropertyHandler::<()>::new(options, ());
                    if let Ok(client) = provider.parse_client_property(properties) {
                        Ok(client.required_env_vars())
                    } else {
                        // We likely can't make a shorthand client from the given provider
                        Ok(HashSet::new())
                    }
                }
            }
        } else {
            anyhow::bail!("Function {} has no client", self.name())
        }
    }

    pub fn walk_impls(
        &'a self,
    ) -> impl Iterator<Item = Walker<'a, (&'a repr::Function, &'a FunctionConfig)>> {
        self.elem().configs.iter().map(|c| Walker {
            db: self.db,
            item: (self.elem(), c),
        })
    }
    pub fn walk_tests(
        &'a self,
    ) -> impl Iterator<Item = Walker<'a, (&'a FunctionNode, &'a TestCase)>> {
        self.elem().tests().iter().map(|i| Walker {
            db: self.db,
            item: (self.item, i),
        })
    }

    pub fn find_test(
        &'a self,
        test_name: &str,
    ) -> Option<Walker<'a, (&'a FunctionNode, &'a TestCase)>> {
        self.walk_tests().find(|t| t.item.1.elem.name == test_name)
    }

    pub fn elem(&self) -> &'a repr::Function {
        &self.item.elem
    }

    pub fn output(&self) -> &'a baml_types::FieldType {
        self.elem().output()
    }

    pub fn inputs(&self) -> &'a Vec<(String, baml_types::FieldType)> {
        self.elem().inputs()
    }

    pub fn span(&self) -> Option<&crate::Span> {
        self.item.attributes.span.as_ref()
    }
}

impl<'a> Walker<'a, &'a Enum> {
    pub fn name(&self) -> &'a str {
        &self.elem().name
    }

    pub fn alias(&self, ctx: &EvaluationContext<'_>) -> Result<Option<String>> {
        self.item
            .attributes
            .get("alias")
            .map(|v| v.resolve_string(ctx))
            .transpose()
    }

    pub fn walk_values(&'a self) -> impl Iterator<Item = Walker<'a, &'a EnumValue>> {
        self.item.elem.values.iter().map(|v| Walker {
            db: self.db,
            item: &v.0,
        })
    }

    pub fn find_value(&self, name: &str) -> Option<Walker<'a, &'a EnumValue>> {
        self.item
            .elem
            .values
            .iter()
            .find(|v| v.0.elem.0 == name)
            .map(|v| Walker {
                db: self.db,
                item: &v.0,
            })
    }

    pub fn elem(&self) -> &'a repr::Enum {
        &self.item.elem
    }

    pub fn span(&self) -> Option<&crate::Span> {
        self.item.attributes.span.as_ref()
    }
}

impl<'a> Walker<'a, &'a EnumValue> {
    pub fn skip(&self, ctx: &EvaluationContext<'_>) -> Result<bool> {
        self.item
            .attributes
            .get("skip")
            .map(|v| v.resolve_bool(ctx))
            .unwrap_or(Ok(false))
    }

    pub fn name(&'a self) -> &'a str {
        &self.item.elem.0
    }

    pub fn alias(&self, ctx: &EvaluationContext<'_>) -> Result<Option<String>> {
        self.item
            .attributes
            .get("alias")
            .map(|v| v.resolve_string(ctx))
            .transpose()
    }

    pub fn description(&self, ctx: &EvaluationContext<'_>) -> Result<Option<String>> {
        self.item
            .attributes
            .get("description")
            .map(|v| v.resolve_string(ctx))
            .transpose()
    }
}

impl<'a> Walker<'a, (&'a FunctionNode, &'a Impl)> {
    #[allow(dead_code)]
    pub fn function(&'a self) -> Walker<'a, &'a FunctionNode> {
        Walker {
            db: self.db,
            item: self.item.0,
        }
    }

    pub fn elem(&self) -> &'a repr::Implementation {
        &self.item.1.elem
    }
}

impl<'a> Walker<'a, (&'a FunctionNode, &'a TestCase)> {
    pub fn matches(&self, function_name: &str, test_name: &str) -> bool {
        self.item.0.elem.name() == function_name && self.item.1.elem.name == test_name
    }

    pub fn name(&self) -> String {
        format!("{}::{}", self.item.0.elem.name(), self.item.1.elem.name)
    }

    pub fn args(&self) -> &IndexMap<String, UnresolvedValue<()>> {
        &self.item.1.elem.args
    }

    pub fn test_case(&self) -> &repr::TestCase {
        &self.item.1.elem
    }

    pub fn span(&self) -> Option<&crate::Span> {
        self.item.1.attributes.span.as_ref()
    }

    pub fn test_case_params(
        &self,
        ctx: &EvaluationContext<'_>,
    ) -> Result<IndexMap<String, Result<BamlValue>>> {
        self.args()
            .iter()
            .map(|(k, v)| Ok((k.clone(), v.resolve_serde::<BamlValue>(ctx))))
            .collect()
    }

    pub fn function(&'a self) -> Walker<'a, &'a FunctionNode> {
        Walker {
            db: self.db,
            item: self.item.0,
        }
    }
}

impl<'a> Walker<'a, &'a Class> {
    pub fn name(&self) -> &'a str {
        &self.elem().name
    }

    pub fn alias(&self, ctx: &EvaluationContext<'_>) -> Result<Option<String>> {
        self.item
            .attributes
            .get("alias")
            .map(|v| v.resolve_string(ctx))
            .transpose()
    }

    pub fn walk_fields(&'a self) -> impl Iterator<Item = Walker<'a, &'a Field>> {
        self.item.elem.static_fields.iter().map(|f| Walker {
            db: self.db,
            item: f,
        })
    }

    pub fn find_field(&'a self, name: &str) -> Option<Walker<'a, &'a Field>> {
        self.item
            .elem
            .static_fields
            .iter()
            .find(|f| f.elem.name == name)
            .map(|f| Walker {
                db: self.db,
                item: f,
            })
    }

    pub fn elem(&self) -> &'a repr::Class {
        &self.item.elem
    }

    pub fn span(&self) -> Option<&crate::Span> {
        self.item.attributes.span.as_ref()
    }

    pub fn inputs(&self) -> &'a Vec<(String, baml_types::FieldType)> {
        self.elem().inputs()
    }
}

impl<'a> Walker<'a, &'a TypeAlias> {
    pub fn elem(&self) -> &'a repr::TypeAlias {
        &self.item.elem
    }

    pub fn name(&self) -> &'a str {
        &self.elem().name
    }

    pub fn span(&self) -> Option<&crate::Span> {
        self.item.attributes.span.as_ref()
    }
}

impl<'a> Walker<'a, &'a Client> {
    pub fn elem(&'a self) -> &'a repr::Client {
        &self.item.elem
    }

    pub fn name(&'a self) -> &'a str {
        &self.elem().name
    }

    pub fn retry_policy(&self) -> &Option<String> {
        &self.elem().retry_policy_id
    }

    pub fn span(&self) -> Option<&crate::Span> {
        self.item.attributes.span.as_ref()
    }

    pub fn options(&'a self) -> &'a internal_llm_client::UnresolvedClientProperty<()> {
        &self.elem().options
    }

    pub fn required_env_vars(&'a self) -> HashSet<String> {
        self.options().required_env_vars()
    }
}

impl<'a> Walker<'a, &'a RetryPolicy> {
    pub fn name(&self) -> &str {
        &self.elem().name.0
    }

    pub fn elem(&self) -> &'a repr::RetryPolicy {
        &self.item.elem
    }

    pub fn max_retries(&self) -> u32 {
        self.elem().max_retries
    }

    pub fn strategy(&self) -> &RetryPolicyStrategy {
        &self.elem().strategy
    }

    pub fn span(&self) -> Option<&crate::Span> {
        self.item.attributes.span.as_ref()
    }
}

impl<'a> Walker<'a, &'a TemplateString> {
    pub fn elem(&self) -> &'a repr::TemplateString {
        &self.item.elem
    }

    pub fn name(&self) -> &str {
        self.elem().name.as_str()
    }

    pub fn inputs(&self) -> &'a Vec<repr::Field> {
        &self.item.elem.params
    }

    pub fn template(&self) -> &str {
        &self.elem().content
    }

    pub fn span(&self) -> Option<&crate::Span> {
        self.item.attributes.span.as_ref()
    }
}

impl<'a> Walker<'a, &'a Field> {
    pub fn name(&self) -> &str {
        &self.elem().name
    }

    pub fn r#type(&'a self) -> &'a baml_types::FieldType {
        &self.elem().r#type.elem
    }

    pub fn elem(&'a self) -> &'a repr::Field {
        &self.item.elem
    }

    pub fn alias(&self, ctx: &EvaluationContext<'_>) -> Result<Option<String>> {
        self.item
            .attributes
            .get("alias")
            .map(|v| v.resolve_string(ctx))
            .transpose()
    }

    pub fn description(&self, ctx: &EvaluationContext<'_>) -> Result<Option<String>> {
        self.item
            .attributes
            .get("description")
            .map(|v| v.resolve_string(ctx))
            .transpose()
    }

    pub fn span(&self) -> Option<&crate::Span> {
        self.item.attributes.span.as_ref()
    }
}
