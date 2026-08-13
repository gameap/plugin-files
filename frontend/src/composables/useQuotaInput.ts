import { ref, computed, watch, type Ref } from 'vue';

// Size unit multipliers (binary - 1024-based)
const SIZE_UNITS: Record<string, number> = {
  'B': 1,
  'K': 1024,
  'KB': 1024,
  'M': 1024 ** 2,
  'MB': 1024 ** 2,
  'G': 1024 ** 3,
  'GB': 1024 ** 3,
  'T': 1024 ** 4,
  'TB': 1024 ** 4,
};

// Regex pattern: optional whitespace, number (int or float), optional whitespace, optional unit
const SIZE_PATTERN = /^\s*(\d+(?:\.\d+)?)\s*([BKMGT]B?|[bkmgt]b?)?\s*$/;

/**
 * Parse a human-readable size string to bytes
 * @param str - Input like "10MB", "1.5 GB", "500K", "0", ""
 * @returns bytes as number, or null if invalid format
 */
export function parseSize(str: string): number | null {
  const trimmed = str.trim();

  // Empty or "0" means unlimited (0 bytes)
  if (trimmed === '' || trimmed === '0') {
    return 0;
  }

  const match = trimmed.match(SIZE_PATTERN);
  if (!match) {
    return null;
  }

  const [, numStr, unit] = match;
  const num = parseFloat(numStr);

  if (isNaN(num) || num < 0) {
    return null;
  }

  // Default to bytes if no unit specified
  const unitUpper = (unit || 'B').toUpperCase();
  const multiplier = SIZE_UNITS[unitUpper];

  if (multiplier === undefined) {
    return null;
  }

  return Math.round(num * multiplier);
}

/**
 * Format bytes to human-readable string
 * @param bytes - Number of bytes
 * @returns Formatted string like "10 GB" or "" for unlimited
 */
export function formatSize(bytes: number): string {
  if (bytes === 0) {
    return '';
  }

  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let unitIndex = 0;
  let size = bytes;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  // Use integer if it's a whole number, otherwise 1 decimal
  const formatted = size % 1 === 0 ? size.toString() : size.toFixed(1);
  return `${formatted} ${units[unitIndex]}`;
}

/**
 * Composable for quota input with human-readable format
 * @param initialBytes - Initial value in bytes (as Ref or number)
 * @returns Reactive state and methods for quota input
 */
export function useQuotaInput(initialBytes: Ref<number> | number = 0) {
  const initialValue = typeof initialBytes === 'number' ? initialBytes : initialBytes.value;

  // The raw text input value
  const inputValue = ref(formatSize(initialValue));

  // Parsed bytes value (null if invalid)
  const parsedBytes = ref<number | null>(initialValue);

  // Validation error message (translation key)
  const errorMessage = ref<string | null>(null);

  // Whether the current input is valid
  const isValid = computed(() => parsedBytes.value !== null);

  // The bytes value to use (0 if invalid or empty)
  const bytesValue = computed(() => parsedBytes.value ?? 0);

  /**
   * Update the input value and parse it
   */
  function updateInput(value: string | number) {
    const strValue = String(value);
    inputValue.value = strValue;
    const parsed = parseSize(strValue);

    if (parsed === null) {
      parsedBytes.value = null;
      errorMessage.value = 'error_quota_format';
    } else {
      parsedBytes.value = parsed;
      errorMessage.value = null;
    }
  }

  /**
   * Set the bytes value directly (e.g., when loading from API)
   */
  function setBytes(bytes: number) {
    parsedBytes.value = bytes;
    inputValue.value = formatSize(bytes);
    errorMessage.value = null;
  }

  // Watch for external changes if initialBytes is a Ref
  if (typeof initialBytes !== 'number') {
    watch(initialBytes, (newBytes) => {
      setBytes(newBytes);
    });
  }

  return {
    inputValue,
    parsedBytes,
    errorMessage,
    isValid,
    bytesValue,
    updateInput,
    setBytes,
  };
}
