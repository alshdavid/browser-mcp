use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::input::DispatchKeyEventParams;
use chromiumoxide::cdp::browser_protocol::input::DispatchKeyEventType;
use chromiumoxide::keys::KeyDefinition;
use chromiumoxide::keys::get_key_definition;

const VALID_KEYS: &[&str] = &[
  "0",
  "1",
  "2",
  "3",
  "4",
  "5",
  "6",
  "7",
  "8",
  "9",
  "Power",
  "Eject",
  "Abort",
  "Help",
  "Backspace",
  "Tab",
  "Numpad5",
  "NumpadEnter",
  "Enter",
  "\r",
  "\n",
  "ShiftLeft",
  "ShiftRight",
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "Pause",
  "CapsLock",
  "Escape",
  "Convert",
  "NonConvert",
  "Space",
  "Numpad9",
  "PageUp",
  "Numpad3",
  "PageDown",
  "End",
  "Numpad1",
  "Home",
  "Numpad7",
  "ArrowLeft",
  "Numpad4",
  "Numpad8",
  "ArrowUp",
  "ArrowRight",
  "Numpad6",
  "Numpad2",
  "ArrowDown",
  "Select",
  "Open",
  "PrintScreen",
  "Insert",
  "Numpad0",
  "Delete",
  "NumpadDecimal",
  "Digit0",
  "Digit1",
  "Digit2",
  "Digit3",
  "Digit4",
  "Digit5",
  "Digit6",
  "Digit7",
  "Digit8",
  "Digit9",
  "KeyA",
  "KeyB",
  "KeyC",
  "KeyD",
  "KeyE",
  "KeyF",
  "KeyG",
  "KeyH",
  "KeyI",
  "KeyJ",
  "KeyK",
  "KeyL",
  "KeyM",
  "KeyN",
  "KeyO",
  "KeyP",
  "KeyQ",
  "KeyR",
  "KeyS",
  "KeyT",
  "KeyU",
  "KeyV",
  "KeyW",
  "KeyX",
  "KeyY",
  "KeyZ",
  "MetaLeft",
  "MetaRight",
  "ContextMenu",
  "NumpadMultiply",
  "NumpadAdd",
  "NumpadSubtract",
  "NumpadDivide",
  "F1",
  "F2",
  "F3",
  "F4",
  "F5",
  "F6",
  "F7",
  "F8",
  "F9",
  "F10",
  "F11",
  "F12",
  "F13",
  "F14",
  "F15",
  "F16",
  "F17",
  "F18",
  "F19",
  "F20",
  "F21",
  "F22",
  "F23",
  "F24",
  "NumLock",
  "ScrollLock",
  "AudioVolumeMute",
  "AudioVolumeDown",
  "AudioVolumeUp",
  "MediaTrackNext",
  "MediaTrackPrevious",
  "MediaStop",
  "MediaPlayPause",
  "Semicolon",
  "Equal",
  "NumpadEqual",
  "Comma",
  "Minus",
  "Period",
  "Slash",
  "Backquote",
  "BracketLeft",
  "Backslash",
  "BracketRight",
  "Quote",
  "AltGraph",
  "Props",
  "Cancel",
  "Clear",
  "Shift",
  "Control",
  "Alt",
  "Accept",
  "ModeChange",
  " ",
  "Print",
  "Execute",
  "\u{0}",
  "a",
  "b",
  "c",
  "d",
  "e",
  "f",
  "g",
  "h",
  "i",
  "j",
  "k",
  "l",
  "m",
  "n",
  "o",
  "p",
  "q",
  "r",
  "s",
  "t",
  "u",
  "v",
  "w",
  "x",
  "y",
  "z",
  "Meta",
  "*",
  "+",
  "-",
  "/",
  ";",
  "=",
  ",",
  ".",
  "`",
  "[",
  "\\",
  "]",
  "'",
  "Attn",
  "CrSel",
  "ExSel",
  "EraseEof",
  "Play",
  "ZoomOut",
  ")",
  "!",
  "@",
  "#",
  "$",
  "%",
  "^",
  "&",
  "(",
  "A",
  "B",
  "C",
  "D",
  "E",
  "F",
  "G",
  "H",
  "I",
  "J",
  "K",
  "L",
  "M",
  "N",
  "O",
  "P",
  "Q",
  "R",
  "S",
  "T",
  "U",
  "V",
  "W",
  "X",
  "Y",
  "Z",
  ":",
  "<",
  "_",
  ">",
  "?",
  "~",
  "{",
  "|",
  "}",
  "\"",
  "SoftLeft",
  "SoftRight",
  "Camera",
  "Call",
  "EndCall",
  "VolumeDown",
  "VolumeUp",
];

