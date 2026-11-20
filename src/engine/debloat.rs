use crate::engine::run_powershell_encoded;
use crate::models::BloatAppItem;
use std::collections::HashSet;

pub const PROTECTED_WHITELIST: &[&str] = &[
    "Microsoft.WindowsStore",
    "Microsoft.StorePurchaseApp",
    "Microsoft.DesktopAppInstaller",
    "Microsoft.VCLibs.140.00",
    "Microsoft.VCLibs.140.00.UWPDesktop",
    "Microsoft.UI.Xaml.2.7",
    "Microsoft.UI.Xaml.2.8",
    "Microsoft.NET.Native.Framework.2.2",
    "Microsoft.NET.Native.Runtime.2.2",
    "Microsoft.WindowsCalculator",
    "Microsoft.Windows.Photos",
    "Microsoft.WindowsTerminal",
    "Microsoft.SecHealthUI",
    "Microsoft.GamingServices",
    "Microsoft.XboxIdentityProvider",
    "Microsoft.ZuneMusic", // Windows 11 built-in Media Player!
    "Microsoft.Paint",
    "Microsoft.WindowsNotepad",
    "Microsoft.ScreenSketch",
];

struct BloatCatalogEntry {
    package_name: &'static str,
    display_name: &'static str,
    category: &'static str,
    description: &'static str,
}

const BLOAT_CATALOG: &[BloatCatalogEntry] = &[
    BloatCatalogEntry {
        package_name: "Microsoft.BingNews",
        display_name: "Microsoft News",
        category: "News & Feeds",
        description: "Background MSN news aggregator and live tile service.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.BingWeather",
        display_name: "MSN Weather",
        category: "News & Feeds",
        description: "Weather widget and background sync service.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.GetHelp",
        display_name: "Microsoft Get Help",
        category: "Support",
        description: "Automated Microsoft troubleshooting helper app.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.Getstarted",
        display_name: "Windows Tips",
        category: "Onboarding",
        description: "Windows Tips and Tour promotional application.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.MicrosoftSolitaireCollection",
        display_name: "Microsoft Solitaire Collection",
        category: "Preinstalled Games",
        description: "Preinstalled ad-supported Solitaire bundle.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.MixedReality.Portal",
        display_name: "Mixed Reality Portal",
        category: "VR / Legacy",
        description: "Deprecated Windows Mixed Reality headset companion.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.People",
        display_name: "Microsoft People",
        category: "Communication",
        description: "Legacy contacts integration app.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.SkypeApp",
        display_name: "Skype",
        category: "Communication",
        description: "Preinstalled consumer Skype UWP app.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.YourPhone",
        display_name: "Phone Link",
        category: "Background Sync",
        description: "Runs background Android/iOS mobile sync daemon.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.WindowsFeedbackHub",
        display_name: "Feedback Hub",
        category: "Telemetry",
        description: "Diagnostic feedback reporter and survey prompt app.",
    },
    BloatCatalogEntry {
        package_name: "Clipchamp.Clipchamp",
        display_name: "Microsoft Clipchamp",
        category: "Media",
        description: "Bundled web-wrapped video editor.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.549981C3F5F10",
        display_name: "Cortana App",
        category: "Legacy AI",
        description: "Retired Cortana UWP assistant app.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.OutlookForWindows",
        display_name: "Outlook (New)",
        category: "Productivity",
        description: "WebView-based replacement Mail/Calendar client.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.Todos",
        display_name: "Microsoft To Do",
        category: "Productivity",
        description: "Cloud task list app.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.PowerAutomateDesktop",
        display_name: "Power Automate Desktop",
        category: "Enterprise",
        description: "RPA desktop automation utility bundled with Win11.",
    },
    BloatCatalogEntry {
        package_name: "MicrosoftTeams",
        display_name: "Microsoft Teams (Legacy)",
        category: "Communication",
        description: "Legacy Win11 Chat/Teams consumer client.",
    },
    BloatCatalogEntry {
        package_name: "MSTeams",
        display_name: "Microsoft Teams (New)",
        category: "Communication",
        description: "Unified WebView2 Microsoft Teams client.",
    },
    BloatCatalogEntry {
        package_name: "SpotifyAB.SpotifyMusic",
        display_name: "Spotify (Store Stub)",
        category: "Third-Party Stub",
        description: "Microsoft Store bundled Spotify client.",
    },
    BloatCatalogEntry {
        package_name: "king.com.CandyCrushSaga",
        display_name: "Candy Crush Saga",
        category: "Sponsored Game",
        description: "Sponsored third-party game stub.",
    },
    BloatCatalogEntry {
        package_name: "BytedancePte.Ltd.TikTok",
        display_name: "TikTok",
        category: "Sponsored App",
        description: "Sponsored social media web wrapper.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.Microsoft3DViewer",
        display_name: "3D Viewer",
        category: "Legacy Media",
        description: "Legacy 3D model viewer.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.WindowsMaps",
        display_name: "Windows Maps",
        category: "Utilities",
        description: "Offline maps app and background update tasks.",
    },
    BloatCatalogEntry {
        package_name: "Microsoft.WindowsSoundRecorder",
        display_name: "Voice Recorder",
        category: "Utilities",
        description: "Basic audio recorder UWP app.",
    },
    BloatCatalogEntry {
        package_name: "MicrosoftCorporationII.QuickAssist",
        display_name: "Quick Assist",
        category: "Support",
        description: "Remote assistance viewer app.",
    },
];

