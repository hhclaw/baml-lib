# BAML library

Github repository: https://github.com/hhclaw/baml-lib.

## Overview

Uses BoundaryML's [BAML](https://github.com/BoundaryML/baml) to generate json schema for LLM prompting,
and parse json from llm output under Python, without a seperate code 
generation step.

This library exposes a `PyBamlContext` class.  The class can load and validate
a BAML schema and store it within its context. It can then be used
to generate the JSON part of the prompt, and parse LLM output.

This allows existing python workflow to enjoy the model performance
improvement achieved with BAML (concise prompting and fault tolerance
"schema aligned" JSON parsing) while keeping the prompt templating
and workflow processing unchanged.

Credit: based on the fork from https://github.com/lmnr-ai/lmnr-baml, updated
with upstream BAML code and exposing some options.

## Technical details

This is a hard fork of BoundaryML's [BAML](https://github.com/BoundaryML/baml).

It contains a very stripped down version the upstream BAML repository that does only:
- Schema validation
- JSON part of LLM prompting (for structured output)
- LLM output parsing (validation against schema)

The only required types from BAML AST for this are:
- Enum
- Class

and their dependencies.

The BAML engine is copied from upstream, mostly untouched except for
exposing a few structs / functions for external call.

## Interface
```python
from typing import Optional

class PyBamlContext:

    def __init__(self, baml_schema: str, target_name: Optional[str]):
        """
        Creates the PyBamlContext.
        :param baml_schema: BAML schema (Class and Enum definitions)
        :param target_name: Target Class or Enum to render
        """
        ...

    def render_prompt(self, prefix: Optional[str], always_hoist_enums: Optional[bool]):
        """
        Renders the prompt with the context
        :param prefix: If specified, use as prefix to the target schema instead of the default
        :always_hoist_enums: Always renders Enum separately, instead of inline type
        """
        ...

    def validate_result(self, results: str, allow_partials: Optional[bool]):
        """
        Try to parse the results
        :param results: Results string (e.g. from LLM output)
        :param allow_partials: Allow partial fulfillment of schema (i.e. not filling
        all required fields)
        """
        ...

```
## Example usage
```python
import baml_lib

baml_schema = """
enum FruitName {
  Apple
  Banana
  Orange
  Others @description("Default")
}

class Fruit {
  fruit       FruitName
  price       int @description("Price per unit") @alias("fruit_price")
  dateSold    string
  received    bool
}

class FruitOrders {
  id    string
  fruit Fruit[]
}
"""

# Create baml context
baml_context = baml_lib.PyBamlContext(baml_schema, "FruitOrders")

# Renders prompt
print(baml_context.render_prompt(None, True))

# Parse result (note the incomplete JSON)
results = """
{
  "id": 1234,
  "fruit": [{
    "fruit": "apple",
    "dateSold": "123456",
    "received": false,
    "fruit_price": 123
""".strip()

print(baml_context.validate_result(results, True))
```
`render_prompt(None, True)` above outputs:
```
FruitName
----
- Apple
- Banana
- Orange
- Others: Default

Answer in JSON using this schema:
{
  id: string,
  fruit: [
    {
      fruit: FruitName,
      // Price per unit
      fruit_price: int,
      dateSold: string,
      received: bool,
    }
  ],
}

```
`validate_result(results, True)` above outputs (after formatting):)
```
{
  "id": "1234",
  "fruit": [
    {
      "fruit": "Apple",
      "fruit_price": null,
      "dateSold": "123456",
      "received": false
    }
  ]
}
```
Note that `fruit_price` is not read: with `allow_partials`, trailing number are not parsed, since the number may not be completed.

