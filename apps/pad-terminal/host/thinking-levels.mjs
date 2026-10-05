// Pi selector order; provider wire values (none/low/etc.) are not selector values.
export const THINKING_LEVELS = Object.freeze(['off', 'minimal', 'low', 'medium', 'high', 'xhigh', 'max']);
export const isThinkingLevel = (level) => THINKING_LEVELS.includes(level);

// Matches Pi 1.0.2 clampThinkingLevel: prefer upward, then downward.
export function clampToThinkingLevels(level, supported) {
  if (supported.includes(level)) return level;
  const index = THINKING_LEVELS.indexOf(level);
  if (index < 0) return supported[0] ?? 'off';
  return THINKING_LEVELS.slice(index).find((candidate) => supported.includes(candidate))
    ?? THINKING_LEVELS.slice(0, index).reverse().find((candidate) => supported.includes(candidate))
    ?? supported[0] ?? 'off';
}
