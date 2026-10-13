use std::collections::HashMap;

use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::css::CssAtRule;
use chromiumoxide::cdp::browser_protocol::css::CssKeyframesRule;
use chromiumoxide::cdp::browser_protocol::css::CssRule;
use chromiumoxide::cdp::browser_protocol::css::CssRuleType;
use chromiumoxide::cdp::browser_protocol::css::CssStyle;
use chromiumoxide::cdp::browser_protocol::css::EnableParams;
use chromiumoxide::cdp::browser_protocol::css::GetMatchedStylesForNodeParams;
use chromiumoxide::cdp::browser_protocol::css::GetMatchedStylesForNodeReturns;
use chromiumoxide::cdp::browser_protocol::css::GetStyleSheetTextParams;
use chromiumoxide::cdp::browser_protocol::css::PseudoElementMatches;
use chromiumoxide::cdp::browser_protocol::css::RuleMatch;
use chromiumoxide::cdp::browser_protocol::css::SourceRange;
use chromiumoxide::cdp::browser_protocol::css::StyleSheetOrigin;
use chromiumoxide::cdp::browser_protocol::dom::BackendNodeId;
use chromiumoxide::cdp::browser_protocol::dom::DescribeNodeParams;
use chromiumoxide::cdp::browser_protocol::dom::GetContainerForNodeParams;
use chromiumoxide::cdp::browser_protocol::dom::Node;
use chromiumoxide::cdp::browser_protocol::dom::NodeId;
use chromiumoxide::cdp::browser_protocol::dom::StyleSheetId;
use rmcp::ErrorData;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::model::ContentBlock;
use rmcp::schemars;

use crate::mcp::tool_context::ToolContext;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetCssStylesRequest {
  #[schemars(description = "Targets a specific page by ID.")]
  pub page_id: f64,
  #[schemars(
    description = "The uid of the element on the page from the page content snapshot to inspect CSS styles for"
  )]
  pub uid: String,
  #[schemars(
    default = "page_size_default",
    description = "Maximum number of CSS rules to return per page. Defaults to 10."
  )]
  pub page_size: Option<i64>,
  #[schemars(
    default = "page_idx_default",
    description = "Page number to return (0-based). Defaults to 0 (the first page)."
  )]
  pub page_idx: Option<i64>,
}

fn page_size_default() -> Option<i64> {
  Some(10)
}

fn page_idx_default() -> Option<i64> {
  Some(0)
}

const DEFAULT_PAGE_SIZE: i64 = 10;
const ANCESTOR_WALK_LIMIT: usize = 64;

pub async fn get_css_styles(
  tool_context: ToolContext,
  Parameters(GetCssStylesRequest {
    page_id,
    uid,
    page_size,
    page_idx,
  }): Parameters<GetCssStylesRequest>,
) -> Result<CallToolResult, ErrorData> {
  let page = tool_context
    .page(page_id)
    .await
    .ok_or_else(|| ErrorData::invalid_params(format!("No page found with id {page_id}"), None))?;

  if !tool_context.has_snapshot(page_id).await {
    return Err(ErrorData::invalid_params(
      format!("No snapshot found for page {page_id}. Use take_snapshot to capture one."),
      None,
    ));
  }

  let backend_node_id = tool_context
    .backend_node_for_uid(page_id, &uid)
    .await
    .ok_or_else(|| {
      ErrorData::invalid_params(
        format!("Element uid \"{uid}\" not found on page {page_id}."),
        None,
      )
    })?;

  page
    .execute(EnableParams::default())
    .await
    .map_err(internal)?;

  let node = page
    .execute(
      DescribeNodeParams::builder()
        .backend_node_id(BackendNodeId::new(backend_node_id))
        .build(),
    )
    .await
    .map_err(internal)?
    .result
    .node;

  let selector = simple_selector(&node);

  let matched = page
    .execute(
      GetMatchedStylesForNodeParams::builder()
        .node_id(node.node_id)
        .build()
        .map_err(|message| ErrorData::internal_error(message, None))?,
    )
    .await
    .map_err(internal)?
    .result;

  let elements = Elements {
    ancestors: ancestor_selectors(&page, node.node_id).await,
    sheets: sheet_texts(&page, &matched).await,
    uids: tool_context.uids_by_backend_node(page_id).await,
  };

  let mut rules = collect_rules(&matched, &elements);
  resolve_container_elements(&page, node.node_id, &elements, &mut rules).await;

  let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).max(1) as usize;
  let total_pages = rules.len().div_ceil(page_size).max(1);
  let requested = page_idx.unwrap_or(0);
  let invalid_page = requested < 0 || requested as usize >= total_pages;
  let current_page = if invalid_page { 0 } else { requested as usize };

  let start = current_page * page_size;
  let end = (start + page_size).min(rules.len());
  let visible = &rules[start.min(rules.len())..end];

  let mut lines = vec![format!("Styles for {selector} (uid: \"{uid}\"):")];

  if invalid_page {
    lines.push("Invalid page number provided. Showing first page.".to_owned());
  }

  lines.push(format!(
    "Showing {}-{} of {} (Page {} of {}).",
    if rules.is_empty() { 0 } else { start + 1 },
    end,
    rules.len(),
    current_page + 1,
    total_pages
  ));

  if current_page < total_pages - 1 {
    lines.push(format!("Next page: {}", current_page + 1));
  }

  if current_page > 0 {
    lines.push(format!("Previous page: {}", current_page - 1));
  }

  if rules.is_empty() {
    lines.push(String::new());
    lines.push("  (no styles)".to_owned());
    return Ok(CallToolResult::success(vec![ContentBlock::text(
      lines.join("\n"),
    )]));
  }

  let mut writer = Writer::new(2);
  for rule in visible {
    writer.blank();
    write_rule(&mut writer, rule);
  }

  lines.extend(writer.lines);

  Ok(CallToolResult::success(vec![ContentBlock::text(
    lines.join("\n"),
  )]))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropStatus {
  Active,
  Overloaded,
  Invalid,
  Disabled,
}

