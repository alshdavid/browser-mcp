use std::path::PathBuf;

pub fn chrome_binary_default() -> anyhow::Result<PathBuf> {
  chrome_binary_candidates()
    .into_iter()
    .find(|path| path.exists())
    .ok_or_else(|| {
      anyhow::anyhow!(
        "could not find a Google Chrome installation; pass --chrome-binary-path or set CHROME_BINARY_PATH"
      )
    })
}

#[cfg(target_os = "macos")]
fn chrome_binary_candidates() -> Vec<PathBuf> {
  let app = "Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
  let mut candidates = vec![PathBuf::from("/").join(app)];
  if let Some(home) = std::env::var_os("HOME") {
    candidates.push(PathBuf::from(home).join(app));
  }
  candidates
}

#[cfg(target_os = "windows")]
fn chrome_binary_candidates() -> Vec<PathBuf> {
  let binary = PathBuf::from("Google")
    .join("Chrome")
    .join("Application")
    .join("chrome.exe");
  ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
    .iter()
    .filter_map(|name| std::env::var_os(name))
    .map(|base| PathBuf::from(base).join(&binary))
    .collect()
}

#[cfg(target_os = "linux")]
fn chrome_binary_candidates() -> Vec<PathBuf> {
  [
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/snap/bin/chromium",
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
