using System;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace AnxiouslyOptimized.Services
{
    public static class ThemeManager
    {
        public static string CurrentTheme { get { return "OLED Black"; } }

        public static void LoadSavedTheme(MainWindow window)
        {
            ApplyOledTheme(window);
        }

        public static void SaveTheme(string themeName)
        {
            try
            {
                string dir = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "data");
                if (!Directory.Exists(dir))
                    Directory.CreateDirectory(dir);
                string file = Path.Combine(dir, "theme.json");
                File.WriteAllText(file, "{\"Theme\":\"OLED\"}");
            }
            catch { }
        }

        public static void ApplyOledTheme(MainWindow window)
        {
            if (window == null) return;

            var res = window.Resources;
            var appRes = Application.Current != null ? Application.Current.Resources : null;

            // Pure True OLED Black Window Base
            window.Background = CreateSolid("#000000");

            var bgOled = CreateSolid("#000000");
            var borderOled = CreateSolid("#18181B");

            SetRes(res, appRes, "WindowBackgroundBrush", bgOled);
            SetRes(res, appRes, "WindowBorderBrush", borderOled);

            if (window.RootBorder != null)
            {
                window.RootBorder.Background = bgOled;
                window.RootBorder.BorderBrush = borderOled;
            }

            if (window.BrdTitleBar != null)
            {
                window.BrdTitleBar.Background = CreateSolid("#050505");
                window.BrdTitleBar.BorderBrush = CreateSolid("#141418");
            }
            if (window.BrdSidebar != null)
            {
                window.BrdSidebar.Background = CreateSolid("#070709");
                window.BrdSidebar.BorderBrush = CreateSolid("#141418");
            }
            if (window.BrdSearchBox != null)
            {
                window.BrdSearchBox.Background = CreateSolid("#0C0C0F");
                window.BrdSearchBox.BorderBrush = CreateSolid("#22222A");
            }
            if (window.TxtGlobalSearch != null)
            {
                window.TxtGlobalSearch.Foreground = CreateSolid("#A1A1AA");
            }
            if (window.BtnOpenActivityLogs != null)
            {
                window.BtnOpenActivityLogs.Background = CreateSolid("#0F0F14");
                window.BtnOpenActivityLogs.BorderBrush = CreateSolid("#22222C");
            }
            if (window.BrdEditionBadge != null)
            {
                window.BrdEditionBadge.Background = CreateSolid("#0E121A");
                window.BrdEditionBadge.BorderBrush = CreateSolid("#1E2638");
            }
            if (window.TxtEditionBadge != null)
            {
                window.TxtEditionBadge.Foreground = CreateSolid("#38BDF8");
            }
            if (window.BrdOverviewDock != null)
            {
                window.BrdOverviewDock.Background = CreateSolid("#08080B");
                window.BrdOverviewDock.BorderBrush = CreateSolid("#1C1C22");
            }
            if (window.BrdOverviewDivider != null)
            {
                window.BrdOverviewDivider.Background = CreateSolid("#1C1C22");
            }
            if (window.PnlTelemetryCapsule != null)
            {
                window.PnlTelemetryCapsule.Background = CreateSolid("#0C0C10");
                window.PnlTelemetryCapsule.BorderBrush = CreateSolid("#202028");
            }
            if (window.BrdConsoleDrawer != null)
            {
                window.BrdConsoleDrawer.Background = CreateSolid("#050507");
                window.BrdConsoleDrawer.BorderBrush = CreateSolid("#1C1C24");
            }

            // High-Performance Dynamic Solid Tokens (NO EXTRA GRADIENTS)
            SetRes(res, appRes, "OverviewCardBg", CreateSolid("#0A0A0D"));
            SetRes(res, appRes, "OverviewCardBorder", CreateSolid("#1C1C22"));
            SetRes(res, appRes, "OverviewCardHoverBg", CreateSolid("#121216"));
            SetRes(res, appRes, "OverviewCardHoverBorder", CreateSolid("#2A2A35"));

            SetRes(res, appRes, "CardActionBtnBg", CreateSolid("#14141A"));
            SetRes(res, appRes, "CardActionBtnBorder", CreateSolid("#262632"));
            SetRes(res, appRes, "CardActionBtnHoverBg", CreateSolid("#1C1C24"));
            SetRes(res, appRes, "CardActionBtnHoverBorder", CreateSolid("#00F5D4"));

            SetRes(res, appRes, "TextPrimaryBrush", CreateSolid("#FFFFFF"));
            SetRes(res, appRes, "TextSecondaryBrush", CreateSolid("#A1A1AA"));
            SetRes(res, appRes, "TextMutedBrush", CreateSolid("#71717A"));

            SetRes(res, appRes, "SidebarActiveBgBrush", CreateSolid("#14141A"));
            SetRes(res, appRes, "SidebarActiveFgBrush", CreateSolid("#FFFFFF"));
            SetRes(res, appRes, "SidebarCardBgBrush", CreateSolid("#0A0A0D"));
            SetRes(res, appRes, "SidebarCardBorderBrush", CreateSolid("#1C1C22"));
            SetRes(res, appRes, "SidebarHoverBgBrush", CreateSolid("#0E0E12"));

            SetRes(res, appRes, "SectionBannerBg", CreateSolid("#0A0A0D"));
            SetRes(res, appRes, "SectionBannerBorder", CreateSolid("#1C1C22"));
            SetRes(res, appRes, "SectionBannerTitleBrush", CreateSolid("#00F5D4"));
            SetRes(res, appRes, "AccentBrush", CreateSolid("#00F5D4"));

            SetRes(res, appRes, "TweakCardBg", CreateSolid("#09090C"));
            SetRes(res, appRes, "TweakCardBorder", CreateSolid("#18181E"));
            SetRes(res, appRes, "TweakCardHoverBg", CreateSolid("#111116"));

            SetRes(res, appRes, "BadgeOptimizedBg", CreateSolid("#061A14"));
            SetRes(res, appRes, "BadgeOptimizedBorder", CreateSolid("#0F5132"));
            SetRes(res, appRes, "BadgeOptimizedFg", CreateSolid("#10B981"));

            SetRes(res, appRes, "BadgeStandardBg", CreateSolid("#121216"));
            SetRes(res, appRes, "BadgeStandardBorder", CreateSolid("#22222A"));
            SetRes(res, appRes, "BadgeStandardFg", CreateSolid("#71717A"));

            SetRes(res, appRes, "InputSearchBg", CreateSolid("#0A0A0D"));
            SetRes(res, appRes, "InputSearchBorder", CreateSolid("#1C1C22"));
            SetRes(res, appRes, "InputSearchFg", CreateSolid("#FFFFFF"));

            SetRes(res, appRes, "CircleNavBgBrush", CreateSolid("#14141A"));
            SetRes(res, appRes, "CircleNavBorderBrush", CreateSolid("#262632"));
        }

        private static void SetRes(ResourceDictionary res, ResourceDictionary appRes, string key, object value)
        {
            if (res != null) res[key] = value;
            if (appRes != null) appRes[key] = value;
        }

        private static SolidColorBrush CreateSolid(string hex)
        {
            var brush = new SolidColorBrush((Color)ColorConverter.ConvertFromString(hex));
            brush.Freeze();
            return brush;
        }
    }
}
