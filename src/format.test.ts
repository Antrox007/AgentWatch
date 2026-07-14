import { describe, expect, it } from "vitest";
import { fmtCost, fmtTokens, shortModel } from "./format";

describe("fmtTokens", () => {
  it("zeigt kleine Zahlen unveraendert", () => {
    expect(fmtTokens(0)).toBe("0");
    expect(fmtTokens(999)).toBe("999");
  });
  it("kuerzt Tausender mit K", () => {
    expect(fmtTokens(1000)).toBe("1K");
    expect(fmtTokens(47800)).toBe("47.8K");
    expect(fmtTokens(172400)).toBe("172.4K");
  });
  it("zeigt fuer null einen Strich", () => {
    expect(fmtTokens(null)).toBe("–");
  });
});

describe("fmtCost", () => {
  it("zeigt groessere Betraege mit zwei Nachkommastellen", () => {
    expect(fmtCost(12.5)).toBe("$12.50");
    expect(fmtCost(0.42)).toBe("$0.42");
  });
  it("zeigt Kleinbetraege mit vier Nachkommastellen", () => {
    expect(fmtCost(0.0042)).toBe("$0.0042");
  });
  it("zeigt null und 0 sauber an", () => {
    expect(fmtCost(null)).toBe("–");
    expect(fmtCost(0)).toBe("$0.00");
  });
});

describe("shortModel", () => {
  it("entfernt das claude-Praefix", () => {
    expect(shortModel("claude-opus-4-8")).toBe("opus-4-8");
    expect(shortModel("claude-sonnet-4-6")).toBe("sonnet-4-6");
  });
  it("entfernt ein Datums-Suffix", () => {
    expect(shortModel("claude-haiku-4-5-20251001")).toBe("haiku-4-5");
  });
  it("gibt fuer null einen leeren String zurueck", () => {
    expect(shortModel(null)).toBe("");
  });
});
