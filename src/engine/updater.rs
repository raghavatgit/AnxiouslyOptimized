use crate::engine::run_powershell_encoded;
use crate::models::{UpdateCheckResult, UpdateStatus};
use std::cmp::Ordering;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO_SLUG: &str = "raghavatgit/AnxiouslyOptimized";

/// Checks GitHub Releases API first, falling back to `main/version.json`, and compares semver against `APP_VERSION`.
pub fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let script = format!(
        "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13; \
         $headers = @{{ 'User-Agent' = 'OptimizerByRaghav/{0}' }}; \
         try {{ \
             $rel = Invoke-RestMethod -Uri 'https://api.github.com/repos/{1}/releases/latest' -Headers $headers -TimeoutSec 8 -ErrorAction Stop; \
             $ver = ($rel.tag_name -replace '^[^0-9]*','').Trim(); \
             $setupUrl = 'https://raw.githubusercontent.com/{1}/main/dist/OptimizerByRaghav-Setup.exe'; \
             foreach ($a in $rel.assets) {{ \
                 if ($a.name -match 'Setup\\.exe$') {{ $setupUrl = $a.browser_download_url; break }} \
                 elseif ($a.name -match 'OptimizerByRaghav\\.exe$') {{ $setupUrl = $a.browser_download_url }} \
             }} \
             $obj = @{{ \
                 version = $ver; \
                 release_url = [string]$rel.html_url; \
                 setup_url = [string]$setupUrl; \
                 notes = [string]$rel.body \
             }}; \
             $obj | ConvertTo-Json -Compress \
         }} catch {{ \
             $vjson = Invoke-RestMethod -Uri 'https://raw.githubusercontent.com/{1}/main/version.json' -Headers $headers -TimeoutSec 8 -ErrorAction Stop; \
             $ver = ($vjson.version -replace '^[^0-9]*','').Trim(); \
             $obj = @{{ \
                 version = $ver; \
                 release_url = [string]$vjson.release_url; \
                 setup_url = [string]$vjson.setup_download_url; \
                 notes = [string]$vjson.notes \
             }}; \
             $obj | ConvertTo-Json -Compress \
         }}",
        APP_VERSION, GITHUB_REPO_SLUG
    );

    let out = run_powershell_encoded(&script, 15_000)?;
    if out.exit_code != 0 || out.stdout.trim().is_empty() {
        return Err(format!(
            "Failed querying GitHub for updates: {}",
            if out.stderr.trim().is_empty() {
                "Network unreachable or no release metadata found"
            } else {
                out.stderr.trim()
            }
        ));
    }

    let val: serde_json::Value = serde_json::from_str(out.stdout.trim())
        .map_err(|e| format!("Invalid update response JSON: {e}"))?;

    let latest_version = val
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or(APP_VERSION)
        .trim()
        .trim_start_matches('v')
        .to_string();

    let release_url = val
        .get("release_url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("https://github.com/raghavatgit/AnxiouslyOptimized/releases")
        .to_string();

    let setup_download_url = val
        .get("setup_url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("https://raw.githubusercontent.com/raghavatgit/AnxiouslyOptimized/main/dist/OptimizerByRaghav-Setup.exe")
        .to_string();

    let release_notes = val
        .get("notes")
        .and_then(|v| v.as_str())
        .unwrap_or("New version available on GitHub.")
        .trim()
        .to_string();

    let is_newer = compare_semver(&latest_version, APP_VERSION) == Ordering::Greater;

    Ok(UpdateCheckResult {
        current_version: APP_VERSION.to_string(),
        latest_version,
        is_newer,
        release_url,
        setup_download_url,
        release_notes,
    })
}

/// Downloads `setup_url` to `%TEMP%\OptimizerByRaghav-Setup.exe` and launches it elevated.
pub fn download_and_launch_update(setup_url: &str) -> Result<String, String> {
    let escaped = setup_url.replace('\'', "''");
    let script = format!(
        "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13; \
         $dst = Join-Path $env:TEMP 'OptimizerByRaghav-Update-Setup.exe'; \
         Invoke-WebRequest -Uri '{0}' -OutFile $dst -UseBasicParsing -TimeoutSec 60 -ErrorAction Stop; \
         if ((Test-Path $dst) -and ((Get-Item $dst).Length -gt 500000)) {{ \
             Start-Process -FilePath $dst -ArgumentList '--silent-launch' -Verb RunAs; \
             Write-Output $dst \
         }} else {{ \
             throw 'Downloaded update binary is incomplete or invalid.' \
         }}",
        escaped
    );

    let out = run_powershell_encoded(&script, 75_000)?;
    if out.exit_code == 0 {
        Ok(format!(
            "Downloaded & launched update installer ({})",
            out.stdout.trim()
        ))
    } else {
        Err(format!(
            "Update download failed: {}",
            out.stderr.trim()
        ))
    }
}

pub fn parse_semver_tuple(v: &str) -> (u32, u32, u32) {
    let clean = v
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .split(['-', '+'])
        .next()
        .unwrap_or("0.0.0");
    let mut parts = clean.split('.').map(|p| p.trim().parse::<u32>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

pub fn compare_semver(a: &str, b: &str) -> Ordering {
    parse_semver_tuple(a).cmp(&parse_semver_tuple(b))
}

#[allow(dead_code)]
fn _status_guard(s: &UpdateStatus) -> bool {
    matches!(s, UpdateStatus::Idle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison_tests() {
        assert_eq!(compare_semver("2.2.0", "2.1.0"), Ordering::Greater);
        assert_eq!(compare_semver("v2.1.1", "2.1.0"), Ordering::Greater);
        assert_eq!(compare_semver("2.1.0", "2.1.0"), Ordering::Equal);
        assert_eq!(compare_semver("2.0.9", "2.1.0"), Ordering::Less);
    }
}
