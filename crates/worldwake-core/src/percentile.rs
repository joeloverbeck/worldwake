use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PercentileBucket {
    pub n: u64,
    pub min: u64,
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
    pub max: u64,
    pub mean: u64,
}

impl PercentileBucket {
    pub fn from_sorted(values: &[u64]) -> Self {
        if values.is_empty() {
            return Self {
                n: 0,
                min: 0,
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                mean: 0,
            };
        }

        let n = values.len() as u64;
        let sum: u128 = values.iter().map(|value| u128::from(*value)).sum();

        Self {
            n,
            min: values[0],
            p50: percentile(values, 50),
            p95: percentile(values, 95),
            p99: percentile(values, 99),
            max: values[values.len() - 1],
            mean: (sum / u128::from(n)) as u64,
        }
    }
}

fn percentile(values: &[u64], percentile: usize) -> u64 {
    debug_assert!(!values.is_empty());
    debug_assert!(percentile <= 100);

    let max_index = values.len() - 1;
    let index = max_index * percentile / 100;
    values[index]
}

#[cfg(test)]
mod tests {
    use super::PercentileBucket;

    #[test]
    fn from_sorted_computes_integer_percentiles() {
        let values: Vec<u64> = (1..=100).collect();

        let bucket = PercentileBucket::from_sorted(&values);

        assert_eq!(
            bucket,
            PercentileBucket {
                n: 100,
                min: 1,
                p50: 50,
                p95: 95,
                p99: 99,
                max: 100,
                mean: 50,
            }
        );
    }

    #[test]
    fn from_sorted_empty_slice_returns_zero_bucket() {
        let bucket = PercentileBucket::from_sorted(&[]);

        assert_eq!(
            bucket,
            PercentileBucket {
                n: 0,
                min: 0,
                p50: 0,
                p95: 0,
                p99: 0,
                max: 0,
                mean: 0,
            }
        );
    }

    #[test]
    fn from_sorted_is_deterministic() {
        let values = [0, 5, 5, 10, 20, 40, 80];

        let first = PercentileBucket::from_sorted(&values);
        let second = PercentileBucket::from_sorted(&values);

        assert_eq!(first, second);
        assert_eq!(
            bincode::serialize(&first).expect("serialize first bucket"),
            bincode::serialize(&second).expect("serialize second bucket")
        );
    }
}
