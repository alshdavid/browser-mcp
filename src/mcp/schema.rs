use serde_json::Map;
use serde_json::Value;

use crate::mcp::tools::common::DESCRIPTIONS;
use crate::mcp::tools::common::ITEM_DESCRIPTIONS;
use crate::mcp::tools::common::MIN_ITEMS;
use crate::mcp::tools::common::NUMERIC_BOUNDS;

pub fn normalize(
  tool_name: &str,
  schema: &mut Map<String, Value>,
) {
  schema.remove("$schema");
  schema.remove("additionalProperties");

  let definitions = match schema.remove("$defs") {
    Some(Value::Object(definitions)) => definitions,
    _ => Map::new(),
  };

  let mut root = Value::Object(std::mem::take(schema));
  inline_refs(&mut root, &definitions, 0);
  if let Value::Object(root) = root {
    *schema = root;
  }

  let required = required_fields(schema);

  let Some(Value::Object(properties)) = schema.get_mut("properties") else {
    return;
  };

  for (name, property) in properties.iter_mut() {
    let Some(property) = property.as_object_mut() else {
      continue;
    };

    if !required.iter().any(|field| field == name) {
      unwrap_nullable(property);
    }

    property.remove("format");
    apply_description(tool_name, name, property);
    apply_item_description(tool_name, name, property);
    apply_bounds(tool_name, name, property);
  }
}

fn inline_refs(
  value: &mut Value,
  definitions: &Map<String, Value>,
  depth: usize,
) {
  if depth > 8 {
    return;
  }

  match value {
    Value::Object(object) => {
      if let Some(Value::String(reference)) = object.get("$ref")
        && let Some(name) = reference.strip_prefix("#/$defs/")
        && let Some(resolved) = definitions.get(name)
      {
        let resolved = resolved.clone();
        object.remove("$ref");
        if let Value::Object(resolved) = resolved {
          for (key, value) in resolved {
            object.entry(key).or_insert(value);
          }
        }
      }

      for nested in object.values_mut() {
        inline_refs(nested, definitions, depth + 1);
      }

      flatten_combinators(object);
    }
    Value::Array(items) => {
      for item in items.iter_mut() {
        inline_refs(item, definitions, depth + 1);
      }
    }
    _ => {}
  }
}

fn flatten_combinators(object: &mut Map<String, Value>) {
  for keyword in ["anyOf", "oneOf", "allOf"] {
    let Some(Value::Array(variants)) = object.get(keyword).cloned() else {
      continue;
    };

    let non_null: Vec<Value> = variants
      .into_iter()
      .filter(|variant| !variant.get("type").is_some_and(|value| value == "null"))
      .collect();

    if non_null.len() != 1 {
      continue;
    }

    object.remove(keyword);
    if let Value::Object(only) = non_null.into_iter().next().unwrap() {
      for (key, value) in only {
        object.entry(key).or_insert(value);
      }
    }
  }
}

fn required_fields(schema: &Map<String, Value>) -> Vec<String> {
  schema
    .get("required")
    .and_then(Value::as_array)
    .map(|values| {
      values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
    })
    .unwrap_or_default()
}

fn unwrap_nullable(property: &mut Map<String, Value>) {
  if let Some(Value::Array(types)) = property.get("type")
    && types.len() == 2
    && let Some(inner) = types.iter().find(|t| *t != "null")
  {
    let inner = inner.clone();
    property.insert("type".to_owned(), inner);
  }
}

fn apply_description(
  tool_name: &str,
  name: &str,
  property: &mut Map<String, Value>,
) {
  if property.contains_key("description") {
    return;
  }
  for (tool, field, description) in DESCRIPTIONS {
    if *tool == tool_name && *field == name {
      property.insert(
        "description".to_owned(),
        Value::String((*description).to_owned()),
      );
      return;
    }
  }
}

fn apply_item_description(
  tool_name: &str,
  name: &str,
  property: &mut Map<String, Value>,
) {
  for (tool, field, description) in ITEM_DESCRIPTIONS {
    if *tool != tool_name || *field != name {
      continue;
    }
    if let Some(Value::Object(items)) = property.get_mut("items") {
      items.insert(
        "description".to_owned(),
        Value::String((*description).to_owned()),
      );
    }
  }
}

fn apply_bounds(
  tool_name: &str,
  name: &str,
  property: &mut Map<String, Value>,
) {
  for (tool, field, minimum, maximum, exclusive_minimum) in NUMERIC_BOUNDS {
    if *tool == tool_name && *field == name {
      if let Some(minimum) = minimum {
        property.insert("minimum".to_owned(), Value::from(*minimum));
      }
      if let Some(maximum) = maximum {
        property.insert("maximum".to_owned(), Value::from(*maximum));
      }
      if *exclusive_minimum {
        property.insert("exclusiveMinimum".to_owned(), Value::from(0));
      }
    }
  }

  for (tool, field, min_items) in MIN_ITEMS {
    if *tool == tool_name && *field == name {
      property.insert("minItems".to_owned(), Value::from(*min_items));
    }
  }

  apply_default(tool_name, name, property);
}

fn apply_default(
  tool_name: &str,
  name: &str,
  property: &mut Map<String, Value>,
) {
  let default = match (tool_name, name) {
    ("get_css_styles", "pageSize") => Value::from(10),
    ("get_css_styles", "pageIdx") => Value::from(0),
    ("lighthouse_audit", "mode") => Value::from("navigation"),
    ("lighthouse_audit", "device") => Value::from("desktop"),
    ("list_console_messages", "includePreservedMessages") => Value::Bool(false),
    ("list_console_messages", "includeStackTraces") => Value::Bool(false),
    ("list_network_requests", "includePreservedRequests") => Value::Bool(false),
    ("performance_start_trace", "reload") => Value::Bool(true),
    ("performance_start_trace", "autoStop") => Value::Bool(true),
    ("take_screenshot", "format") => Value::from("png"),
    _ => return,
  };
  property.insert("default".to_owned(), default);
}
