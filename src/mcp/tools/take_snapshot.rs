use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::accessibility::AxNode;
use chromiumoxide::cdp::browser_protocol::accessibility::AxValue;
use chromiumoxide::cdp::browser_protocol::accessibility::GetFullAxTreeParams;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::PageSnapshot;
use crate::mcp::tool_context::SnapshotNode;
use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TakeSnapshotRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "Whether to include all possible information available in the full a11y tree. Default is false."
  )]
  pub verbose: Option<bool>,
  #[schemars(
    description = "The absolute path, or a path relative to the current working directory, to save the snapshot to instead of attaching it to the response."
  )]
  pub file_path: Option<String>,
}

const EXCLUDED_ATTRIBUTES: &[&str] = &[
  "id",
  "role",
  "name",
  "elementHandle",
  "children",
  "backendNodeId",
  "loaderId",
];

const BOOLEAN_PROPERTIES: &[(&str, &str)] = &[
  ("disabled", "disableable"),
  ("expanded", "expandable"),
  ("focused", "focusable"),
  ("selected", "selectable"),
];

#[derive(Debug, Clone)]
pub struct FlatNode {
  uid: String,
  role: String,
  name: Option<String>,
  value: Option<String>,
  description: Option<String>,
  attributes: Vec<(String, serde_json::Value)>,
  ignored: bool,
  children: Vec<usize>,
}

