use frame_election_provider_support::private::sp_arithmetic::traits::{Bounded, CheckedAdd, CheckedDiv, CheckedMul};
use frame_support::dispatch::TypeInfo;
use pallet_democracy::{Conviction, Delegations};
use sp_runtime::traits::Zero;
use pallet_democracy::VoteWeight;

/// A custom quadratic vote weight implementation for democracy pallet.
/// Follows the formula votes = Sqrt(capital) * conviction
/// This ensures that the voting power grows quadratically with the capital invested
/// and allows for a fairer voting system where larger investments do not disproportionately skew the results.
/// Locked votes are applied AFTER the quadratic calculation, meaning that the conviction multiplier is applied to the final vote count.
#[derive(TypeInfo, Default)]
pub struct QuadraticVoteWeight;

impl VoteWeight for QuadraticVoteWeight {
    fn votes<B: From<u8> + Zero + Copy + CheckedMul + CheckedDiv + CheckedAdd + PartialOrd + Bounded>(
        conviction: Conviction,
        capital: B,
    ) -> Delegations<B> {
        // Account for zero separately
        if capital.is_zero() {
            return Delegations { votes: Zero::zero(), capital };
        }

        // Use Newton's method to approximate the square root of capital
        // This is both more efficient and works with the annoying generic type B
        let one: B = 1u8.into();
        let mut x = capital;
        let mut y = x
            .checked_add(&one)
            .and_then(|sum| sum.checked_div(&one.checked_add(&one).unwrap()))
            .unwrap_or(x);

        while y < x {
            x = y;
            y = x
                .checked_add(&capital.checked_div(&x).unwrap_or(x))
                .and_then(|sum| sum.checked_div(&one.checked_add(&one).unwrap()))
                .unwrap_or(x);
        }
        let q_cap = x;
        let votes = match conviction {
            Conviction::None => q_cap.checked_div(&10u8.into()).unwrap_or_else(Zero::zero),
            x => q_cap.checked_mul(&u8::from(x).into()).unwrap_or_else(B::max_value),
        };
        Delegations { votes, capital }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic_vote_weight_64x1() {
        // 64 at locked 1x should yield 8 votes
        // sqrt(64) * 1 = 8
        let capital: u128 = 64;
        let conviction = Conviction::Locked1x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 8);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_64x2() {
        // 64 at locked 2x should yield 16 votes
        // sqrt(64) * 2 = 16
        let capital: u128 = 64;
        let conviction = Conviction::Locked2x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 16);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_64x3() {
        // 64 at locked 3x should yield 24 votes
        // sqrt(64) * 3 = 24
        let capital: u128 = 64;
        let conviction = Conviction::Locked3x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 24);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_64x4() {
        // 64 at locked 4x should yield 32 votes
        // sqrt(64) * 4 = 32
        let capital: u128 = 64;
        let conviction = Conviction::Locked4x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 32);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_64x5() {
        // 64 at locked 5x should yield 40 votes
        // sqrt(64) * 5 = 40
        let capital: u128 = 64;
        let conviction = Conviction::Locked5x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 40);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_64x6() {
        // 64 at locked 6x should yield 48 votes
        // sqrt(64) * 6 = 48
        let capital: u128 = 64;
        let conviction = Conviction::Locked6x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 48);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_10000x1() {
        // 10000 at locked 1x should yield 100 votes
        // sqrt(10000) * 1 = 100
        let capital: u128 = 10000;
        let conviction = Conviction::Locked1x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 100);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_no_conviction() {
        // 10000 at no conviction should yield 10 votes
        // sqrt(10000) * 0.1 = 10
        let capital: u128 = 10000;
        let conviction = Conviction::None;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 10);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_tiny_capital_x1() {
        // 1 at 1x conviction should yield 1 vote
        // sqrt(1) * 1 = 1
        let capital: u128 = 1;
        let conviction = Conviction::Locked1x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 1);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_tiny_capital_none() {
        // 1 at 1x conviction should yield 0 votes
        // sqrt(1) * 0.1 = 0
        let capital: u128 = 1;
        let conviction = Conviction::None;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 0);
        assert_eq!(result.capital, capital);
    }

    #[test]
    fn test_quadratic_vote_weight_zero_capital() {
        // 0 at 1x conviction should yield 0 votes
        // sqrt(0) * 1 = 0
        let capital: u128 = 0;
        let conviction = Conviction::Locked1x;
        let result = QuadraticVoteWeight::votes(conviction, capital);
        assert_eq!(result.votes, 0);
        assert_eq!(result.capital, capital);
    }
}
