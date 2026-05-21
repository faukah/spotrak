export type ThemePreference = "follow" | "light" | "dark";
export type EffectiveTheme = "light" | "dark";
export type ThemeTransitionOrigin = { x: number; y: number };

type ViewTransition = { finished: Promise<void> };
type ViewTransitionDocument = Document & {
  startViewTransition(callback: () => void): ViewTransition;
};

export const THEME_STORAGE_KEY = "spotrak.theme";

const THEME_COLORS: Record<EffectiveTheme, string> = {
  light: "oklch(0.94 0.01 85)",
  dark: "oklch(0.145 0.012 255)",
};
const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

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
  transitionOrigin?: ThemeTransitionOrigin,
): EffectiveTheme {
  const theme = effectiveTheme(preference);
  if (typeof document === "undefined") return theme;

  const commit = () => applyThemeToDocument(preference, theme);
  if (shouldUseThemeTransition(theme, transitionOrigin)) {
    prepareThemeTransition(transitionOrigin);
    const transition = document.startViewTransition(commit);
    void transition.finished.then(clearThemeTransition, clearThemeTransition);
  } else {
    commit();
  }

  return theme;
}

export function setThemePreference(
  preference: ThemePreference,
  transitionOrigin?: ThemeTransitionOrigin,
): EffectiveTheme {
  if (typeof window !== "undefined") {
    if (preference === "follow") {
      window.localStorage.removeItem(THEME_STORAGE_KEY);
    } else {
      window.localStorage.setItem(THEME_STORAGE_KEY, preference);
    }
  }
  return applyThemePreference(preference, transitionOrigin);
}

function applyThemeToDocument(
  preference: ThemePreference,
  theme: EffectiveTheme,
): void {
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
}

function shouldUseThemeTransition(
  theme: EffectiveTheme,
  transitionOrigin: ThemeTransitionOrigin | undefined,
): transitionOrigin is ThemeTransitionOrigin {
  if (!transitionOrigin || typeof window === "undefined") return false;
  if (!supportsViewTransitions(document)) return false;
  if (window.matchMedia(REDUCED_MOTION_QUERY).matches) return false;
  return document.documentElement.dataset.theme !== theme;
}

function supportsViewTransitions(
  targetDocument: Document,
): targetDocument is ViewTransitionDocument {
  return "startViewTransition" in targetDocument;
}

function prepareThemeTransition(origin: ThemeTransitionOrigin): void {
  const root = document.documentElement;
  const maxX = Math.max(origin.x, window.innerWidth - origin.x);
  const maxY = Math.max(origin.y, window.innerHeight - origin.y);
  const radius = Math.hypot(maxX, maxY);

  root.dataset.themeTransition = "active";
  root.style.setProperty("--theme-transition-x", `${origin.x}px`);
  root.style.setProperty("--theme-transition-y", `${origin.y}px`);
  root.style.setProperty("--theme-transition-radius", `${radius}px`);
}

function clearThemeTransition(): void {
  const root = document.documentElement;
  delete root.dataset.themeTransition;
  root.style.removeProperty("--theme-transition-x");
  root.style.removeProperty("--theme-transition-y");
  root.style.removeProperty("--theme-transition-radius");
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
