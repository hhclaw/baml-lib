pub mod coerce_alias;
mod coerce_class;
pub mod coerce_enum;

use core::panic;

use anyhow::Result;
use internal_baml_core::ir::FieldType;

use crate::deserializer::{coercer::TypeCoercer, types::BamlValueWithFlags};

use super::{ParsingContext, ParsingError};

pub(super) enum IrRef<'a> {
    Enum(&'a String),
    Class(&'a String),
    RecursiveAlias(&'a String),
}

impl TypeCoercer for IrRef<'_> {
    fn coerce(
        &self,
        ctx: &ParsingContext,
        target: &FieldType,
        value: Option<&crate::jsonish::Value>,
    ) -> Result<BamlValueWithFlags, ParsingError> {
        match self {
            IrRef::Enum(e) => match ctx.of.find_enum(e.as_str()) {
                Ok(e) => e.coerce(ctx, target, value),
                Err(e) => Err(ctx.error_internal(e.to_string())),
            },
            IrRef::Class(c) => match ctx.of.find_class(c.as_str()) {
                Ok(c) => c.coerce(ctx, target, value),
                Err(e) => Err(ctx.error_internal(e.to_string())),
            },
            IrRef::RecursiveAlias(a) => match ctx.of.find_recursive_alias_target(a.as_str()) {
                Ok(a) => a.coerce(ctx, target, value),
                Err(e) => Err(ctx.error_internal(e.to_string())),
            },
        }
    }
}
