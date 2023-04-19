# TRN ERC721 Precompile supported interfaces

```solidity
interface TRNPeg is IERC165 {
    event Erc20Withdrawal(address indexed beneficiary, address tokenAddress, uint128 balance);
    event Erc721Withdrawal(address indexed beneficiary, address[] tokenAddresses, uint32[][] serialNumbers);
    function Erc20Withdraw(address beneficiary, address asset, uint128 balance) returns (uint64);
    function Erc721Withdraw(address beneficiary, address[] tokenAddresses, uint32[][] serialNumbers) returns (uint64);
}
```