pub async fn take_snapshot(
  tool_context: ToolContext,
  Parameters(TakeSnapshotRequest {
    page_id,
    verbose,
    file_path,
  }): Parameters<TakeSnapshotRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  let verbose = verbose.unwrap_or(false);
  let nodes = accessibility_tree(&page).await.map_err(internal)?;

  let (snapshot_id, previous_uids) = tool_context.begin_snapshot(page_id).await;
  let (root, snapshot) = assign_uids(&nodes, snapshot_id, verbose, previous_uids);
  let text = format_tree(root.as_ref(), verbose);
  tool_context.store_snapshot(page_id, snapshot).await;

  let text = match file_path {
    Some(path) => {
      let path = ensure_txt_extension(&path);
      save(&path, &text).await?;
      format!("Saved snapshot to {}.", path.to_string_lossy())
    }
    None => text,
  };

  Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

pub(crate) async fn accessibility_tree(
  page: &Page
) -> Result<Vec<AxNode>, chromiumoxide::error::CdpError> {
  let mut nodes = page
    .execute(GetFullAxTreeParams::default())
    .await?
    .result
    .nodes;

  let frames = page.frames().await.unwrap_or_default();
  let main_frame = page.mainframe().await.ok().flatten();

  for frame in frames {
    if Some(frame.clone()) == main_frame {
      continue;
    }

    let params = GetFullAxTreeParams {
      depth: None,
      frame_id: Some(frame),
    };
    if let Ok(response) = page.execute(params).await {
      nodes.extend(response.result.nodes);
    }
  }

  Ok(nodes)
}

pub(crate) fn assign_uids(
  nodes: &[AxNode],
  snapshot_id: u64,
  verbose: bool,
  mut previous_uids: HashMap<i64, String>,
) -> (Option<Vec<FlatNode>>, PageSnapshot) {
  let by_id: HashMap<&str, &AxNode> = nodes
    .iter()
    .map(|node| (node.node_id.inner().as_str(), node))
    .collect();

  let roots: Vec<&AxNode> = nodes
    .iter()
    .filter(|node| node.parent_id.is_none())
    .collect();

  let mut flat: Vec<FlatNode> = Vec::new();
  let mut snapshot = PageSnapshot::default();
  let mut counter = 0u64;

  let Some(primary) = roots.first() else {
    return (None, snapshot);
  };

  let mut root_index: Option<usize> = None;
  let mut stack = vec![(*primary, 0usize, None::<usize>)];
  let mut root_queue: Vec<&AxNode> = roots.iter().skip(1).copied().collect();

  loop {
    while let Some((node, depth, parent)) = stack.pop() {
      let backend_node_id = node.backend_dom_node_id.map(|id| *id.inner());

      let uid = match backend_node_id.and_then(|id| previous_uids.remove(&id)) {
        Some(uid) => uid,
        None => {
          let uid = format!("{snapshot_id}_{counter}");
          counter += 1;
          uid
        }
      };

      if let Some(backend_node_id) = backend_node_id {
        snapshot
          .uids_by_backend_node
          .insert(backend_node_id, uid.clone());
      }

      snapshot.nodes_by_uid.insert(
        uid.clone(),
        SnapshotNode {
          backend_node_id,
          role: role_of(node),
        },
      );

      let children = node
        .child_ids
        .as_ref()
        .map(|ids| {
          ids
            .iter()
            .filter_map(|id| by_id.get(id.inner().as_str()).copied())
            .collect::<Vec<_>>()
        })
        .unwrap_or_default();

      let index = flat.len();
      flat.push(FlatNode {
        uid,
        role: role_of(node),
        name: name_of(node),
        value: value_of(node),
        description: description_of(node),
        attributes: attribute_list(node),
        ignored: node.ignored,
        children: Vec::new(),
      });

      match parent {
        Some(parent) => flat[parent].children.push(index),
        None => match root_index {
          Some(root) if root != index => flat[root].children.push(index),
          _ => root_index = Some(index),
        },
      }

      for child in children.into_iter().rev() {
        stack.push((child, depth + 1, Some(index)));
      }
    }

    match root_queue.pop() {
      Some(root) => stack.push((root, 1, root_index)),
      None => break,
    }
  }

  let mut compact = Vec::new();
  if let Some(root_index) = root_index {
    flatten(&flat, root_index, verbose, &mut compact);
  }

  (Some(compact), snapshot)
}

fn flatten(
  flat: &[FlatNode],
  index: usize,
  verbose: bool,
  output: &mut Vec<FlatNode>,
) -> Vec<usize> {
  let hidden = !verbose && flat[index].ignored && index != 0;

  let position = if hidden {
    None
  } else {
    let mut node = flat[index].clone();
    node.children = Vec::new();
    output.push(node);
    Some(output.len() - 1)
  };

  let mut promoted = Vec::new();
  for child in flat[index].children.clone() {
    let child_nodes = flatten(flat, child, verbose, output);

    match position {
      Some(parent) => {
        for child_position in &child_nodes {
          output[parent].children.push(*child_position);
        }
      }
      None => promoted.extend(child_nodes),
    }
  }

  match position {
    Some(position) => vec![position],
    None => promoted,
  }
}

fn attribute_list(node: &AxNode) -> Vec<(String, serde_json::Value)> {
  let mut attributes = Vec::new();

  if let Some(properties) = node.properties.as_ref() {
    for property in properties {
      let name = property.name.as_ref().to_owned();
      if EXCLUDED_ATTRIBUTES.contains(&name.as_str()) {
        continue;
      }
      if let Some(mapped) = boolean_property(&name) {
        attributes.push((mapped.to_owned(), serde_json::Value::Bool(true)));
      }
      if let Some(value) = json_of(&property.value) {
        attributes.push((name, value));
      }
    }
  }

  attributes.sort_by(|left, right| left.0.cmp(&right.0));
  attributes
}

fn boolean_property(name: &str) -> Option<&'static str> {
  BOOLEAN_PROPERTIES
    .iter()
    .find(|(source, _)| *source == name)
    .map(|(_, mapped)| *mapped)
}

fn json_of(value: &AxValue) -> Option<serde_json::Value> {
  match value.value.as_ref()? {
    serde_json::Value::Bool(false) | serde_json::Value::Null => None,
    serde_json::Value::Array(_) | serde_json::Value::Object(_) => None,
    other => Some(other.clone()),
  }
}

fn role_of(node: &AxNode) -> String {
  text_of(node.role.as_ref()).unwrap_or_default()
}

fn name_of(node: &AxNode) -> Option<String> {
  text_of(node.name.as_ref())
}

