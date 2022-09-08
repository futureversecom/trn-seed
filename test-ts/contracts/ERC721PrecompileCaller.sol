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
        // Calling IERC721(precompile).transferFrom(from, to, token) and IERC721(precompile).approve(to, token)
        // doesn't work using the IERC721 cast. This is because solidity inserts an EXTCODESIZE check when calling a
        // contract with this casting syntax. when it calls a precompile address EXTCODESIZE is 0 so it reverts,
        // doing address.call{} syntax doesn’t insert this check so it works.
        (bool success,) = precompile.call(
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
