//! USD-Kostenschaetzung pro Session anhand des offiziellen Anthropic-Preisblatts.
//! Jeder Token-Eimer (Input, Output, Cache-Read, Cache-Write) wird mit seinem
//! eigenen Preis abgerechnet; die Session-Kosten sind die Summe ueber alle
//! Assistant-Turns (jeweils mit dem dort verwendeten Modell). Bewusst nur
//! informativ — der User ist auf Subscription, exakte Abrechnung erfolgt nicht hier.

/// Preise in USD pro 1 Mio Tokens (offizielle Anthropic-Listenpreise).
struct ModelPricing {
    input: f64,
    output: f64,
}

/// Cache-Multiplikatoren relativ zum Input-Preis (Anthropic-weit konstant).
const CACHE_READ_MULT: f64 = 0.1; // Cache-Read ~ 0,1x Input
const CACHE_WRITE_5M_MULT: f64 = 1.25; // Cache-Write, 5-Min-TTL = 1,25x Input
const CACHE_WRITE_1H_MULT: f64 = 2.0; // Cache-Write, 1h-TTL = 2x Input

/// Listenpreise je Modellfamilie. Modellnamen werden per Substring erkannt;
/// die Reihenfolge ist wichtig (z.B. "fable" vor allgemeinen Faellen).
fn pricing_for(model: &str) -> ModelPricing {
    let m = model.to_lowercase();
    if m.contains("fable") {
        ModelPricing {
            input: 10.0,
            output: 50.0,
        }
    } else if m.contains("opus") {
        ModelPricing {
            input: 5.0,
            output: 25.0,
        }
    } else if m.contains("haiku") {
        ModelPricing {
            input: 1.0,
            output: 5.0,
        }
    } else if m.contains("sonnet") {
        ModelPricing {
            input: 3.0,
            output: 15.0,
        }
    } else {
        // Unbekanntes Modell: mittleres (Sonnet-)Tier als Naeherung.
        ModelPricing {
            input: 3.0,
            output: 15.0,
        }
    }
}

/// Token-Verbrauch eines einzelnen Assistant-Turns, aufgeteilt nach Preis-Eimer.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write_5m: u64,
    pub cache_write_1h: u64,
}

/// Kosten eines einzelnen Turns in USD, jeder Eimer mit seinem eigenen Preis.
pub fn turn_cost_usd(model: &str, usage: &TokenUsage) -> f64 {
    let p = pricing_for(model);
    let per = |tokens: u64, price_per_mtok: f64| tokens as f64 / 1_000_000.0 * price_per_mtok;
    per(usage.input, p.input)
        + per(usage.output, p.output)
        + per(usage.cache_read, p.input * CACHE_READ_MULT)
        + per(usage.cache_write_5m, p.input * CACHE_WRITE_5M_MULT)
        + per(usage.cache_write_1h, p.input * CACHE_WRITE_1H_MULT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_and_output_priced_separately() {
        // 1 Mio Input-Tokens Opus = 5 USD, 1 Mio Output = 25 USD.
        let only_in = TokenUsage {
            input: 1_000_000,
            ..Default::default()
        };
        let only_out = TokenUsage {
            output: 1_000_000,
            ..Default::default()
        };
        assert!((turn_cost_usd("claude-opus-4-8", &only_in) - 5.0).abs() < 1e-9);
        assert!((turn_cost_usd("claude-opus-4-8", &only_out) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn cache_buckets_use_input_multipliers() {
        // Opus Input = 5 USD/Mtok -> Read 0,5 / Write-5m 6,25 / Write-1h 10 pro Mtok.
        let read = TokenUsage {
            cache_read: 1_000_000,
            ..Default::default()
        };
        let write_5m = TokenUsage {
            cache_write_5m: 1_000_000,
            ..Default::default()
        };
        let write_1h = TokenUsage {
            cache_write_1h: 1_000_000,
            ..Default::default()
        };
        assert!((turn_cost_usd("claude-opus-4-8", &read) - 0.5).abs() < 1e-9);
        assert!((turn_cost_usd("claude-opus-4-8", &write_5m) - 6.25).abs() < 1e-9);
        assert!((turn_cost_usd("claude-opus-4-8", &write_1h) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn model_tiers_ordered_by_price() {
        let u = TokenUsage {
            input: 100_000,
            ..Default::default()
        };
        let haiku = turn_cost_usd("claude-haiku-4-5", &u);
        let sonnet = turn_cost_usd("claude-sonnet-4-6", &u);
        let opus = turn_cost_usd("claude-opus-4-8", &u);
        let fable = turn_cost_usd("claude-fable-5", &u);
        assert!(haiku < sonnet && sonnet < opus && opus < fable);
    }

    #[test]
    fn empty_usage_is_free() {
        assert_eq!(turn_cost_usd("claude-opus-4-8", &TokenUsage::default()), 0.0);
    }
}
