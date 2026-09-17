import { describe, expect, it } from 'vitest';
import { translations } from '@/translations';

// The panel falls back to English for a whole locale, not per key: a key
// missing from a locale that exists is shown to its users as the raw key.
type Key = keyof typeof translations.en;

function placeholders(text: string): string[] {
  return (text.match(/\{\w+\}|(?<!\w):[a-z]\w*/g) ?? []).sort();
}

describe('translations', () => {
  const englishKeys = Object.keys(translations.en).sort();

  for (const [locale, strings] of Object.entries(translations)) {
    it(`${locale} defines exactly the English keys`, () => {
      expect(Object.keys(strings).sort()).toEqual(englishKeys);
    });

    it(`${locale} keeps the English placeholders`, () => {
      for (const [key, text] of Object.entries(strings)) {
        expect(placeholders(text), key).toEqual(placeholders(translations.en[key as Key]));
      }
    });
  }
});