const ALIAS_KEYS: &[(&str, &str)] = &[
  ("\r", "Enter"),
  ("\n", "Enter"),
  ("ShiftLeft", "Shift"),
  ("ShiftRight", "Shift"),
  ("ControlLeft", "Control"),
  ("ControlRight", "Control"),
  ("AltLeft", "Alt"),
  ("AltRight", "Alt"),
  ("MetaLeft", "Meta"),
  ("MetaRight", "Meta"),
  ("Space", " "),
  ("Numpad0", "0"),
  ("Numpad1", "1"),
  ("Numpad2", "2"),
  ("Numpad3", "3"),
  ("Numpad4", "4"),
  ("Numpad5", "5"),
  ("Numpad6", "6"),
  ("Numpad7", "7"),
  ("Numpad8", "8"),
  ("Numpad9", "9"),
  ("NumpadAdd", "+"),
  ("NumpadSubtract", "-"),
  ("NumpadMultiply", "*"),
  ("NumpadDivide", "/"),
  ("NumpadDecimal", "."),
  ("NumpadEnter", "Enter"),
  ("NumpadEqual", "="),
  ("Digit0", "0"),
  ("Digit1", "1"),
  ("Digit2", "2"),
  ("Digit3", "3"),
  ("Digit4", "4"),
  ("Digit5", "5"),
  ("Digit6", "6"),
  ("Digit7", "7"),
  ("Digit8", "8"),
  ("Digit9", "9"),
  ("KeyA", "a"),
  ("KeyB", "b"),
  ("KeyC", "c"),
  ("KeyD", "d"),
  ("KeyE", "e"),
  ("KeyF", "f"),
  ("KeyG", "g"),
  ("KeyH", "h"),
  ("KeyI", "i"),
  ("KeyJ", "j"),
  ("KeyK", "k"),
  ("KeyL", "l"),
  ("KeyM", "m"),
  ("KeyN", "n"),
  ("KeyO", "o"),
  ("KeyP", "p"),
  ("KeyQ", "q"),
  ("KeyR", "r"),
  ("KeyS", "s"),
  ("KeyT", "t"),
  ("KeyU", "u"),
  ("KeyV", "v"),
  ("KeyW", "w"),
  ("KeyX", "x"),
  ("KeyY", "y"),
  ("KeyZ", "z"),
  ("Semicolon", ";"),
  ("Equal", "="),
  ("Comma", ","),
  ("Minus", "-"),
  ("Period", "."),
  ("Slash", "/"),
  ("Backquote", "`"),
  ("BracketLeft", "["),
  ("BracketRight", "]"),
  ("Backslash", "\\"),
  ("Quote", "'"),
];

const MODIFIER_MASK_CONTROL: i64 = 2;
const MODIFIER_MASK_ALT: i64 = 1;
const MODIFIER_MASK_META: i64 = 4;

