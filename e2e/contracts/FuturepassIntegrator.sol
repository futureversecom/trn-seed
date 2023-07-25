// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.17;

contract CurrencyTester {
  mapping(address => uint256) public deposits;

  event Deposit(address indexed depositer, uint256 indexed value);
  event FakeDeposit(address indexed depositer, uint256 indexed value);

  function deposit() public payable {
    deposits[msg.sender] += msg.value;
    emit Deposit(msg.sender, msg.value);
  }

  // can receive ether directly
  receive() external payable {
    deposits[msg.sender] += msg.value;
    emit Deposit(msg.sender, msg.value);
  }
}
