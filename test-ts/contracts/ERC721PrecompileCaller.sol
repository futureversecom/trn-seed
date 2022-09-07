// SPDX-License-Identifier: Apache-2.0
pragma solidity 0.8.13;
import "@openzeppelin/contracts/interfaces/IERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/IERC721Metadata.sol";

contract ERC721PrecompileCaller {
    address precompile;

    constructor(address _precompile) {
        precompile = _precompile;
    }

    function balanceOfProxy(address who) public view returns (uint256) {
        return IERC721(precompile).balanceOf(who);
    }

    function ownerOfProxy(uint256 tokenId) public view returns (address) {
        return IERC721(precompile).ownerOf(tokenId);
    }

    function nameProxy() public view returns (string memory) {
        return IERC721Metadata(precompile).name();
    }

    function symbolProxy() public view returns (string memory) {
        return IERC721Metadata(precompile).symbol();
    }

    function tokenURIProxy(uint256 serial_number) public view returns (string memory) {
        return IERC721Metadata(precompile).tokenURI(serial_number);
    }

    function transferFromProxy(
        address precompile_address,
        uint256 serial_number
    ) external {
        IERC721(precompile_address).transferFrom(msg.sender, address(this), serial_number);
    }

    function approveProxy(
        uint256 serial_number
    ) external {
        IERC721(precompile).approve(address(this), serial_number);
    }
}