pub(crate) fn parse_key(key_input: &str) -> Result<Vec<String>, String> {
  let mut current = String::new();
  let mut tokens: Vec<String> = Vec::new();

  for character in key_input.chars() {
    if character == '+' && !current.is_empty() {
      tokens.push(validate_key(&current)?);
      current.clear();
    } else {
      current.push(character);
    }
  }

  if !current.is_empty() {
    tokens.push(validate_key(&current)?);
  }

  if tokens.is_empty() {
    return Err(format!("Key {key_input} could not be parsed."));
  }

  if tokens
    .iter()
    .collect::<std::collections::HashSet<_>>()
    .len()
    != tokens.len()
  {
    return Err(format!("Key {key_input} contains duplicate keys."));
  }

  let primary = tokens.pop().unwrap_or_default();
  tokens.insert(0, primary);

  Ok(tokens)
}

fn validate_key(key: &str) -> Result<String, String> {
  if VALID_KEYS.contains(&key) {
    return Ok(key.to_owned());
  }

  Err(format!(
    "{key} is invalid. Valid keys are: {}.",
    VALID_KEYS.join(",")
  ))
}

pub(crate) fn resolve_key(key: &str) -> Option<&'static KeyDefinition> {
  if let Some(definition) = get_key_definition(key) {
    return Some(definition);
  }

  let alias = ALIAS_KEYS
    .iter()
    .find(|(from, _)| *from == key)
    .map(|(_, to)| *to)?;

  get_key_definition(alias)
}

pub(crate) async fn dispatch(
  page: &Page,
  tokens: &[String],
) -> Result<(), String> {
  let (primary, modifiers) = tokens
    .split_first()
    .ok_or_else(|| "Key could not be parsed.".to_owned())?;

  let mut held: Vec<&'static KeyDefinition> = Vec::new();
  let mut result = Ok(());

  for modifier in modifiers {
    let definition = resolve_key(modifier).ok_or_else(|| format!("Key not found: {modifier}"))?;

    if let Err(error) = key_event(
      page,
      definition,
      DispatchKeyEventType::KeyDown,
      false,
      modifier_mask(&held),
    )
    .await
    {
      result = Err(error);
      break;
    }

    held.push(definition);
  }

  if result.is_ok() {
    match resolve_key(primary) {
      Some(definition) => {
        let mask = modifier_mask(&held);
        result = key_event(page, definition, DispatchKeyEventType::KeyDown, true, mask).await;
        if result.is_ok() {
          result = key_event(page, definition, DispatchKeyEventType::KeyUp, false, mask).await;
        }
      }
      None => result = Err(format!("Key not found: {primary}")),
    }
  }

  for (index, definition) in held.iter().enumerate().rev() {
    let mask = modifier_mask(&held[..index]);
    let released = key_event(page, definition, DispatchKeyEventType::KeyUp, false, mask).await;
    if result.is_ok() {
      result = released;
    }
  }

  result
}

fn modifier_mask(held: &[&'static KeyDefinition]) -> i64 {
  held.iter().fold(0, |mask, definition| {
    mask
      | match definition.key {
        "Control" => MODIFIER_MASK_CONTROL,
        "Alt" => MODIFIER_MASK_ALT,
        "Meta" => MODIFIER_MASK_META,
        _ => 0,
      }
  })
}

pub(crate) async fn key_event(
  page: &Page,
  definition: &KeyDefinition,
  event_type: DispatchKeyEventType,
  is_press_down: bool,
  modifiers: i64,
) -> Result<(), String> {
  let text = if is_press_down && event_type == DispatchKeyEventType::KeyDown {
    definition
      .text
      .or(Some(definition.key))
      .filter(|text| text.len() <= 1)
  } else {
    None
  };

  let event_type = match (event_type, text.is_some()) {
    (DispatchKeyEventType::KeyDown, false) => DispatchKeyEventType::RawKeyDown,
    (other, _) => other,
  };

  let mut builder = DispatchKeyEventParams::builder()
    .r#type(event_type)
    .modifiers(modifiers)
    .key(definition.key)
    .code(definition.code)
    .windows_virtual_key_code(definition.key_code)
    .native_virtual_key_code(definition.key_code);

  if let Some(text) = text {
    builder = builder.text(text);
  }

  page
    .execute(builder.build().map_err(|error| error.to_string())?)
    .await
    .map_err(|error| error.to_string())?;

  Ok(())
}
