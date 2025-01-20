use baml_types::LiteralValue;

use super::*;

test_deserializer!(
    test_simple_recursive_alias_list,
    r#"
type A = A[]
    "#,
    "[[], [], [[]]]",
    FieldType::RecursiveTypeAlias("A".into()),
    [[], [], [[]]]
);

test_deserializer!(
    test_simple_recursive_alias_map,
    r#"
type A = map<string, A>
    "#,
    r#"{"one": {"two": {}}, "three": {"four": {}}}"#,
    FieldType::RecursiveTypeAlias("A".into()),
    {
        "one": {"two": {}},
        "three": {"four": {}}
    }
);

test_deserializer!(
    test_recursive_alias_cycle,
    r#"
type A = B
type B = C
type C = A[]
    "#,
    "[[], [], [[]]]",
    FieldType::RecursiveTypeAlias("A".into()),
    [[], [], [[]]]
);

test_deserializer!(
    test_json_without_nested_objects,
    r#"
type JsonValue = int | float | bool | string | null | JsonValue[] | map<string, JsonValue> 
    "#,
    r#"
    {
        "int": 1,
        "float": 1.0,
        "string": "test",
        "bool": true
    }
    "#,
    FieldType::RecursiveTypeAlias("JsonValue".into()),
    {
        "int": 1,
        "float": 1.0,
        "string": "test",
        "bool": true
    }
);

test_deserializer!(
    test_json_with_nested_list,
    r#"
type JsonValue = int | float | bool | string | null | JsonValue[] | map<string, JsonValue> 
    "#,
    r#"
    {
        "number": 1,
        "string": "test",
        "bool": true,
        "list": [1, 2, 3]
    }
    "#,
    FieldType::RecursiveTypeAlias("JsonValue".into()),
    {
        "number": 1,
        "string": "test",
        "bool": true,
        "list": [1, 2, 3]
    }
);

test_deserializer!(
    test_json_with_nested_object,
    r#"
type JsonValue = int | float | bool | string | null | JsonValue[] | map<string, JsonValue> 
    "#,
    r#"
    {
        "number": 1,
        "string": "test",
        "bool": true,
        "json": {
            "number": 1,
            "string": "test",
            "bool": true
        }
    }
    "#,
    FieldType::RecursiveTypeAlias("JsonValue".into()),
    {
        "number": 1,
        "string": "test",
        "bool": true,
        "json": {
            "number": 1,
            "string": "test",
            "bool": true
        }
    }
);

test_deserializer!(
    test_full_json_with_nested_objects,
    r#"
type JsonValue = int | float | bool | string | null | JsonValue[] | map<string, JsonValue> 
    "#,
    r#"
    {
        "number": 1,
        "string": "test",
        "bool": true,
        "list": [1, 2, 3],
        "object": {
            "number": 1,
            "string": "test",
            "bool": true,
            "list": [1, 2, 3]
        },
        "json": {
            "number": 1,
            "string": "test",
            "bool": true,
            "list": [1, 2, 3],
            "object": {
                "number": 1,
                "string": "test",
                "bool": true,
                "list": [1, 2, 3]
            }
        }
    }
    "#,
    FieldType::RecursiveTypeAlias("JsonValue".into()),
    {
        "number": 1,
        "string": "test",
        "bool": true,
        "list": [1, 2, 3],
        "object": {
            "number": 1,
            "string": "test",
            "bool": true,
            "list": [1, 2, 3]
        },
        "json": {
            "number": 1,
            "string": "test",
            "bool": true,
            "list": [1, 2, 3],
            "object": {
                "number": 1,
                "string": "test",
                "bool": true,
                "list": [1, 2, 3]
            }
        }
    }
);

