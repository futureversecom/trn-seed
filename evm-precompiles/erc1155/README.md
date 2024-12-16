# TRN ERC1155 Precompile supported interfaces

Precompile address spec: `0xBBBBBBBB[4-byte-collection-id]000000000000000000000000`

```solidity
interface IERC165 {
    function supportsInterface(bytes4 interfaceId) external view returns (bool);
}
```

```solidity
interface IERC1155 is IERC165 {
    event TransferSingle(address indexed operator, address indexed from, address indexed to, uint256 id, uint256 value);
    event TransferBatch(address indexed operator, address indexed from, address indexed to, uint256[] ids, uint256[] values);
    event ApprovalForAll(address indexed account, address indexed operator, bool approved);

    function balanceOf(address owner, uint256 id) external view returns (uint256);
    function balanceOfBatch(address[] calldata owners, uint256[] calldata ids) external view returns (uint256[] memory);
    function setApprovalForAll(address operator, bool approved) external;
    function isApprovedForAll(address account, address operator) external view returns (bool);
    function safeTransferFrom(address from, address to, uint256 id, uint256 amount, bytes calldata data) external;
    function safeBatchTransferFrom(address from, address to, uint256[] calldata ids, uint256[] calldata amounts, bytes calldata data) external;
}
```

```solidity
interface IERC1155Burnable is IERC1155 {
    function burn(address account, uint256 id, uint256 value) external;
    function burnBatch(address account, uint256[] calldata ids, uint256[] calldata values) external;
}
```

```solidity
interface IERC1155Supply is IERC1155 {
    function totalSupply(uint256 id) external view returns (uint256);
    function exists(uint256 id) external view returns (bool);
}
```

```solidity
interface IERC1155MetadataURI is IERC1155 {
    function uri(uint256 id) external view returns (string memory);
}
```

```solidity
interface TRN1155 is IERC165 {
    event TokenCreated(uint32 indexed serialNumber);
    event MaxSupplyUpdated(uint128 indexed maxSupply);
    event BaseURIUpdated(string baseURI);
    event PublicMintToggled(uint256 id, bool indexed enabled);
    event MintFeeUpdated(uint256 id, address indexed paymentAsset, uint256 indexed mintFee);

    function createToken(bytes calldata name, uint128 initialIssuance, uint128 maxIssuance, address tokenOwner) external returns (uint32);
    function mint(address owner, uint256 id, uint256 amount) external;
    function mintBatch(address owner, uint256[] calldata ids, uint256[] calldata amounts) external;
    function setMaxSupply(uint256 id, uint32 maxSupply) external;
    function setBaseURI(bytes calldata baseURI) external;
    function togglePublicMint(uint256 id, bool enabled) external;
    function setMintFee(uint256 id, address paymentAsset, uint256 mintFee) external;
}
```

```solidity
interface Ownable is IERC165 {
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    function owner() external view returns (address);
    function renounceOwnership() external;
    function transferOwnership(address owner) external;
}
```
