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
        address from,
        address to,
        uint256 token_id
    ) external {
        (bool success, bytes memory returnData) = precompile.call(
            abi.encodeWithSignature(
                "transferFrom(address,address,uint256)",
                from,
                to,
                token_id
            )
        );
        require(success, "call failed");
    }
}
