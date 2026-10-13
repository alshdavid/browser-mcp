use rmcp::schemars;

pub type Descriptions = &'static [(&'static str, &'static str, &'static str)];
pub type ItemDescriptions = &'static [(&'static str, &'static str, &'static str)];
pub type NumericBounds = &'static [(&'static str, &'static str, Option<i64>, Option<i64>, bool)];
pub type MinItems = &'static [(&'static str, &'static str, u64)];

pub const DESCRIPTIONS: Descriptions = &[];

pub const ITEM_DESCRIPTIONS: ItemDescriptions = &[
  (
    "evaluate_script",
    "args",
    "The uid of an element on the page from the page content snapshot",
  ),
  ("fill_form", "elements", "An element to fill out"),
];

pub const NUMERIC_BOUNDS: NumericBounds = &[
  ("emulate", "cpuThrottlingRate", Some(1), Some(20), false),
  (
    "get_css_styles",
    "pageSize",
    None,
    Some(9007199254740991),
    true,
  ),
  (
    "get_css_styles",
    "pageIdx",
    Some(0),
    Some(9007199254740991),
    false,
  ),
  (
    "list_console_messages",
    "pageSize",
    None,
    Some(9007199254740991),
    true,
  ),
  (
    "list_console_messages",
    "pageIdx",
    Some(0),
    Some(9007199254740991),
    false,
  ),
  (
    "list_network_requests",
    "pageSize",
    None,
    Some(9007199254740991),
    true,
  ),
  (
    "list_network_requests",
    "pageIdx",
    Some(0),
    Some(9007199254740991),
    false,
  ),
  (
    "navigate_page",
    "timeout",
    Some(-9007199254740991),
    Some(9007199254740991),
    false,
  ),
  (
    "new_page",
    "timeout",
    Some(-9007199254740991),
    Some(9007199254740991),
    false,
  ),
  (
    "wait_for",
    "timeout",
    Some(-9007199254740991),
    Some(9007199254740991),
    false,
  ),
  ("take_screenshot", "quality", Some(0), Some(100), false),
];

pub const MIN_ITEMS: MinItems = &[("upload_file", "filePaths", 1), ("wait_for", "text", 1)];

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum DialogAction {
  #[serde(rename = "accept")]
  Accept,
  #[serde(rename = "dismiss")]
  Dismiss,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
pub enum NavigationType {
  #[serde(rename = "url")]
  Url,
  #[serde(rename = "back")]
  Back,
  #[serde(rename = "forward")]
  Forward,
  #[serde(rename = "reload")]
  Reload,
}
