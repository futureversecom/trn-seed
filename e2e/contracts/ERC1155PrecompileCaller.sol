// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.17;
import "@openzeppelin/contracts/interfaces/IERC1155.sol";
import "@openzeppelin/contracts/token/ERC1155/extensions/IERC1155MetadataURI.sol";

contract ERC1155PrecompileCaller {
    address public precompile;

    constructor(address _precompile) {
        precompile = _precompile;
    }

    function balanceOfProxy(address who, uint256 id) public view returns (uint256) {
        return IERC1155(precompile).balanceOf(who, id);
    }

    function balanceOfBatchProxy(address[] memory owners, uint256[] memory ids) public view returns (uint256[] memory) {
        return IERC1155(precompile).balanceOfBatch(owners, ids);
    }

    function setApprovalForAllProxy(address operator, bool approved) external {
        IERC1155(precompile).setApprovalForAll(operator, approved);
    }

    function isApprovedForAllProxy(address owner, address operator) public view returns (bool) {
        return IERC1155(precompile).isApprovedForAll(owner, operator);
    }

    function safeTransferFromProxy(
        address from,
        address to,
        uint256 id,
        uint256 amount,
        bytes memory data
    ) external {
        IERC1155(precompile).safeTransferFrom(from, to, id, amount, data);
    }

    function safeBatchTransferFromProxy(
        address from,
        address to,
        uint256[] memory ids,
        uint256[] memory amounts,
        bytes memory data
    ) external {
        IERC1155(precompile).safeBatchTransferFrom(from, to, ids, amounts, data);
    }

    function uriProxy(uint256 id) public view returns (string memory) {
        return IERC1155MetadataURI(precompile).uri(id);
    }
}