pub fn is_protected_package(package_name: &str) -> bool {
    let lower = package_name.trim().to_ascii_lowercase();
    PROTECTED_WHITELIST
        .iter()
        .any(|p| p.eq_ignore_ascii_case(&lower) || lower.starts_with(&p.to_ascii_lowercase()))
}

pub fn filter_protected_bloat(items: Vec<BloatAppItem>) -> Vec<BloatAppItem> {
    items
        .into_iter()
        .filter(|i| !is_protected_package(&i.package_name))
        .collect()
}

pub fn scan_installed_bloat() -> Result<Vec<BloatAppItem>, String> {
    let script = "Get-AppxPackage -AllUsers | Select-Object Name, PackageFullName | ForEach-Object { \"$($_.Name)|$($_.PackageFullName)\" }";
    let out = run_powershell_encoded(script, 30_000)?;
    if out.exit_code != 0 {
        return Err(format!(
            "Get-AppxPackage failed (exit {}): {}",
            out.exit_code,
            out.stderr.trim()
        ));
    }

    let mut installed_map: Vec<(String, String)> = Vec::new();
    for line in out.stdout.lines() {
        let trimmed = line.trim();
        if let Some((name, full)) = trimmed.split_once('|') {
            installed_map.push((name.trim().to_string(), full.trim().to_string()));
        }
    }

    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for entry in BLOAT_CATALOG {
        if is_protected_package(entry.package_name) {
            continue;
        }
        if let Some((exact_name, full_name)) = installed_map
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(entry.package_name))
        {
            if seen.insert(exact_name.to_ascii_lowercase()) {
                result.push(BloatAppItem {
                    package_name: exact_name.clone(),
                    full_name: full_name.clone(),
                    display_name: entry.display_name.to_string(),
                    category: entry.category.to_string(),
                    description: entry.description.to_string(),
                    is_selected: false, // Never pre-check destructive removals!
                });
            }
        }
    }

    Ok(filter_protected_bloat(result))
}

pub fn remove_selected_bloat(items: &[BloatAppItem]) -> Vec<(String, Result<(), String>)> {
    let mut results = Vec::new();
    for item in items.iter().filter(|i| i.is_selected) {
        if is_protected_package(&item.package_name) {
            results.push((
                item.display_name.clone(),
                Err("Blocked: protected Windows component".to_string()),
            ));
            continue;
        }

        // Exact package Name & exact DisplayName match only — never substring `-like` wildcard!
        let escaped_name = item.package_name.replace('\'', "''");
        let script = format!(
            "$target = '{0}'; \
             Get-AppxPackage -AllUsers | Where-Object {{ $_.Name -eq $target }} | Remove-AppxPackage -AllUsers -ErrorAction SilentlyContinue; \
             Get-AppxProvisionedPackage -Online | Where-Object {{ $_.DisplayName -eq $target }} | Remove-AppxProvisionedPackage -Online -ErrorAction SilentlyContinue; \
             $still = Get-AppxPackage -AllUsers | Where-Object {{ $_.Name -eq $target }}; \
             if ($still) {{ Write-Output 'STILL_PRESENT' }} else {{ Write-Output 'REMOVED' }}",
            escaped_name
        );

        let outcome = match run_powershell_encoded(&script, 30_000) {
            Ok(out) => {
                if out.stdout.contains("REMOVED") {
                    Ok(())
                } else {
                    Err(format!(
                        "Package remained present (system-protected or in use): {}",
                        out.stderr.trim()
                    ))
                }
            }
            Err(e) => Err(e),
        };
        results.push((item.display_name.clone(), outcome));
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debloat_whitelist_enforcement_tests() {
        assert!(is_protected_package("Microsoft.WindowsStore"));
        assert!(is_protected_package("Microsoft.ZuneMusic"));
        assert!(is_protected_package("Microsoft.VCLibs.140.00.UWPDesktop"));
        assert!(!is_protected_package("Microsoft.BingNews"));

        let input = vec![
            BloatAppItem {
                package_name: "Microsoft.WindowsStore".into(),
                full_name: "x".into(),
                display_name: "Store".into(),
                category: "Sys".into(),
                description: "".into(),
                is_selected: true,
            },
            BloatAppItem {
                package_name: "Microsoft.BingNews".into(),
                full_name: "y".into(),
                display_name: "News".into(),
                category: "News".into(),
                description: "".into(),
                is_selected: false,
            },
        ];
        let filtered = filter_protected_bloat(input);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].package_name, "Microsoft.BingNews");
    }
}
