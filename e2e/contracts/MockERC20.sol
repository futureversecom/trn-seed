// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract MockERC20 is ERC20, Ownable {
  constructor() ERC20("MyToken", "MTK") {}

  function mint(address to, uint256 amount) public onlyOwner {
    _mint(to, amount);
  }
}
