# TRN ERC721 Precompile supported interfaces

Precompile address: `0x0000000000000000000000000000000000000793`

```solidity
interface TRNPeg is IERC165 {
    event Erc20Withdrawal(uint64 indexed eventProofId, address indexed beneficiary, address indexed tokenAddress, uint128 balance);
    event Erc721Withdrawal(uint64 indexed eventProofId, address indexed beneficiary, address indexed tokenAddress, uint32[] serialNumbers);
    
    function erc20Withdraw(address beneficiary, address asset, uint128 balance) returns (uint64);
    function erc721Withdraw(address beneficiary, address[] tokenAddresses, uint32[][] serialNumbers) returns (uint64);
}
```
