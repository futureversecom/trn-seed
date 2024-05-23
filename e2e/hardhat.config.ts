import "@nomicfoundation/hardhat-toolbox";
import * as dotenv from "dotenv";
import { utils } from "ethers";
import { HardhatUserConfig, task } from "hardhat/config";

import { ALITH_PRIVATE_KEY, BOB_PRIVATE_KEY } from "./common";

dotenv.config();

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
      url: "http://127.0.0.1:9944",
      accounts: [ALITH_PRIVATE_KEY, BOB_PRIVATE_KEY],
      chainId: 7672,
    },
  },
  mocha: {
    timeout: 120_000, // global tests timeout
  },
};

export default config;
