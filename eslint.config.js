import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';
import globals from 'globals';

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'src-tauri', 'src/lib/types.gen.ts'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: { globals: globals.browser },
    plugins: { 'react-hooks': reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,

      // `any` is banned — docs/engineering/coding-standards.md. Use `unknown` and narrow.
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/consistent-type-imports': 'error',

      // Invariant 8: the frontend is presentation only, and every `invoke` goes
      // through src/lib/ipc.ts. Importing the Tauri API anywhere else is the
      // start of that boundary rotting — see docs/engineering/project-structure.md.
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: '@tauri-apps/api/core',
              message: 'Call Tauri commands through src/lib/ipc.ts only (invariant 8).',
            },
            {
              name: '@tauri-apps/api/event',
              message: 'Subscribe to core events through src/lib/ipc.ts only (invariant 8).',
            },
          ],
        },
      ],
    },
  },
  {
    // ipc.ts is the one door; it is allowed to import the Tauri API.
    files: ['src/lib/ipc.ts'],
    rules: { 'no-restricted-imports': 'off' },
  },
  {
    files: ['**/*.test.ts'],
    languageOptions: { globals: { ...globals.node } },
  },
);
