// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.0;

// Calls Seed ERC721 precompile
contract ERC721CallerSimple {
    // transfer nft from from to to
    function transferFromProxy(address precompile, address from, address to, uint256 token_id) external {
        (bool success, bytes memory returnData) = precompile.call(abi.encodeWithSignature("transferFrom(address,address,uint256)", from, to, token_id));
        assembly {
            if eq(success, 0) {
                revert(add(returnData, 0x20), returndatasize())
            }
        }
    }
}