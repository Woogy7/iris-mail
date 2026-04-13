type CatppuccinFlavour = 'latte' | 'frappe' | 'macchiato' | 'mocha';

let currentTheme = $state<CatppuccinFlavour>('mocha');

function detectSystemTheme(): CatppuccinFlavour {
  if (typeof window === 'undefined') return 'mocha';
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'latte' : 'mocha';
}

function applyTheme(theme: CatppuccinFlavour) {
  if (typeof document === 'undefined') return;
  document.documentElement.setAttribute('data-theme', theme);
}

export function initTheme() {
  currentTheme = detectSystemTheme();
  applyTheme(currentTheme);

  if (typeof window !== 'undefined') {
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
      const autoTheme = e.matches ? 'mocha' : 'latte';
      currentTheme = autoTheme;
      applyTheme(autoTheme);
    });
  }
}

export function setTheme(theme: CatppuccinFlavour) {
  currentTheme = theme;
  applyTheme(theme);
}

export function getTheme(): CatppuccinFlavour {
  return currentTheme;
}
