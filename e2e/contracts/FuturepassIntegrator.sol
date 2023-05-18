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

  function depositRevert(uint256 _amount) public payable {
    deposits[msg.sender] += msg.value;
    emit Deposit(msg.sender, msg.value);

    // explicitly final statement as we want to test no balance state changes
    require(msg.value >= _amount, "CurrencyTester: msg.value < _amount");
  }

  // can receive ether directly
  receive() external payable {
    deposits[msg.sender] += msg.value;
    emit Deposit(msg.sender, msg.value);
  }
}

contract CreateTester {
  constructor() {}

  function getValue() public pure returns (uint256) {
    return 420;
  }
}

contract CreateTesterPayable {
  event Deposit(address indexed depositer, uint256 indexed value);

  uint256 public deposit;

  // constructor(uint256 _amount) payable {
  //   require(msg.value >= _amount, "CreateTesterPayable: msg.value < _amount");
  //   deposit = msg.value;
  // }

  constructor(uint256 _amount) payable {
    require(msg.value >= _amount, "CreateTesterPayable: msg.value < _amount");
    deposit = _amount;
    emit Deposit(msg.sender, msg.value);
  }

  function getDeposit() public view returns (uint256) {
    return deposit;
  }

  function getValue() public pure returns (uint256) {
    return 420;
  }
}
