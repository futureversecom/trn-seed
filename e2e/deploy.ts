import { BigNumber, Contract, ContractFactory, Wallet, utils } from "ethers";
import { ethers} from "hardhat";

async function main() {

  let owner: SignerWithAddress;
  let user: SignerWithAddress;
  let alpha: CustomERC20;
  let beta: CustomERC20;
  let weth: WETH9;
  let uniswapV2Factory: UniswapV2Factory;
  let uniswapV2Router02: UniswapV2Router02;

  const provider = ethers.provider;

      // deploy all contracts
    const WETH9ERC20Factory = await ethers.getContractFactory("WETH9", owner);
    weth = await WETH9ERC20Factory.deploy();
    await weth.deployed();

    const ERC20Factory = await ethers.getContractFactory("CustomERC20");

    // deploy the AlphaERC20
    alpha = await ERC20Factory.deploy("Alpha", "ALPHA");
    await alpha.deployed();

    // deploy the BetaERC20
    beta = await ERC20Factory.deploy("Beta", "BETA");
    await beta.deployed();

    // Set up owner for re-use
    const [_owner, _user] = await ethers.getSigners();
    owner = _owner;
    user = _user;

    // mint some tokens to the owner
    await alpha.mint(owner.address, utils.parseEther("10000000"));
    await beta.mint(owner.address, utils.parseEther("10000000"));

    // deposit weth to owner
    await weth.connect(owner).deposit({ value: utils.parseEther("100") });

    // deploy the UniswapV2Factory
    const UniswapV2FactoryFactory = await ethers.getContractFactory("UniswapV2Factory");
    uniswapV2Factory = await UniswapV2FactoryFactory.deploy(owner.address);
    await uniswapV2Factory.deployed();

    // deploy the UniswapV2Router02
    const UniswapV2Router02Factory = await ethers.getContractFactory("UniswapV2Router02");
    uniswapV2Router02 = await UniswapV2Router02Factory.deploy(uniswapV2Factory.address, weth.address);
    await uniswapV2Router02.deployed();


    // add liquidity
    await alpha.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));
    await weth.connect(owner).approve(uniswapV2Router02.address, utils.parseEther("10000000"));

    await uniswapV2Router02
      .connect(owner)
      .addLiquidityETH(
        alpha.address,
        utils.parseEther("1000").toString(),
        utils.parseEther("1000").toString(),
        utils.parseEther("250").toString(),
        owner.address,
        ethers.constants.MaxUint256,
        {
          value: utils.parseEther("250"),
        },
      );

    const pairAddress = await uniswapV2Factory.getPair(alpha.address, weth.address);
    const lpToken: CustomERC20 = await ethers.getContractAt("CustomERC20", pairAddress);
    const lpBalance = await lpToken.balanceOf(owner.address);
    console.log("lp balance is: ", lpBalance);

        // 1.2 user approves router to spend tokens
    await alpha.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));
    await weth.connect(user).approve(uniswapV2Router02.address, utils.parseEther("1000"));

    // 1.3 mint some tokens to user
    await alpha.mint(user.address, utils.parseEther("100"));

    const alphaBalanceBefore = await alpha.balanceOf(user.address);
    const wethBalanceBefore = await weth.balanceOf(user.address);
    const wethBalanceContractBefore = await weth.balanceOf(uniswapV2Router02.address);
    const ethBalanceBefore = await provider.getBalance(owner.address);

    console.log("alpha balance before is: ", alphaBalanceBefore);
    console.log("weth balance before is: ", wethBalanceBefore);
    console.log("weth balance of router before is: ", wethBalanceContractBefore);
    console.log("eth balance  before is: ", ethBalanceBefore);

    // 1.5 check amount of tokens retrievable
    const [, ethAmountOut] = await uniswapV2Router02
      .connect(user)
      .getAmountsOut(utils.parseEther("100"), [alpha.address, weth.address]);
    console.log("eth amount is: ", ethAmountOut);

    // 1.6 run callstatic for swap
    await uniswapV2Router02
      .connect(user)
      .swapExactTokensForETH(
        utils.parseEther("100"),
        ethAmountOut,
        [alpha.address, weth.address],
        owner.address,
        ethers.constants.MaxUint256,
      );

    const alphaBalanceAfter = await alpha.balanceOf(user.address);
    const wethBalanceAfter = await weth.balanceOf(user.address);
    const wethBalanceContractAfter = await weth.balanceOf(uniswapV2Router02.address);
    const ethBalanceAfter = await provider.getBalance(owner.address);

    console.log("alpha balance after is: ", alphaBalanceAfter);
    console.log("weth balance after is: ", wethBalanceAfter);
    console.log("weth balance of router after is: ", wethBalanceContractAfter);
    console.log("eth balance  after is: ", ethBalanceAfter);
    console.log("eth diff is: ", ethBalanceAfter - ethBalanceBefore);

    


}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
