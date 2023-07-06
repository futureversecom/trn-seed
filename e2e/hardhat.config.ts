import "@nomicfoundation/hardhat-toolbox";
import * as dotenv from "dotenv";
import { utils } from "ethers";
import { HardhatUserConfig, task } from "hardhat/config";

import { ALITH_PRIVATE_KEY, BOB_PRIVATE_KEY } from "./common";

dotenv.config();

// This is a sample Hardhat task. To learn how to create your own go to
// https://hardhat.org/guides/create-task.html
task("accounts", "Prints the list of accounts", async (taskArgs, hre) => {
  const accounts = await hre.ethers.getSigners();

  for (const account of accounts) {
    console.log(account.address);
  }
});

// You need to export an object to set up your config
// Go to https://hardhat.org/config/ to learn more

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.17",
    // settings: {
    //   optimizer: {
    //     enabled: true,
    //     runs: 200,
    //   },
    // },
  },
  networks: {
    hardhat: {
      chainId: 1337,
      gasPrice: utils.parseUnits("100", "gwei").toNumber(),
    },
    seed: {
      url: "http://localhost:9933",
      accounts: [ALITH_PRIVATE_KEY, BOB_PRIVATE_KEY],
      chainId: 7672,
    },
  },
  etherscan: {
    apiKey: process.env.ETHERSCAN_API_KEY,
  },
  mocha: {
    timeout: 120_000, // global tests timeout
  },
};

export default config;
