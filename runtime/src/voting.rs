use frame_election_provider_support::private::sp_arithmetic::traits::{Bounded, CheckedAdd, CheckedDiv, CheckedMul};
use frame_support::dispatch::TypeInfo;
use pallet_democracy::{Conviction, Delegations, VoteWeight};
use sp_runtime::traits::{IntegerSquareRoot, Zero};

// Precompute exponent conviction values for faster lookup at runtime
// value = (conviction ^ 1.285) / (6 ^ 1.285)
const EXP_CONVICTION_FACTORS: [u32; 7] = [
    5_188_904,     // 0.005188904
    100_017_419,   // 0.100017419
    243_724_500,   // 0.243724500
    410_370_804,   // 0.410370804
    593_912_864,   // 0.593912864
    791_137_734,   // 0.791137734
    1_000_000_000, // 1.0
];

// To get around Substrate's annoying type restrictions, this helper can convert a u32 into
// B using just the allowed operations
fn from_u32<B>(n: u32) -> Option<B>
where
    B: From<u8> + CheckedMul + CheckedAdd + Zero + Copy,
{
    // Build B from decimal digits using only *10 and +digit.
    if n == 0 {
        return Some(B::zero());
    }
    let ten = B::from(10);
    let mut acc = B::zero();
    // Collect digits (little-endian)
    let mut tmp = n;
    let mut digits = [0u8; 10]; // u32 fits in <=10 decimal digits
    let mut len = 0usize;
    while tmp > 0 {
        digits[len] = (tmp % 10) as u8;
        tmp /= 10;
        len += 1;
    }
    // Rebuild big-endian: acc = acc*10 + digit
    while len > 0 {
        let d = digits[len - 1];
        acc = acc.checked_mul(&ten)?;
        acc = acc.checked_add(&B::from(d))?;
        len -= 1;
    }
    Some(acc)
}

/// A custom quadratic vote weight implementation for democracy pallet.
/// Follows the formula votes = capital * Sqrt(conviction / 6)
#[derive(TypeInfo, Default)]
pub struct QuadraticVoteWeight;

impl VoteWeight for QuadraticVoteWeight {
    fn votes<B: From<u8> + Zero + Copy + CheckedMul + CheckedDiv + CheckedAdd + PartialOrd + Bounded + IntegerSquareRoot>(
        conviction: Conviction,
        capital: B,
    ) -> Delegations<B> {
        // Account for zero separately
        if capital.is_zero() {
            return Delegations { votes: Zero::zero(), capital };
        }

        // Get sqrt(rep) factor from the precomputed list and multiply by capital
        let id = u8::from(conviction) as usize;
        let factor: u32 = EXP_CONVICTION_FACTORS.get(id).copied().unwrap_or(0);
        let factor = match from_u32::<B>(factor) {
            Some(v) => v,
            None => return Delegations { votes: B::zero(), capital },
        };
        let scale = match from_u32::<B>(1_000_000_000) {
            Some(v) => v,
            None => return Delegations { votes: B::zero(), capital },
        };
        let votes = capital.checked_mul(&factor).unwrap_or_else(B::max_value);
        let votes = votes.checked_div(&scale).unwrap_or_else(Zero::zero);

        Delegations { votes, capital }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic_vote_weight_x0() {
        let capital: u128 = 1_000_000;
        let conviction = Conviction::None;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 5_188);
    }

    #[test]
    fn test_quadratic_vote_weight_x1() {
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked1x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 100_017);
    }

    #[test]
    fn test_quadratic_vote_weight_x2() {
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked2x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 243_724);
    }

    #[test]
    fn test_quadratic_vote_weight_x3() {
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked3x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 410_370);
    }

    #[test]
    fn test_quadratic_vote_weight_x4() {
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked4x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 593_912);
    }

    #[test]
    fn test_quadratic_vote_weight_x5() {
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked5x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 791_137);
    }

    #[test]
    fn test_quadratic_vote_weight_x6() {
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked6x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 1_000_000);
    }

    #[test]
    fn test_quadratic_vote_weight_tiny_capital_x1() {
        // 10 at 1x conviction should yield 1 vote
        let capital: u128 = 10;
        let conviction = Conviction::Locked1x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 1);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_tiny_capital_none() {
        // 192 at 0.1x conviction should yield 0 votes
        let capital: u128 = 192;
        let conviction = Conviction::None;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 0);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_zero_capital() {
        // 0 * sqrt(1/6) = 0
        let capital: u128 = 0;
        let conviction = Conviction::Locked1x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 0);
        assert_eq!(result.capital, capital);
    }
}
