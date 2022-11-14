// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.14;
import "@openzeppelin/contracts/interfaces/IERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/IERC721Metadata.sol";

contract ERC721PrecompileCaller {
    address public precompile;

    constructor(address _precompile) {
        precompile = _precompile;
    }

    function balanceOfProxy(address who) public view returns (uint256) {
        return IERC721(precompile).balanceOf(who);
    }

    function ownerOfProxy(uint256 tokenId) public view returns (address) {
        return IERC721(precompile).ownerOf(tokenId);
    }

    function getApprovedProxy(uint256 tokenId) public view returns (address) {
        return IERC721(precompile).getApproved(tokenId);
    }

    function isApprovedForAllProxy(address owner, address operator) public view returns (bool) {
        return IERC721(precompile).isApprovedForAll(owner, operator);
    }

    function nameProxy() public view returns (string memory) {
        return IERC721Metadata(precompile).name();
    }

    function symbolProxy() public view returns (string memory) {
        return IERC721Metadata(precompile).symbol();
    }

    function tokenURIProxy(uint256 tokenId) public view returns (string memory) {
        return IERC721Metadata(precompile).tokenURI(tokenId);
    }

    function transferFromProxy(
        address from,
        address to,
        uint256 tokenId
    ) external {
        IERC721(precompile).transferFrom(from, to, tokenId);
    }

    function safeTransferFromProxy(
        address from,
        address to,
        uint256 tokenId
    ) external {
        IERC721(precompile).safeTransferFrom(from, to, tokenId);
    }

    function setApprovalForAllProxy(
        address operator,
        bool approved
    ) external {
        IERC721(precompile).setApprovalForAll(operator, approved);
    }

    function approveProxy(
        address who,
        uint256 tokenId
    ) external {
        IERC721(precompile).approve(who, tokenId);
    }
}
