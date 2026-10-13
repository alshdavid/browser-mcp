use std::path::PathBuf;

pub fn chrome_binary_default() -> anyhow::Result<PathBuf> {
  chrome_binary_candidates()
    .into_iter()
    .find(|path| path.is_file())
    .ok_or_else(|| {
      anyhow::anyhow!(
        "could not find a Google Chrome installation; pass --chrome-binary-path or set CHROME_BINARY_PATH"
      )
    })
}

#[cfg(target_os = "macos")]
fn chrome_binary_candidates() -> Vec<PathBuf> {
  let mut candidates = vec![
    PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
    PathBuf::from("/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary"),
    PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium"),
  ];

  if let Some(home) = std::env::var_os("HOME") {
    candidates.push(
      PathBuf::from(home).join("Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
    );
  }

  candidates
}

#[cfg(target_os = "windows")]
fn chrome_binary_candidates() -> Vec<PathBuf> {
  let program_files = std::env::var_os("PROGRAMFILES")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("C:\\Program Files"));

  let program_files_x86 = std::env::var_os("PROGRAMFILES(X86)")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("C:\\Program Files (x86)"));

  let local_app_data = std::env::var_os("LOCALAPPDATA")
    .map(PathBuf::from)
    .or_else(|| {
      std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("AppData").join("Local"))
    })
    .unwrap_or_else(|| PathBuf::from("C:\\Program Files").join("..").join("Users"));

  let chrome = |vendor: &str| {
    PathBuf::from(vendor)
      .join("Chrome")
      .join("Application")
      .join("chrome.exe")
  };

  vec![
    program_files.join(chrome("Google")),
    program_files_x86.join(chrome("Google")),
    local_app_data.join(chrome("Google")),
    program_files.join(chrome("Google Chrome Canary")),
    local_app_data.join(
      PathBuf::from("Google Chrome Canary")
        .join("Application")
        .join("chrome.exe"),
    ),
    program_files.join(
      PathBuf::from("Chromium")
        .join("Application")
        .join("chrome.exe"),
    ),
  ]
}

#[cfg(target_os = "linux")]
fn chrome_binary_candidates() -> Vec<PathBuf> {
  [
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/snap/bin/chromium",
    "/usr/local/bin/google-chrome",
    "/usr/local/bin/chromium",
    "/opt/google/chrome/google-chrome",
  ]
  .into_iter()
  .map(PathBuf::from)
  .collect()
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn chrome_binary_candidates() -> Vec<PathBuf> {
  Vec::new()
}