fn description_of(node: &AxNode) -> Option<String> {
  text_of(node.description.as_ref())
}

fn value_of(node: &AxNode) -> Option<String> {
  let value = node.value.as_ref()?.value.as_ref()?;
  match value {
    serde_json::Value::String(text) => Some(text.clone()),
    other => Some(other.to_string()),
  }
}

fn text_of(value: Option<&AxValue>) -> Option<String> {
  match value?.value.as_ref()? {
    serde_json::Value::String(text) => Some(text.clone()),
    _ => None,
  }
}

pub(crate) fn format_tree(
  root: Option<&Vec<FlatNode>>,
  verbose: bool,
) -> String {
  let Some(flat) = root else {
    return String::new();
  };

  let mut output = String::new();
  if !flat.is_empty() {
    format_node(flat, 0, 0, verbose, &mut output);
  }
  output
}

fn format_node(
  flat: &[FlatNode],
  index: usize,
  depth: usize,
  verbose: bool,
  output: &mut String,
) {
  let node = &flat[index];
  output.push_str(&"  ".repeat(depth));
  output.push_str(&format_attributes(node, verbose));
  output.push('\n');

  if has_redundant_text_children(flat, node, verbose) {
    return;
  }

  for child in &node.children {
    format_node(flat, *child, depth + 1, verbose, output);
  }
}

fn has_redundant_text_children(
  flat: &[FlatNode],
  node: &FlatNode,
  verbose: bool,
) -> bool {
  let Some(name) = node.name.as_ref() else {
    return false;
  };
  if verbose || node.children.is_empty() {
    return false;
  }

  let mut text = String::new();
  for child in &node.children {
    let child = &flat[*child];
    if !is_text_role(&child.role) || !child.children.is_empty() {
      return false;
    }
    text.push_str(child.name.as_deref().unwrap_or_default());
  }

  strip_whitespace(&text) == strip_whitespace(name)
}

fn is_text_role(role: &str) -> bool {
  role == "StaticText" || role == "InlineTextBox"
}

fn strip_whitespace(text: &str) -> String {
  text.chars().filter(|c| !c.is_whitespace()).collect()
}

fn format_attributes(
  node: &FlatNode,
  verbose: bool,
) -> String {
  let mut attributes = vec![format!("uid={}", node.uid)];

  if !node.role.is_empty() && node.role != "StaticText" {
    if node.role == "none" {
      attributes.push("ignored".to_owned());
    } else {
      attributes.push(node.role.clone());
    }
  }

  if let Some(name) = node.name.as_ref() {
    attributes.push(format!("\"{name}\""));
  }

  let is_option = node.role == "option";
  for (name, value) in &node.attributes {
    if is_option && name == "selectable" {
      continue;
    }
    match value {
      serde_json::Value::Bool(true) => attributes.push(name.clone()),
      serde_json::Value::String(text) => {
        attributes.push(format!("{name}=\"{text}\""));
      }
      serde_json::Value::Number(number) => {
        attributes.push(format!("{name}={number}"));
      }
      _ => {}
    }
  }

  if verbose {
    if let Some(value) = node.value.as_ref()
      && !(is_option && node.name.as_deref() == Some(value.as_str()))
    {
      attributes.push(format!("value=\"{value}\""));
    }
    if let Some(description) = node.description.as_ref() {
      attributes.push(format!("description=\"{description}\""));
    }
  }

  attributes.join(" ")
}

fn ensure_txt_extension(path: &str) -> PathBuf {
  Path::new(path).with_extension("txt")
}

async fn save(
  path: &Path,
  text: &str,
) -> Result<(), ErrorData> {
  if let Some(parent) = path.parent()
    && !parent.as_os_str().is_empty()
  {
    tokio::fs::create_dir_all(parent)
      .await
      .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
  }

  tokio::fs::write(path, text)
    .await
    .map_err(|error| ErrorData::internal_error(error.to_string(), None))
}

fn internal(error: chromiumoxide::error::CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
