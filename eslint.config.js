import js from '@eslint/js'
import stylistic from '@stylistic/eslint-plugin'
import tailwindcss from 'eslint-plugin-better-tailwindcss'
import tseslint from 'typescript-eslint'
import svelte from 'eslint-plugin-svelte'
import globals from 'globals'
import svelteConfig from './packages/app/svelte.config.js'

export default tseslint.config(
  {
    ignores: [
      '**/node_modules/',
      '**/dist/',
      '**/build/',
      '**/.svelte-kit/',
      '**/src-tauri/target/',
      '**/src-tauri/gen/',
      // shadcn-svelte copy-ins: upstream code, updated by its CLI. `hooks` is a
      // second root because components.json maps that alias outside `ui/`, and
      // `sidebar` pulls `is-mobile` in through it.
      'packages/app/src/lib/components/ui/',
      'packages/app/src/lib/hooks/',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs.recommended,
  // Formatting is an ESLint concern here (no Prettier): `eslint --fix` is the formatter.
  stylistic.configs.customize({
    indent: 2,
    quotes: 'single',
    semi: false,
    commaDangle: 'always-multiline',
    braceStyle: '1tbs',
    arrowParens: true,
  }),
  {
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
    },
    rules: {
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      '@stylistic/max-len': ['warn', { code: 100, ignoreStrings: true, ignoreUrls: true, ignoreTemplateLiterals: true }],
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
        parser: tseslint.parser,
        svelteConfig,
      },
    },
    rules: {
      // Template indentation belongs to the Svelte plugin;
      // @stylistic/indent only understands the script block.
      '@stylistic/indent': 'off',
      'svelte/indent': ['error', { indent: 2 }],
    },
  },
  {
    files: ['packages/app/**/*.{svelte,ts,js}'],
    ...tailwindcss.configs.recommended,
    rules: {
      ...tailwindcss.configs.recommended.rules,
      'better-tailwindcss/enforce-consistent-line-wrapping': ['warn', {
        printWidth: 100, indent: 2, preferSingleLine: true,
      }],
    },
    settings: {
      'better-tailwindcss': {
        // Tailwind 4 config is CSS; the plugin reads theme tokens from here.
        entryPoint: 'packages/app/src/app.css',
        messageStyle: 'compact',
      },
    },
  },
  {
    files: ['packages/extension/**/*.ts'],
    languageOptions: { globals: { chrome: 'readonly' } },
  },
)