test_deserializer!(
    test_list_of_json_objects,
    r#"
type JsonValue = int | float | bool | string | null | JsonValue[] | map<string, JsonValue> 
    "#,
    r#"
    [
        {
            "number": 1,
            "string": "test",
            "bool": true,
            "list": [1, 2, 3]
        },
        {
            "number": 1,
            "string": "test",
            "bool": true,
            "list": [1, 2, 3]
        }
    ]
    "#,
    FieldType::RecursiveTypeAlias("JsonValue".into()),
    [
        {
            "number": 1,
            "string": "test",
            "bool": true,
            "list": [1, 2, 3]
        },
        {
            "number": 1,
            "string": "test",
            "bool": true,
            "list": [1, 2, 3]
        }
    ]
);

test_deserializer!(
    test_nested_list,
    r#"
type JsonValue = int | float | bool | string | null | JsonValue[] | map<string, JsonValue> 
    "#,
    r#"
    [[42.1]]
    "#,
    FieldType::RecursiveTypeAlias("JsonValue".into()),
    // [[[[[[[[[[[[[[[[[[[[42]]]]]]]]]]]]]]]]]]]]
    [[42.1]]
);

test_deserializer!(
    test_json_defined_with_cycles,
    r#"
type JsonValue = int | float | bool | string | null | JsonArray | JsonObject
type JsonArray = JsonValue[]
type JsonObject = map<string, JsonValue>
    "#,
    r#"
    {
        "number": 1,
        "string": "test",
        "bool": true,
        "json": {
            "number": 1,
            "string": "test",
            "bool": true
        }
    }
    "#,
    FieldType::RecursiveTypeAlias("JsonValue".into()),
    {
        "number": 1,
        "string": "test",
        "bool": true,
        "json": {
            "number": 1,
            "string": "test",
            "bool": true
        }
    }
);

test_deserializer!(
    test_ambiguous_int_string_json_type,
    r#"
type JsonValue = int | float | bool | string | null | JsonValue[] | map<string, JsonValue> 
    "#,
    r#"
    {
        "recipe": {
            "name": "Chocolate Chip Cookies",
            "servings": 24,
            "ingredients": [
                "2 1/4 cups all-purpose flour", "1/2 teaspoon baking soda",
                "1 cup unsalted butter, room temperature",
                "1/2 cup granulated sugar",
                "1 cup packed light-brown sugar",
                "1 teaspoon salt", "2 teaspoons pure vanilla extract",
                "2 large eggs", "2 cups semisweet and/or milk chocolate chips"
            ],
            "instructions": [
                "Preheat oven to 350°F (180°C).",
                "In a small bowl, whisk together flour and baking soda; set aside.",
                "In a large bowl, cream butter and sugars until light and fluffy.",
                "Add salt, vanilla, and eggs; mix well.",
                "Gradually stir in flour mixture.",
                "Fold in chocolate chips.",
                "Drop by rounded tablespoons onto ungreased baking sheets.",
                "Bake for 10-12 minutes or until golden brown.",
                "Cool on wire racks."
            ]
        }
    }
    "#,
    FieldType::RecursiveTypeAlias("JsonValue".into()),
    {
        "recipe": {
            "name": "Chocolate Chip Cookies",
            "servings": 24,
            "ingredients": [
                "2 1/4 cups all-purpose flour", "1/2 teaspoon baking soda",
                "1 cup unsalted butter, room temperature",
                "1/2 cup granulated sugar",
                "1 cup packed light-brown sugar",
                "1 teaspoon salt", "2 teaspoons pure vanilla extract",
                "2 large eggs", "2 cups semisweet and/or milk chocolate chips"
            ],
            "instructions": [
                "Preheat oven to 350°F (180°C).",
                "In a small bowl, whisk together flour and baking soda; set aside.",
                "In a large bowl, cream butter and sugars until light and fluffy.",
                "Add salt, vanilla, and eggs; mix well.",
                "Gradually stir in flour mixture.",
                "Fold in chocolate chips.",
                "Drop by rounded tablespoons onto ungreased baking sheets.",
                "Bake for 10-12 minutes or until golden brown.",
                "Cool on wire racks."
            ]
        }
    }
);
