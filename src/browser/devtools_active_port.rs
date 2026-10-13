use std::path::Path;
use std::path::PathBuf;

pub fn chrome_user_data_dir() -> anyhow::Result<PathBuf> {
  let dir = chrome_user_data_dir_candidates()
    .into_iter()
    .find(|path| path.is_dir())
    .ok_or_else(|| {
      anyhow::anyhow!(
        "could not find a Google Chrome user data directory; enable remote debugging in the running Chrome instance at chrome://inspect/#remote-debugging"
      )
    })?;

  Ok(dir)
}

// Chrome launched without an explicit `--remote-debugging-port` writes the port
// it chose to `DevToolsActivePort` inside its profile, which is the only way to
// reach an instance that is already running.
pub fn devtools_active_port_ws_url(user_data_dir: &Path) -> anyhow::Result<String> {
  let port_path = user_data_dir.join("DevToolsActivePort");

  let content = std::fs::read_to_string(&port_path).map_err(|error| {
    anyhow::anyhow!(
      "could not read {}: {error}; check that Chrome is running and that remote debugging is enabled by going to chrome://inspect/#remote-debugging",
      port_path.display()
    )
  })?;

  let mut lines = content
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty());
  let port = lines.next().unwrap_or_default();
  let path = lines.next().unwrap_or_default();

  let port = port
    .parse::<u16>()
    .map_err(|_| anyhow::anyhow!("invalid port '{port}' in {}", port_path.display()))?;

  if port == 0 || path.is_empty() {
    return Err(anyhow::anyhow!(
      "invalid DevToolsActivePort '{content}' in {}",
      port_path.display()
    ));
  }

  Ok(format!("ws://127.0.0.1:{port}{path}"))
}

#[cfg(target_os = "macos")]
fn chrome_user_data_dir_candidates() -> Vec<PathBuf> {
  let mut candidates = vec![
    PathBuf::from("/Library/Application Support/Google/Chrome"),
    PathBuf::from("/Library/Application Support/Chromium"),
  ];

  if let Some(home) = std::env::var_os("HOME") {
    let home = PathBuf::from(home);
    candidates.push(home.join("Library/Application Support/Google/Chrome"));
    candidates.push(home.join("Library/Application Support/Chromium"));
  }

  candidates
}

#[cfg(target_os = "windows")]
fn chrome_user_data_dir_candidates() -> Vec<PathBuf> {
  let local_app_data = std::env::var_os("LOCALAPPDATA")
    .map(PathBuf::from)
    .or_else(|| {
      std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("AppData").join("Local"))
    })
    .unwrap_or_else(|| PathBuf::from("C:\\Program Files").join("..").join("Users"));

  vec![
    local_app_data
      .join("Google")
      .join("Chrome")
      .join("User Data"),
    local_app_data
      .join("Google")
      .join("Chrome Beta")
      .join("User Data"),
    local_app_data
      .join("Google")
      .join("Chrome Dev")
      .join("User Data"),
    local_app_data
      .join("Google")
      .join("Chrome SxS")
      .join("User Data"),
    local_app_data.join("Chromium").join("User Data"),
  ]
}

#[cfg(target_os = "linux")]
fn chrome_user_data_dir_candidates() -> Vec<PathBuf> {
  let mut candidates = vec![
    PathBuf::from("/config/google-chrome"),
    PathBuf::from("/config/chromium"),
  ];

  if let Some(home) = std::env::var_os("HOME") {
    let home = PathBuf::from(home);
    candidates.push(home.join(".config/google-chrome"));
    candidates.push(home.join(".config/chromium"));
  }

  candidates
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn chrome_user_data_dir_candidates() -> Vec<PathBuf> {
  Vec::new()
}
