# BAML library

Github: [baml-lib](https://github.com/hhclaw/baml-lib).

Uses BoundaryML's BAML to generate json schema for LLM prompting,
and parse json from llm output under Python, without a seperate code 
generation step.

This is a hard fork of BoundaryML's [BAML](https://github.com/BoundaryML/baml).

It contains a very stripped down version that does only:
- Schema validation
- JSON part of LLM prompting (for structured output)
- LLM output parsing (validation against schema)

The only required types from BAML AST for this are:
- Enum
- Class

and their dependencies.

To interface those functions with Python, this library exposes
interfaces through Pyo3 PyBamlContext.  The context class does schema 
validation upon instance creation. The validated
schema is stored in the PyBamlContext for later use
to generate the JSON part of the prompt and parse LLM output.

This allows existing python workflow to enjoy the model performance
improvement achieved with BAML (concise prompting and fault tolerance
JSON parsing) while keeping the prompt templating and workflow 
processing.
