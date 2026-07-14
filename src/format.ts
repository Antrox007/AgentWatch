// Reine Formatierungs-Helfer (testbar, ohne React/Tauri).

/** Tokens kompakt: 47800 -> "47.8K", 999 -> "999", null -> "–". */
export function fmtTokens(n: number | null): string {
  if (n == null) return "–";
  if (n >= 1000) return (n / 1000).toFixed(1).replace(/\.0$/, "") + "K";
  return String(n);
}

/**
 * Kumulative Session-Kosten in USD. Kleinbetraege bekommen mehr Nachkommastellen,
 * damit sie nicht auf "$0.00" gerundet werden: >=1 -> 2, >=0.01 -> 2, sonst 4.
 */
export function fmtCost(usd: number | null): string {
  if (usd == null) return "–";
  if (usd >= 0.01) return "$" + usd.toFixed(2);
  if (usd > 0) return "$" + usd.toFixed(4);
  return "$0.00";
}

/** Modellname kuerzen: "claude-opus-4-8" -> "opus-4-8", Datum-Suffix entfernen. */
export function shortModel(model: string | null): string {
  if (!model) return "";
  return model.replace(/^claude-/, "").replace(/-\d{8}.*$/, "");
}