impl PropStatus {
  fn tag(self) -> &'static str {
    match self {
      PropStatus::Active => "",
      PropStatus::Overloaded => "[overloaded] ",
      PropStatus::Invalid => "[invalid] ",
      PropStatus::Disabled => "[disabled] ",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Property {
  name: String,
  value: String,
  status: PropStatus,
  important: bool,
}

// CDP exposes no winning/losing flag for a declaration, but each matched
// selector carries its specificity, so the cascade is ranked here: important
// first, then the (a, b, c) specificity, then author order. Inline styles are
// excluded because Chrome always places them ahead of matched rules, and
// inherited declarations are ranked separately by their own origin.
type CascadeRank = (bool, (i64, i64, i64));

fn mark_overloaded(rules: &mut [Rule]) {
  let mut winners: HashMap<String, (usize, CascadeRank)> = HashMap::new();

  for (index, rule) in rules.iter().enumerate() {
    let Rule::Matched {
      properties,
      specificity,
      ..
    } = rule
    else {
      continue;
    };
    let Some(specificity) = specificity else {
      continue;
    };

    for property in properties {
      if property.status != PropStatus::Active {
        continue;
      }

      let candidate = (property.important, *specificity);
      let key = property.name.clone();

      match winners.get(&key) {
        Some((_, winner)) if *winner >= candidate => {}
        _ => {
          winners.insert(key, (index, candidate));
        }
      }
    }
  }

  for (index, rule) in rules.iter_mut().enumerate() {
    let Rule::Matched { properties, .. } = rule else {
      continue;
    };

    for property in properties.iter_mut() {
      if property.status != PropStatus::Active {
        continue;
      }

      if winners
        .get(&property.name)
        .is_some_and(|(winner, _)| *winner != index)
      {
        property.status = PropStatus::Overloaded;
      }
    }
  }
}

fn specificity_of(rule_match: &RuleMatch) -> Option<(i64, i64, i64)> {
  let index = *rule_match.matching_selectors.first()? as usize;
  let specificity = rule_match
    .rule
    .selector_list
    .selectors
    .get(index)?
    .specificity
    .as_ref()?;

  Some((specificity.a, specificity.b, specificity.c))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Ancestor {
  Layer(Option<String>),
  Media(String),
  Container {
    query: String,
    name: Option<String>,
    element: Option<String>,
  },
  Scope(Option<String>),
  Supports(String),
  StartingStyle,
  Nesting(String),
  Unsupported {
    at_rule_type: String,
    name: Option<String>,
  },
}

impl Ancestor {
  fn comment(&self) -> Option<String> {
    match self {
      Ancestor::Container {
        element: Some(element),
        ..
      } => Some(format!("container: {element}")),
      _ => None,
    }
  }

  fn header(&self) -> String {
    match self {
      Ancestor::Layer(Some(name)) => format!("@layer {name}"),
      Ancestor::Layer(None) => "@layer".to_owned(),
      Ancestor::Media(query) => format!("@media {query}"),
      Ancestor::Container { query, name, .. } => {
        // Blink already folds the container name into the query text, so
        // repeating it would render `@container cq (cq (min-width: 300px))`.
        let prepend = match name {
          Some(name) if !query.starts_with(name.as_str()) => format!("{name} "),
          _ => String::new(),
        };
        format!("@container {prepend}{query}")
      }
      Ancestor::Scope(Some(query)) => format!("@scope ({query})"),
      Ancestor::Scope(None) => "@scope".to_owned(),
      Ancestor::Supports(query) => format!("@supports {query}"),
      Ancestor::StartingStyle => "@starting-style".to_owned(),
      Ancestor::Nesting(selector) => selector.clone(),
      Ancestor::Unsupported { at_rule_type, name } => match name {
        Some(name) => format!("@{at_rule_type} {name}"),
        None => format!("@{at_rule_type}"),
      },
    }
  }
}

#[derive(Debug, Clone, Default)]
struct Header {
  selector: String,
  source: Option<String>,
  ancestors: Vec<Ancestor>,
}

#[derive(Debug, Clone)]
enum Rule {
  Matched {
    header: Header,
    properties: Vec<Property>,
    specificity: Option<(i64, i64, i64)>,
  },
  Inherited {
    node: String,
    header: Header,
    properties: Vec<Property>,
  },
  Pseudo {
    pseudo_type: String,
    node: Option<String>,
    header: Header,
    properties: Vec<Property>,
  },
  Keyframes {
    name: String,
    source: Option<String>,
    steps: Vec<(String, Vec<Property>)>,
  },
}

#[derive(Debug, Clone, Default)]
struct Elements {
  // Ancestors from the immediate parent outwards, matching the order Chrome
  // reports inherited style entries in.
  ancestors: Vec<String>,
  sheets: HashMap<String, Sheet>,
  uids: HashMap<i64, String>,
}

#[derive(Debug, Clone)]
struct Sheet {
  text: String,
  source_url: String,
  start_line: i64,
  start_column: i64,
}

impl Sheet {
  fn source_name(&self) -> String {
    if self.source_url.is_empty() {
      return "<style>".to_owned();
    }
    filename_from_url(&self.source_url)
  }

  // An inline `<style>` sheet is embedded in the document, so its own position
  // inside the resource has to be added to the rule's range; an external sheet
  // is its own resource and its ranges are already absolute.
  fn line(
    &self,
    range: Option<&SourceRange>,
  ) -> Option<i64> {
    let range = range?;

    if self.source_url.is_empty() {
      return Some(self.start_line + range.start_line);
    }

    let offset = count_lines(&self.text, self.start_column as usize);
    Some(self.start_line + offset + range.start_line)
  }
}

fn count_lines(
  text: &str,
  end: usize,
) -> i64 {
  text
    .chars()
    .take(end)
    .filter(|character| *character == '\n')
    .count() as i64
}

fn filename_from_url(url: &str) -> String {
  if url.starts_with("data:") {
    return "data-uri".to_owned();
  }
  if url.starts_with("blob:") {
    return "blob".to_owned();
  }

  let path = url
    .split(['?', '#'])
    .next()
    .unwrap_or(url)
    .trim_end_matches('/');
  let last = path.rsplit('/').next().unwrap_or_default();

  if last.is_empty() {
    "index".to_owned()
  } else {
    last.to_owned()
  }
}

struct Writer {
  lines: Vec<String>,
  indent: usize,
}

impl Writer {
  fn new(indent: usize) -> Self {
    Self {
      lines: Vec::new(),
      indent,
    }
  }

  fn blank(&mut self) {
    self.lines.push(String::new());
  }

  fn line(
    &mut self,
    text: &str,
  ) {
    self
      .lines
      .push(format!("{}{text}", " ".repeat(self.indent)));
  }

  fn comment(
    &mut self,
    text: &str,
  ) {
    self.line(&format!("/* {text} */"));
  }

  fn block(
    &mut self,
    header: &str,
    properties: &[Property],
  ) {
    self.line(&format!("{header} {{"));
    self.indent += 2;
    for property in properties {
      self.line(&format_property(property));
    }
    self.indent -= 2;
    self.line("}");
  }

  fn rule(
    &mut self,
    header: &Header,
    properties: &[Property],
  ) {
    for ancestor in &header.ancestors {
      if let Some(comment) = ancestor.comment() {
        self.comment(&comment);
      }
      self.line(&format!("{} {{", ancestor.header()));
      self.indent += 2;
    }

    let head = match header.source.as_ref() {
      Some(source) => format!("{} ({source})", header.selector),
      None => header.selector.clone(),
    };
    self.block(&head, properties);

    for _ in &header.ancestors {
      self.indent -= 2;
      self.line("}");
    }
  }
}

fn format_property(property: &Property) -> String {
  let important = if property.important {
    " !important"
  } else {
    ""
  };

  format!(
    "{}{}: {}{important};",
    property.status.tag(),
    property.name,
    property.value
  )
}

fn write_rule(
  writer: &mut Writer,
  rule: &Rule,
) {
  match rule {
    Rule::Matched {
      header, properties, ..
    } => writer.rule(header, properties),
    Rule::Inherited {
      node,
      header,
      properties,
    } => {
      writer.line(&format!("Inherited from {node}:"));
      writer.indent += 2;
      writer.rule(header, properties);
      writer.indent -= 2;
    }
    Rule::Pseudo {
      pseudo_type,
      node,
      header,
      properties,
    } => {
      let inherited = match node {
        Some(node) => format!(" (inherited from {node})"),
        None => String::new(),
      };
      writer.comment(&format!("Pseudo {pseudo_type} element{inherited}"));
      writer.rule(header, properties);
    }
    Rule::Keyframes {
      name,
      source,
      steps,
    } => {
      let header = match source {
        Some(source) => format!("@keyframes {name} ({source})"),
        None => format!("@keyframes {name}"),
      };
      writer.line(&format!("{header} {{"));
      writer.indent += 2;
      for (key_text, properties) in steps {
        writer.block(&key_text.clone(), properties);
      }
      writer.indent -= 2;
      writer.line("}");
    }
  }
}

fn collect_rules(
  matched: &GetMatchedStylesForNodeReturns,
  elements: &Elements,
) -> Vec<Rule> {
  let mut rules = Vec::new();

  if let Some(style) = matched.inline_style.as_ref() {
    push_plain(&mut rules, "element.style", style);
  }

  if let Some(style) = matched.attributes_style.as_ref() {
    push_plain(&mut rules, "[attributes style]", style);
  }

  for rule_match in matched.matched_css_rules.iter().flatten() {
    push_matched(&mut rules, rule_match, elements);
  }

  for pseudo in matched.pseudo_elements.iter().flatten() {
    push_pseudo(&mut rules, pseudo, None, elements);
  }

  let mut inherited_count = 0;

  for entry in matched.inherited.iter().flatten() {
    // The CDP revision vendored here dropped `nodeId` from
    // InheritedStyleEntry, so each entry is paired positionally with the DOM
    // ancestor chain, which Chrome serialises in the same parent-first order.
    let Some(node) = elements.ancestors.get(inherited_count) else {
      break;
    };
    inherited_count += 1;

    if let Some(style) = entry.inline_style.as_ref() {
      push_inherited(&mut rules, node, "element.style", style);
    }

    for rule_match in &entry.matched_css_rules {
      let properties = properties(&rule_match.rule.style);
      if properties.is_empty() {
        continue;
      }
      rules.push(Rule::Inherited {
        node: node.clone(),
        header: header(&rule_match.rule, elements),
        properties,
      });
    }
  }

  for entry in matched.inherited_pseudo_elements.iter().flatten() {
    let Some(node) = elements.ancestors.get(inherited_count) else {
      break;
    };
    inherited_count += 1;

    for pseudo in &entry.pseudo_elements {
      push_pseudo(&mut rules, pseudo, Some(node.clone()), elements);
    }
  }

  for keyframes in matched.css_keyframes_rules.iter().flatten() {
    let steps = keyframe_steps(keyframes);
    if steps.is_empty() {
      continue;
    }

    let source = keyframes
      .keyframes
      .iter()
      .find_map(|keyframe| source_of(&keyframe.style, keyframe.style_sheet_id.as_ref(), elements))
      .map(|location| location.render());

    rules.push(Rule::Keyframes {
      name: keyframes.animation_name.text.clone(),
      source,
      steps,
    });
  }

  for at_rule in matched.css_at_rules.iter().flatten() {
    let properties = properties(&at_rule.style);
    if properties.is_empty() {
      continue;
    }

    rules.push(Rule::Matched {
      header: Header {
        selector: at_rule_selector(at_rule),
        source: source_of(&at_rule.style, at_rule.style_sheet_id.as_ref(), elements)
          .map(|location| location.render()),
        ancestors: at_rule_ancestors(at_rule),
      },
      properties,
      specificity: None,
    });
  }

  for rule in matched.css_position_try_rules.iter().flatten() {
    let properties = properties(&rule.style);
    if properties.is_empty() {
      continue;
    }

    let state = if rule.active { "" } else { "[inactive] " };
    rules.push(Rule::Matched {
      header: Header {
        selector: format!("{state}@position-try {}", rule.name.text),
        source: source_of(&rule.style, rule.style_sheet_id.as_ref(), elements)
          .map(|location| location.render()),
        ancestors: Vec::new(),
      },
      properties,
      specificity: None,
    });
  }

  for rule in matched.css_property_rules.iter().flatten() {
    let properties = properties(&rule.style);
    if properties.is_empty() {
      continue;
    }

    rules.push(Rule::Matched {
      header: Header {
        selector: format!("@property {}", rule.property_name.text),
        source: source_of(&rule.style, rule.style_sheet_id.as_ref(), elements)
          .map(|location| location.render()),
        ancestors: Vec::new(),
      },
      properties,
      specificity: None,
    });
  }

  mark_overloaded(&mut rules);

  rules
}

// Chrome resolves each `@container` query against the concrete ancestor that
// establishes it, and reports that element beside the query. The lookup lives
// in the DOM domain and is per-query, so it runs once over the collected rules
// rather than per rendered rule.
async fn resolve_container_elements(
  page: &Page,
  node_id: NodeId,
  elements: &Elements,
  rules: &mut [Rule],
) {
  let mut queries: Vec<(String, Option<String>)> = Vec::new();

  for rule in rules.iter() {
    let ancestors = match rule {
      Rule::Matched { header, .. }
      | Rule::Inherited { header, .. }
      | Rule::Pseudo { header, .. } => &header.ancestors,
      Rule::Keyframes { .. } => continue,
    };

    for ancestor in ancestors {
      if let Ancestor::Container { query, name, .. } = ancestor
        && !queries
          .iter()
          .any(|(existing, existing_name)| existing == query && existing_name == name)
      {
        queries.push((query.clone(), name.clone()));
      }
    }
  }

  if queries.is_empty() {
    return;
  }

  let mut resolved: HashMap<(String, Option<String>), String> = HashMap::new();

  for (query, name) in &queries {
    let params = GetContainerForNodeParams {
      container_name: name.clone(),
      ..GetContainerForNodeParams::new(node_id)
    };

    let Ok(response) = page.execute(params).await else {
      continue;
    };
    let Some(container) = response.result.node_id else {
      continue;
    };

    let Ok(described) = page
      .execute(DescribeNodeParams::builder().node_id(container).build())
      .await
    else {
      continue;
    };

    let node = described.result.node;
    let uid = elements.uids.get(node.backend_node_id.inner());

    let label = match uid {
      Some(uid) => format!("{} (uid: \"{uid}\")", simple_selector(&node)),
      None => simple_selector(&node),
    };

    resolved.insert((query.clone(), name.clone()), label);
  }

  if resolved.is_empty() {
    return;
  }

  for rule in rules.iter_mut() {
    let ancestors = match rule {
      Rule::Matched { header, .. }
      | Rule::Inherited { header, .. }
      | Rule::Pseudo { header, .. } => &mut header.ancestors,
      Rule::Keyframes { .. } => continue,
    };

    for ancestor in ancestors.iter_mut() {
      if let Ancestor::Container {
        query,
        name,
        element,
      } = ancestor
        && let Some(label) = resolved.get(&(query.clone(), name.clone()))
      {
        *element = Some(label.clone());
      }
    }
  }
}

fn push_plain(
  rules: &mut Vec<Rule>,
  selector: &str,
  style: &CssStyle,
) {
  let properties = properties(style);
  if properties.is_empty() {
    return;
  }

  rules.push(Rule::Matched {
    header: Header {
      selector: selector.to_owned(),
      source: None,
      ancestors: Vec::new(),
    },
    properties,
    specificity: None,
  });
}

fn push_matched(
  rules: &mut Vec<Rule>,
  rule_match: &RuleMatch,
  elements: &Elements,
) {
  let properties = properties(&rule_match.rule.style);
  if properties.is_empty() {
    return;
  }

  let mut header = header(&rule_match.rule, elements);
  header.selector = matching_selector_text(rule_match)
    .unwrap_or_else(|| rule_match.rule.selector_list.text.clone());

  rules.push(Rule::Matched {
    header,
    properties,
    specificity: specificity_of(rule_match),
  });
}

fn push_inherited(
  rules: &mut Vec<Rule>,
  node: &str,
  selector: &str,
  style: &CssStyle,
) {
  let properties = properties(style);
  if properties.is_empty() {
    return;
  }

  rules.push(Rule::Inherited {
    node: node.to_owned(),
    header: Header {
      selector: selector.to_owned(),
      source: None,
      ancestors: Vec::new(),
    },
    properties,
  });
}

fn push_pseudo(
  rules: &mut Vec<Rule>,
  pseudo: &PseudoElementMatches,
  node: Option<String>,
  elements: &Elements,
) {
  let pseudo_type = pseudo_label(pseudo);

  for rule_match in &pseudo.matches {
    let properties = properties(&rule_match.rule.style);
    if properties.is_empty() {
      continue;
    }

    let mut header = header(&rule_match.rule, elements);
    header.selector = matching_selector_text(rule_match)
      .unwrap_or_else(|| rule_match.rule.selector_list.text.clone());

    rules.push(Rule::Pseudo {
      pseudo_type: pseudo_type.clone(),
      node: node.clone(),
      header,
      properties,
    });
  }
}

// Chrome renders a selector group through the single selector that matched the
// element, while a lone selector keeps its full list text.
fn matching_selector_text(rule_match: &RuleMatch) -> Option<String> {
  if rule_match.rule.selector_list.selectors.len() <= 1 {
    return None;
  }

  let matched: Vec<&str> = rule_match
    .matching_selectors
    .iter()
    .filter_map(|index| {
      rule_match
        .rule
        .selector_list
        .selectors
        .get(*index as usize)
        .map(|selector| selector.text.as_str())
    })
    .collect();

  (!matched.is_empty()).then(|| matched.join(", "))
}

fn properties(style: &CssStyle) -> Vec<Property> {
  style
    .css_properties
    .iter()
    .map(structured_property)
    .collect()
}

// CDP reports `text` as the declaration exactly as authored, including the
// `!important` annotation that is also surfaced through `important`.
fn structured_property(
  property: &chromiumoxide::cdp::browser_protocol::css::CssProperty
) -> Property {
  let important = property.important == Some(true);

  let raw = property.text.clone().unwrap_or_else(|| {
    if important {
      format!("{} !important", property.value)
    } else {
      property.value.clone()
    }
  });

  let value = if important {
    strip_important(&raw)
  } else {
    raw.trim_end().to_owned()
  };

  Property {
    status: if property.parsed_ok == Some(false) {
      PropStatus::Invalid
    } else if property.disabled == Some(true) {
      PropStatus::Disabled
    } else {
      PropStatus::Active
    },
    name: property.name.clone(),
    value,
    important,
  }
}

fn strip_important(value: &str) -> String {
  let trimmed = value.trim_end();
  let lower = trimmed.to_ascii_lowercase();

  let Some(head) = lower
    .strip_suffix("important")
    .and_then(|_| trimmed.get(..trimmed.len() - "important".len()))
  else {
    return trimmed.to_owned();
  };

  match head.trim_end().strip_suffix('!') {
    Some(head) => head.trim_end().to_owned(),
    None => trimmed.to_owned(),
  }
}

fn header(
  rule: &CssRule,
  elements: &Elements,
) -> Header {
  Header {
    selector: rule.selector_list.text.clone(),
    source: source_of_rule(rule, elements),
    ancestors: ancestors_of(rule),
  }
}

fn ancestors_of(rule: &CssRule) -> Vec<Ancestor> {
  let Some(types) = rule.rule_types.as_ref() else {
    return Vec::new();
  };

  let mut media = rule.media.iter().flatten();
  let mut containers = rule.container_queries.iter().flatten();
  let mut layers = rule.layers.iter().flatten();
  let mut scopes = rule.scopes.iter().flatten();
  let mut supports = rule.supports.iter().flatten();
  let mut nestings = rule.nesting_selectors.iter().flatten();

  let mut ancestors = Vec::new();

  for rule_type in types {
    let ancestor = match rule_type {
      CssRuleType::MediaRule => media.next().map(|item| Ancestor::Media(item.text.clone())),
      CssRuleType::ContainerRule => containers.next().map(|item| Ancestor::Container {
        query: item.text.clone(),
        name: item.name.clone().filter(|name| !name.is_empty()),
        element: None,
      }),
      CssRuleType::LayerRule => layers
        .next()
        .map(|item| Ancestor::Layer(Some(item.text.clone()).filter(|text| !text.is_empty()))),
      CssRuleType::ScopeRule => scopes
        .next()
        .map(|item| Ancestor::Scope(Some(item.text.clone()).filter(|text| !text.is_empty()))),
      CssRuleType::SupportsRule => supports
        .next()
        .map(|item| Ancestor::Supports(item.text.clone())),
      CssRuleType::StartingStyleRule => Some(Ancestor::StartingStyle),
      CssRuleType::StyleRule => nestings.next().map(|item| Ancestor::Nesting(item.clone())),
    };

    if let Some(ancestor) = ancestor {
      ancestors.push(ancestor);
    }
  }

  ancestors.reverse();
  ancestors
}

fn at_rule_selector(at_rule: &CssAtRule) -> String {
  let at_rule_type = at_rule.r#type.as_ref();

  if let Some(subsection) = at_rule.subsection.as_ref() {
    return format!("@{}", subsection.as_ref());
  }

  match at_rule.name.as_ref().map(|name| name.text.as_str()) {
    Some(name) => format!("@{at_rule_type} {name}"),
    None => format!("@{at_rule_type}"),
  }
}

fn at_rule_ancestors(at_rule: &CssAtRule) -> Vec<Ancestor> {
  match at_rule.subsection.is_some() {
    true => vec![Ancestor::Unsupported {
      at_rule_type: at_rule.r#type.as_ref().to_owned(),
      name: at_rule.name.as_ref().map(|name| name.text.clone()),
    }],
    false => Vec::new(),
  }
}

fn pseudo_label(pseudo: &PseudoElementMatches) -> String {
  match pseudo.pseudo_identifier.as_deref() {
    Some(identifier) => format!("::{}({identifier})", pseudo.pseudo_type.as_ref()),
    None => format!("::{}", pseudo.pseudo_type.as_ref()),
  }
}

struct SourceLocation {
  name: String,
  line: Option<i64>,
}

impl SourceLocation {
  fn render(&self) -> String {
    match self.line {
      Some(line) => format!("{}:{}", self.name, line + 1),
      None => self.name.clone(),
    }
  }
}

fn source_of_rule(
  rule: &CssRule,
  elements: &Elements,
) -> Option<String> {
  Some(match rule.origin {
    StyleSheetOrigin::Injected => "injected stylesheet".to_owned(),
    StyleSheetOrigin::Inspector => "via inspector".to_owned(),
    _ => source_of(&rule.style, rule.style_sheet_id.as_ref(), elements)
      .map(|location| location.render())
      .unwrap_or_else(|| "<style>".to_owned()),
  })
}

fn source_of(
  style: &CssStyle,
  sheet_id: Option<&StyleSheetId>,
  elements: &Elements,
) -> Option<SourceLocation> {
  let sheet = sheet_id.and_then(|id| elements.sheets.get(id.as_ref()));

  let name = match sheet {
    Some(sheet) => sheet.source_name(),
    None => "<style>".to_owned(),
  };

  let line = sheet.and_then(|sheet| sheet.line(style.range.as_ref()));

  Some(SourceLocation { name, line })
}

fn keyframe_steps(keyframes: &CssKeyframesRule) -> Vec<(String, Vec<Property>)> {
  let mut steps = Vec::new();

  for keyframe in &keyframes.keyframes {
    let properties = properties(&keyframe.style);
    if properties.is_empty() {
      continue;
    }

    steps.push((keyframe.key_text.text.clone(), properties));
  }

  steps
}

fn simple_selector(node: &Node) -> String {
  let attributes = node.attributes.as_ref();

  let attribute = |wanted: &str| -> Option<String> {
    attributes?
      .chunks(2)
      .find(|pair| pair.first().is_some_and(|key| key == wanted))
      .and_then(|pair| pair.get(1))
      .cloned()
  };

  match (attribute("id"), attribute("class")) {
    (Some(id), _) => format!("{}#{id}", node.local_name),
    (None, Some(classes)) => match classes.split_whitespace().next() {
      Some(class_name) => format!("{}.{class_name}", node.local_name),
      None => node.local_name.clone(),
    },
    (None, None) => node.local_name.clone(),
  }
}

async fn ancestor_selectors(
  page: &Page,
  node_id: NodeId,
) -> Vec<String> {
  let mut selectors = Vec::new();
  let mut current = Some(node_id);
  let mut depth = 0;

  while let Some(id) = current {
    if depth >= ANCESTOR_WALK_LIMIT {
      break;
    }
    depth += 1;

    let Ok(response) = page
      .execute(DescribeNodeParams::builder().node_id(id).build())
      .await
    else {
      break;
    };

    let node = response.result.node;
    let parent = node
      .parent_id
      .or_else(|| node.content_document.as_ref().map(|doc| doc.node_id));
    current = parent;

    if parent.is_some() {
      selectors.push(simple_selector(&node));
    }
  }

  selectors
}

async fn sheet_texts(
  page: &Page,
  matched: &GetMatchedStylesForNodeReturns,
) -> HashMap<String, Sheet> {
  let mut ids: Vec<String> = Vec::new();

  let mut push = |id: Option<&StyleSheetId>| {
    if let Some(id) = id {
      let key = id.as_ref().to_owned();
      if !ids.contains(&key) {
        ids.push(key);
      }
    }
  };

  for rule_match in matched.matched_css_rules.iter().flatten() {
    push(rule_match.rule.style_sheet_id.as_ref());
  }
  for pseudo in matched.pseudo_elements.iter().flatten() {
    for rule_match in &pseudo.matches {
      push(rule_match.rule.style_sheet_id.as_ref());
    }
  }
  for entry in matched.inherited.iter().flatten() {
    push(
      entry
        .inline_style
        .as_ref()
        .and_then(|style| style.style_sheet_id.as_ref()),
    );
    for rule_match in &entry.matched_css_rules {
      push(rule_match.rule.style_sheet_id.as_ref());
    }
  }
  for entry in matched.inherited_pseudo_elements.iter().flatten() {
    for pseudo in &entry.pseudo_elements {
      for rule_match in &pseudo.matches {
        push(rule_match.rule.style_sheet_id.as_ref());
      }
    }
  }
  for keyframes in matched.css_keyframes_rules.iter().flatten() {
    for keyframe in &keyframes.keyframes {
      push(keyframe.style_sheet_id.as_ref());
    }
  }
  for at_rule in matched.css_at_rules.iter().flatten() {
    push(at_rule.style_sheet_id.as_ref());
  }
  for rule in matched.css_position_try_rules.iter().flatten() {
    push(rule.style_sheet_id.as_ref());
  }
  for rule in matched.css_property_rules.iter().flatten() {
    push(rule.style_sheet_id.as_ref());
  }

  let mut sheets = HashMap::new();

  for (index, id) in ids.iter().enumerate() {
    let Ok(response) = page
      .execute(GetStyleSheetTextParams::new(StyleSheetId::new(id.clone())))
      .await
    else {
      continue;
    };

    // Chrome only exposes line numbers through the SDK's stylesheet objects,
    // which are not part of CDP, so the sheet text is indexed here instead.
    // The document is always reported first, and every rule in it carries the
    // offset of its own `<style>` element counted from the document start.
    let (source_url, start_line, start_column) = if index == 0 {
      (String::new(), 0, 0)
    } else {
      (id.clone(), 0, 0)
    };

    sheets.insert(
      id.clone(),
      Sheet {
        text: response.result.text,
        source_url,
        start_line,
        start_column,
      },
    );
  }

  sheets
}

fn internal(error: chromiumoxide::error::CdpError) -> ErrorData {
  ErrorData::internal_error(error.to_string(), None)
}
