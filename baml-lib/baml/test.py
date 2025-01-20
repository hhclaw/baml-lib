import baml_lib
from pydantic import BaseModel, Field
from typing import Optional, Union, List, Type, get_origin, get_args
import types
from enum import Enum

class DocEnum(Enum):
  def __new__(cls, value, doc=None):
    self = cls.__new__(cls)
    self._value_ = value
    if doc is not None:
      self.__doc__ = doc
    else:
      self.__doc__ = None
    return self

prompt="""

enum FruitName {
  @@description("asf")
  Apple
  @description("apple enum")
  Banana
  Orange

}

class Fruit {
  fruit       FruitName
  price       int @description("Price ")
  size       Size
  description string?
  description1 string?
  dateSold    string
  received    bool

}

class Size {
  value  float
  unit   string
}
class FruitOrders {
  id    string 
  fruit Fruit[]
}


"""

# Define the Enum for FruitName
class FruitName(str, DocEnum):
    Apple = "Apple", "Represents Apple"
    Banana = "Banana"
    Orange = "Orange", "Represents orange"

print (FruitName.Apple.__doc__)
class Size(BaseModel):
    value: float
    unit: str

# Define the Fruit model
class Fruit(BaseModel):
    """
    class fruit!
    """
    fruit: FruitName
    price: int = Field(..., description="Price ", alias="pricing")
    size: Size
    description: Optional[str]
    description1: str | None
    dateSold: str
    received: bool

# Define the FruitOrders model
class FruitOrders(BaseModel):
    id: str
    fruit: List[Fruit]

def get_name(field_type):
    if field_type is str:
        return "string"
    return field_type.__name__

# Function to generate the output format
def generate_output(model: Type[BaseModel]) -> str:
    # Enum declaration for Enum fields
    enum_declarations = ""
    for name, field in model.__annotations__.items():
        print(field)
        if isinstance(field, type) and issubclass(field, Enum):
            enum_name = field.__name__
            enum_description = field.__doc__
            if enum_description and enum_description.strip():
                enum_desc_line = [f"@@description(\"{enum_description.strip()}\")"]
            else:
                enum_desc_line = []
            print(enum_description)
            enum_declarations += f"enum {enum_name} {{\n  " + "\n  ".join([item.value + (f" @description(\"{item.__doc__}\")" if item.__doc__ else "")  for item in field] + enum_desc_line) + "\n}\n"

    # Class declaration
    class_output = f"class {model.__name__} {{\n"
    for field_name, field_info in model.model_fields.items():
        field_type = field_info.annotation
        origin = get_origin(field_type)
        description = field_info.description or ""
        field_alias = field_info.alias or ""
        
        # Format the field type
        if origin is Union or origin is types.UnionType:
            print(get_args(field_type))

            type_annotation = get_name(get_args(field_type)[0]) + "?"
        elif origin is list:
            type_annotation = get_name(get_args(field_type)[0]) + "[]"
        elif origin is None:
            print(origin)
            type_annotation = get_name(field_type)
        else:
            print(f"{origin} not supported")
            continue
        class_output += f"  {field_alias or field_name} {type_annotation}"
        if description:
            class_output += f" @description(\"{description}\")"
        class_output += "\n"
    class_output += "}\n"

    return enum_declarations + class_output

if __name__ == "__main__":
   output = generate_output(Size) + generate_output(Fruit) + generate_output(FruitOrders)
   print(output)
   print(baml_lib.render_prompt(output, "FruitOrders", None, True))

   results = """
{
  "id": 1234,
  "fruit": [{
    "fruit": "apple",
    "pricing": 123,
    "dateSold": "123456",
    "received": false,
    "size": {
       "unit": pcs,
       "value": 12"""
   print(baml_lib.validate_result(output, results.strip(), "FruitOrders", True))
