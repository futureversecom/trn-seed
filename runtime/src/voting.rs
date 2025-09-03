use frame_election_provider_support::private::sp_arithmetic::traits::{Bounded, CheckedAdd, CheckedDiv, CheckedMul};
use frame_support::dispatch::TypeInfo;
use pallet_democracy::{Conviction, Delegations, VoteWeight};
use sp_runtime::traits::{IntegerSquareRoot, Zero};

// Precompute square root conviction factors for faster lookup at runtime
// Factors: sqrt({0.1,1,2,3,4,5,6} / 6)
const SQRT_CONVICTION_FACTORS: [u32; 7] = [
    129_099_445, // sqrt(0.1/6) ≈ 0.1290994449
    408_248_290, // sqrt(1/6)   ≈ 0.4082482905
    577_350_269, // sqrt(2/6)   ≈ 0.5773502692
    707_106_781, // sqrt(3/6)   ≈ 0.7071067812
    816_496_581, // sqrt(4/6)   ≈ 0.8164965809
    912_870_929, // sqrt(5/6)   ≈ 0.9128709292
    1_000_000_000, // sqrt(6/6) = 1.0
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
        let factor: u32 = SQRT_CONVICTION_FACTORS.get(id).copied().unwrap_or(0);
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
        // 1_000_000 * sqrt(0.1/6) = 129_099
        let capital: u128 = 1_000_000;
        let conviction = Conviction::None;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 129_099);
    }

    #[test]
    fn test_quadratic_vote_weight_x1() {
        // 1_000_000 * sqrt(1/6) = 408_248
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked1x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 408_248);
    }

    #[test]
    fn test_quadratic_vote_weight_x2() {
        // 1_000_000 * sqrt(2/6) = 577_350
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked2x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 577_350);
    }

    #[test]
    fn test_quadratic_vote_weight_x3() {
        // 1_000_000 * sqrt(3/6) = 707_106
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked3x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 707_106);
    }

    #[test]
    fn test_quadratic_vote_weight_x4() {
        // 1_000_000 * sqrt(4/6) = 816_496
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked4x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 816_496);
    }

    #[test]
    fn test_quadratic_vote_weight_x5() {
        // 1_000_000 * sqrt(5/6) = 912_870
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked5x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 912_870);
    }

    #[test]
    fn test_quadratic_vote_weight_x6() {
        // 1_000_000 * sqrt(6/6) = 1_000_000
        let capital: u128 = 1_000_000;
        let conviction = Conviction::Locked6x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.capital, capital);
        assert_eq!(result.votes, 1_000_000);
    }

    #[test]
    fn test_quadratic_vote_weight_tiny_capital_x1() {
        // 3 at 1x conviction should yield 1 vote
        // 3 * sqrt(1/6) = 1
        let capital: u128 = 3;
        let conviction = Conviction::Locked1x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 1);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_tiny_capital_none() {
        // 1 at 1x conviction should yield 0 votes
        // 7 * sqrt(0.1/6)
        let capital: u128 = 7;
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
