export type ThemePreference = "follow" | "light" | "dark";
export type EffectiveTheme = "light" | "dark";

export const THEME_STORAGE_KEY = "spotrak.theme";

const THEME_COLORS: Record<EffectiveTheme, string> = {
  light: "oklch(0.94 0.01 85)",
  dark: "oklch(0.145 0.012 255)",
};

export function isThemePreference(value: unknown): value is ThemePreference {
  return value === "follow" || value === "light" || value === "dark";
}

export function getStoredThemePreference(): ThemePreference {
  if (typeof window === "undefined") return "follow";
  const value = window.localStorage.getItem(THEME_STORAGE_KEY);
  return isThemePreference(value) ? value : "follow";
}

export function effectiveTheme(
  preference: ThemePreference = getStoredThemePreference(),
): EffectiveTheme {
  if (preference === "light" || preference === "dark") return preference;
  if (typeof window === "undefined") return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function applyThemePreference(
  preference: ThemePreference = getStoredThemePreference(),
): EffectiveTheme {
  const theme = effectiveTheme(preference);
  if (typeof document === "undefined") return theme;

  const root = document.documentElement;
  root.dataset.theme = theme;
  root.dataset.themePreference = preference;
  root.style.colorScheme = theme;

  const meta = document.querySelector<HTMLMetaElement>(
    'meta[name="theme-color"]',
  );
  meta?.setAttribute("content", THEME_COLORS[theme]);

  if (typeof window !== "undefined") {
    window.dispatchEvent(
      new CustomEvent("spotrak:theme-change", {
        detail: { preference, theme },
      }),
    );
  }

  return theme;
}

export function setThemePreference(
  preference: ThemePreference,
): EffectiveTheme {
  if (typeof window !== "undefined") {
    if (preference === "follow") {
      window.localStorage.removeItem(THEME_STORAGE_KEY);
    } else {
      window.localStorage.setItem(THEME_STORAGE_KEY, preference);
    }
  }
  return applyThemePreference(preference);
}

export function watchSystemTheme(): () => void {
  if (typeof window === "undefined") return () => undefined;

  const media = window.matchMedia("(prefers-color-scheme: dark)");
  const listener = () => {
    if (getStoredThemePreference() === "follow") applyThemePreference("follow");
  };

  media.addEventListener("change", listener);
  return () => media.removeEventListener("change", listener);
}
