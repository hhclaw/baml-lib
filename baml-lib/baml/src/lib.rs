#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms, unsafe_code)]

use baml_types::{BamlValue, FieldType};
use either::Either;
use internal_baml_core::ast::{WithAttributes, WithName};
pub use internal_baml_core::{
    self,
    internal_baml_diagnostics::{self, Diagnostics, SourceFile},
    internal_baml_parser_database::{self},
    Configuration, ValidatedSchema,
};
use internal_baml_jinja::types::{OutputFormatContent, RenderOptions};

/// Parse and analyze a Prisma schema.
// pub fn parse_and_validate_schema(
//     root_path: &PathBuf,
//     files: impl Into<Vec<SourceFile>>,
// ) -> Result<ValidatedSchema, Diagnostics> {
//     let mut schema = validate(root_path, files.into());
//     schema.diagnostics.to_result()?;
//     Ok(schema)
// }

/// The most general API for dealing with Prisma schemas. It accumulates what analysis and
/// validation information it can, and returns it along with any error and warning diagnostics.
pub fn validate(schema_string: &String) -> ValidatedSchema {
    internal_baml_core::validate(schema_string)
}

// -------------------------------------------------------------------------------------------------
// UNCOMMENT THIS BLOCK TO ENABLE PYTHON INTERFACE
// Laminar specific Python interface

use pyo3::prelude::PyModuleMethods;
use python_interface::{render_prompt, validate_result};
mod python_interface;

#[pyo3::prelude::pymodule]
fn baml_lib(m: &pyo3::Bound<'_, pyo3::prelude::PyModule>) -> pyo3::PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(render_prompt, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(validate_result, m)?)?;
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Exported structs and functions

/// The context around a BAML schema.
#[derive(Debug)]
pub struct BamlContext {
    /// The prompt prefix for the language model.
    pub format: OutputFormatContent,
    /// Target output: one of `FieldType::Enum` and `FieldType::Class`.
    pub target: FieldType,
    /// The validated schema.
    pub validated_schema: ValidatedSchema,
}

impl BamlContext {
    /// try to build a `BamlContext` from a schema string and an optional target name.
    pub fn try_from_schema(
        schema_string: &String,
        target_name: Option<String>,
    ) -> anyhow::Result<Self> {
        let validated_schema = validate(schema_string);
        let diagnostics = &validated_schema.diagnostics;
        if diagnostics.has_errors() {
            let formatted_error = diagnostics.to_pretty_string();
            return Err(anyhow::anyhow!(formatted_error));
        }
        let target = Self::build_target_type(&validated_schema, target_name)?;
        let format = Self::build_output_format(&validated_schema, target.clone());
        Ok(Self {
            format,
            target,
            validated_schema,
        })
    }

    /// Render the prompt prefix for the output.
    pub fn render_prompt(&self, prefix: Option<String>, always_hoist_enums: Option<bool>) -> anyhow::Result<String> {
        let output = self.format.render(RenderOptions::new(
            prefix.map(Some),
            None,
            None,
            always_hoist_enums,
        ))?;

        Ok(output.unwrap_or_default())
    }

    /// Check the LLM output for validity.
    pub fn validate_result(&self, result: &String, allow_partials: bool) -> anyhow::Result<String> {
        let result = jsonish::from_str(&self.format, &self.target, &result, allow_partials);
        result.map(|r| {
            let baml_value: BamlValue = r.into();
            // BAML serializes values using `serde_json::json!` which adds quotes around strings.
            // Enum result is a JSON string, so remove quotes around it.
            baml_value
                .serialize_json()
                .to_string()
                .trim_matches('"')
                .to_string()
        })
    }

    fn build_target_type(
        validated_schema: &ValidatedSchema,
        target_name: Option<String>,
    ) -> anyhow::Result<FieldType> {
        let target = if let Some(target_name) = &target_name {
            let target = validated_schema.db.find_type_by_str(target_name).unwrap();
            match target {
                Either::Left(cl) => FieldType::Class(cl.ast_class().name.name().to_string()),
                Either::Right(enm) => FieldType::Enum(enm.ast_enum().name.name().to_string()),
            }
        } else {
            let first_class = validated_schema.db.walk_classes().next();
            let first_enum = validated_schema.db.walk_enums().next();
            if first_class.is_none() && first_enum.is_none() {
                return Err(anyhow::anyhow!(
                    "No BAML `class` or `enum` found in the schema"
                ));
            }
            if let Some(cl) = first_class {
                FieldType::Class(cl.ast_class().name.name().to_string())
            } else {
                FieldType::Enum(first_enum.unwrap().ast_enum().name.name().to_string())
            }
        };

        Ok(target)
    }

    fn build_output_format(
        validated_schema: &ValidatedSchema,
        target: FieldType,
    ) -> OutputFormatContent {
        let enums = validated_schema
            .db
            .walk_enums()
            .map(|e| {
                let ast_enum = e.ast_enum();
                let values = ast_enum
                    .iter_values()
                    .map(|(_id, v)| {
                        let name = internal_baml_jinja::Name::new(v.name().to_string());
                        let description = v
                            .attributes()
                            .iter()
                            .find(|a| a.name() == "description")
                            .and_then(|a| a.arguments.iter().next())
                            .and_then(|(_id, val)| val.value.as_string_value())
                            .map(|ast_string_val| ast_string_val.0.to_string());
                        // let doc = v.documentation().map(|d| d.to_string());
                        (name, description)
                    })
                    .collect::<Vec<_>>();
                internal_baml_jinja::Enum {
                    name: ast_enum.name.name().to_string().into(),
                    values,
                }
            })
            .collect::<Vec<_>>();

        let classes = validated_schema
            .db
            .walk_classes()
            .map(|c| {
                let ast_class = c.ast_class();
                let fields = ast_class
                    .iter_fields()
                    .map(|(_id, f)| {
                        let name = internal_baml_jinja::Name::new(f.name().to_string());
                        let t = validated_schema.db.to_raw_field_type(&f.field_type);
                        let description = f
                            .attributes()
                            .iter()
                            .find(|a| a.name() == "description")
                            .and_then(|a| a.arguments.iter().next())
                            .and_then(|(_id, val)| val.value.as_string_value())
                            .map(|ast_string_val| ast_string_val.0.to_string());
                        (name, t, description)
                    })
                    .collect::<Vec<_>>();
                internal_baml_jinja::Class {
                    name: ast_class.name.name().to_string().into(),
                    fields,
                }
            })
            .collect::<Vec<_>>();
        OutputFormatContent::new(enums, classes, target.clone())
    }
}
